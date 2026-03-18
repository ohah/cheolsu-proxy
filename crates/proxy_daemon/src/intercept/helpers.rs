/// 인터셉트 모듈에서 사용하는 공통 헬퍼 함수 및 정규식 캐시
use lru::LruCache;
use proxyapi_v2::hyper::http::{HeaderMap, HeaderName, HeaderValue};
use proxyapi_v2::Body;
use regex::Regex;
use std::cell::RefCell;
use std::num::NonZeroUsize;

/// 스레드 로컬 정규식 캐시의 최대 크기.
/// tokio 멀티스레드 런타임에서는 워커 스레드마다 독립된 캐시가 생성되므로,
/// 전역적으로 최대 REGEX_CACHE_SIZE × (워커 스레드 수)개의 패턴이 캐싱될 수 있다.
const REGEX_CACHE_SIZE: usize = 256;

// 스레드 로컬 정규식 LRU 캐시. 동일한 패턴의 반복 컴파일을 방지한다.
// 캐시가 가득 차면 가장 오래 사용되지 않은 항목부터 제거(LRU eviction)된다.
thread_local! {
    static REGEX_CACHE: RefCell<LruCache<String, Regex>> =
        RefCell::new(LruCache::new(NonZeroUsize::new(REGEX_CACHE_SIZE).unwrap()));
}

/// 캐싱된 정규식 컴파일. 동일 패턴은 스레드 로컬 LRU 캐시에서 반환한다.
/// 캐시가 가득 차면 가장 오래 사용되지 않은 패턴이 자동으로 제거된다.
pub(crate) fn cached_regex(pattern: &str) -> Result<Regex, regex::Error> {
    REGEX_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(re) = cache.get(pattern) {
            return Ok(re.clone());
        }
        let re = Regex::new(pattern)?;
        let result = re.clone();
        cache.put(pattern.to_string(), re);
        Ok(result)
    })
}

/// 헤더 제거 + 추가를 수행하는 공통 헬퍼
pub(crate) fn apply_header_modifications(
    headers: &mut HeaderMap,
    remove_headers: &[String],
    add_headers: &std::collections::HashMap<String, String>,
) {
    for name in remove_headers {
        if let Ok(header_name) = name.parse::<HeaderName>() {
            headers.remove(header_name);
        }
    }
    for (name, value) in add_headers {
        if let (Ok(header_name), Ok(header_value)) =
            (name.parse::<HeaderName>(), value.parse::<HeaderValue>())
        {
            headers.insert(header_name, header_value);
        }
    }
}

/// 헤더 값에 대해 정규식 치환을 수행하는 공통 함수
pub(crate) fn rewrite_headers(
    headers: &HeaderMap,
    re: &Regex,
    replace_with: &str,
) -> Vec<(HeaderName, HeaderValue)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            let val_str = value.to_str().ok()?;
            if re.is_match(val_str) {
                let new_val = re.replace_all(val_str, replace_with);
                HeaderValue::from_str(&new_val)
                    .ok()
                    .map(|hv| (name.clone(), hv))
            } else {
                None
            }
        })
        .collect()
}

/// 바디를 읽어서 정규식 치환 후 새 바디 바이트를 반환하는 공통 함수
pub(crate) async fn rewrite_body_bytes(
    body: &mut Body,
    re: &Regex,
    replace_with: &str,
) -> Option<bytes::Bytes> {
    use http_body_util::BodyExt;
    let body_bytes = body.collect().await;
    if let Ok(collected) = body_bytes {
        let bytes = collected.to_bytes();
        if let Ok(body_str) = std::str::from_utf8(&bytes) {
            let new_body = re.replace_all(body_str, replace_with);
            return Some(bytes::Bytes::from(new_body.into_owned()));
        }
    }
    None
}

/// Rewrite 액션의 정규식 컴파일 + 헤더/바디 치환을 수행하는 공통 매크로.
/// Request와 Response 모두에서 사용 가능하도록 매크로로 구현한다.
/// `$msg`에 borrow 분리를 위해 `$msg.headers_mut()`, `$msg.body_mut()` 호출을 분리한다.
macro_rules! apply_rewrite_action {
    ($msg:expr, $match_pattern:expr, $replace_with:expr, $is_header:expr, $log_label:expr, $method:expr, $url:expr, $rule_name:expr) => {
        match $crate::intercept::helpers::cached_regex($match_pattern) {
            Ok(re) => {
                if $is_header {
                    tracing::info!(
                        "[Intercept] Rewrite {} 헤더: {} {} (규칙: {})",
                        $log_label,
                        $method,
                        $url,
                        $rule_name
                    );
                    let replacements = $crate::intercept::helpers::rewrite_headers(
                        $msg.headers(),
                        &re,
                        $replace_with,
                    );
                    for (name, value) in replacements {
                        $msg.headers_mut().insert(name, value);
                    }
                } else {
                    tracing::info!(
                        "[Intercept] Rewrite {} 바디: {} {} (규칙: {})",
                        $log_label,
                        $method,
                        $url,
                        $rule_name
                    );
                    if let Some(new_bytes) = $crate::intercept::helpers::rewrite_body_bytes(
                        $msg.body_mut(),
                        &re,
                        $replace_with,
                    )
                    .await
                    {
                        $crate::header_utils::clear_content_encoding_headers($msg.headers_mut());
                        use http_body_util::Full;
                        *$msg.body_mut() = proxyapi_v2::Body::from(Full::new(new_bytes));
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "[Intercept] Rewrite 정규식 컴파일 실패: {} - {}",
                    $match_pattern,
                    e
                );
            }
        }
    };
}

pub(crate) use apply_rewrite_action;
