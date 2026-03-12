use crate::protocol::{InterceptAction, InterceptRule, RewriteTarget, ServerReplayEntry};
use bytes::Bytes;
use proxyapi_v2::hyper::http::HeaderMap;
use proxyapi_v2::hyper::Request;
use proxyapi_v2::{
    hyper::http::{HeaderName, HeaderValue, StatusCode},
    hyper::Response,
    Body, RequestOrResponse,
};
use regex::Regex;
use std::cell::RefCell;
use std::collections::HashMap;
use tracing::{error, info};

use super::handler::LoggingHandler;

// 스레드 로컬 정규식 캐시. 동일한 패턴의 반복 컴파일을 방지한다.
thread_local! {
    static REGEX_CACHE: RefCell<HashMap<String, Regex>> = RefCell::new(HashMap::new());
}

/// 캐싱된 정규식 컴파일. 동일 패턴은 스레드 로컬 캐시에서 반환한다.
fn cached_regex(pattern: &str) -> Result<Regex, regex::Error> {
    REGEX_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(re) = cache.get(pattern) {
            return Ok(re.clone());
        }
        let re = Regex::new(pattern)?;
        // 캐시가 과도하게 커지는 것을 방지 (최대 256개)
        if cache.len() >= 256 {
            cache.clear();
        }
        cache.insert(pattern.to_string(), re.clone());
        Ok(re)
    })
}

/// 헤더 제거 + 추가를 수행하는 공통 헬퍼
fn apply_header_modifications(
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
fn rewrite_headers(
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
async fn rewrite_body_bytes(
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
        match cached_regex($match_pattern) {
            Ok(re) => {
                if $is_header {
                    info!(
                        "[Intercept] Rewrite {} 헤더: {} {} (규칙: {})",
                        $log_label, $method, $url, $rule_name
                    );
                    let replacements = rewrite_headers($msg.headers(), &re, $replace_with);
                    for (name, value) in replacements {
                        $msg.headers_mut().insert(name, value);
                    }
                } else {
                    info!(
                        "[Intercept] Rewrite {} 바디: {} {} (규칙: {})",
                        $log_label, $method, $url, $rule_name
                    );
                    if let Some(new_bytes) =
                        rewrite_body_bytes($msg.body_mut(), &re, $replace_with).await
                    {
                        crate::header_utils::clear_content_encoding_headers($msg.headers_mut());
                        use http_body_util::Full;
                        *$msg.body_mut() = Body::from(Full::new(new_bytes));
                    }
                }
            }
            Err(e) => {
                error!(
                    "[Intercept] Rewrite 정규식 컴파일 실패: {} - {}",
                    $match_pattern, e
                );
            }
        }
    };
}

impl LoggingHandler {
    /// 서버 리플레이 매칭: method + URL이 일치하는 엔트리 검색
    pub(crate) async fn find_server_replay_match(
        &self,
        url: &str,
        method: &str,
    ) -> Option<ServerReplayEntry> {
        let entries = self.intercept.server_replay_entries.read().await;
        entries
            .iter()
            .find(|entry| entry.method.eq_ignore_ascii_case(method) && entry.url == url)
            .cloned()
    }

    /// URL과 메서드가 인터셉트 규칙에 매칭되는지 확인
    pub(crate) fn rule_matches(rule: &InterceptRule, url: &str, method: &str) -> bool {
        if !rule.enabled {
            return false;
        }
        if let Some(rule_method) = &rule.method {
            if rule_method.to_uppercase() != method.to_uppercase() {
                return false;
            }
        }
        crate::pattern_utils::wildcard_matches(&rule.pattern, url)
    }

    /// 파일 확장자로 Content-Type 추론
    pub(crate) fn guess_content_type(file_path: &str) -> String {
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "json" => "application/json",
            "html" | "htm" => "text/html; charset=utf-8",
            "xml" => "application/xml",
            "js" | "mjs" => "application/javascript",
            "css" => "text/css",
            "txt" => "text/plain; charset=utf-8",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "woff" | "woff2" => "font/woff2",
            "pdf" => "application/pdf",
            _ => "application/octet-stream",
        }
        .to_string()
    }

    /// 매칭되는 인터셉트 규칙 검색
    pub(crate) async fn find_matching_intercept_rules(
        &self,
        url: &str,
        method: &str,
    ) -> Vec<InterceptRule> {
        let rules_guard = self.intercept.intercept_rules.read().await;
        rules_guard
            .iter()
            .filter(|rule| Self::rule_matches(rule, url, method))
            .cloned()
            .collect()
    }

    /// 인터셉트 규칙에 따라 요청을 차단하거나 수정
    pub(crate) async fn apply_request_intercept(
        &self,
        req: Request<Body>,
        url: &str,
        method: &str,
    ) -> RequestOrResponse {
        let rules = self.find_matching_intercept_rules(url, method).await;

        let mut current_req = req;

        for rule in &rules {
            match &rule.action {
                InterceptAction::Block { status_code, body } => {
                    info!(
                        "[Intercept] 요청 차단: {} {} -> {} (규칙: {})",
                        method, url, status_code, rule.name
                    );
                    let mut response = Response::builder()
                        .status(StatusCode::from_u16(*status_code).unwrap_or(StatusCode::FORBIDDEN))
                        .body(Body::from(body.clone()))
                        .unwrap_or_else(|_| Response::new(Body::empty()));
                    // Content-Type 설정
                    if !body.is_empty() {
                        if body.starts_with('{') || body.starts_with('[') {
                            response.headers_mut().insert(
                                "content-type",
                                HeaderValue::from_static("application/json"),
                            );
                        } else {
                            response.headers_mut().insert(
                                "content-type",
                                HeaderValue::from_static("text/plain; charset=utf-8"),
                            );
                        }
                    }
                    return response.into();
                }
                InterceptAction::ModifyRequest {
                    add_headers,
                    remove_headers,
                    set_body,
                } => {
                    info!(
                        "[Intercept] 요청 수정: {} {} (규칙: {})",
                        method, url, rule.name
                    );
                    apply_header_modifications(
                        current_req.headers_mut(),
                        remove_headers,
                        add_headers,
                    );
                    // 바디 변경
                    if let Some(new_body) = set_body {
                        use http_body_util::Full;
                        *current_req.body_mut() =
                            Body::from(Full::new(bytes::Bytes::from(new_body.clone())));
                    }
                }
                InterceptAction::ModifyResponse { .. } => {
                    // 응답 수정 규칙은 handle_response에서 처리
                }
                InterceptAction::Rewrite {
                    target,
                    match_pattern,
                    replace_with,
                } if *target == RewriteTarget::RequestHeader
                    || *target == RewriteTarget::RequestBody =>
                {
                    apply_rewrite_action!(
                        current_req,
                        match_pattern,
                        replace_with,
                        *target == RewriteTarget::RequestHeader,
                        "요청",
                        method,
                        url,
                        &rule.name
                    );
                }
                InterceptAction::Rewrite { .. } => {
                    // response 대상 rewrite는 apply_response_intercept에서 처리
                }
                InterceptAction::MapLocal {
                    file_path,
                    status_code,
                    headers,
                } => {
                    info!(
                        "[Intercept] Map Local: {} {} -> {} (규칙: {})",
                        method, url, file_path, rule.name
                    );
                    match std::fs::read(file_path) {
                        Ok(file_bytes) => {
                            let file_bytes = Bytes::from(file_bytes);
                            let mut response = Response::builder()
                                .status(
                                    StatusCode::from_u16(*status_code).unwrap_or(StatusCode::OK),
                                )
                                .header("x-cheolsu-map-local", file_path.as_str());

                            // Content-Length 설정
                            response =
                                response.header("content-length", file_bytes.len().to_string());

                            // Content-Type 추론
                            let content_type = headers
                                .get("content-type")
                                .cloned()
                                .unwrap_or_else(|| Self::guess_content_type(file_path));
                            response = response.header("content-type", content_type);

                            // 추가 헤더 설정
                            for (name, value) in headers {
                                if name.to_lowercase() != "content-type" {
                                    response = response.header(name.as_str(), value.as_str());
                                }
                            }

                            let resp = response
                                .body(Body::from(http_body_util::Full::new(file_bytes)))
                                .unwrap_or_else(|_| Response::new(Body::empty()));
                            return resp.into();
                        }
                        Err(e) => {
                            error!(
                                "[Intercept] Map Local 파일 읽기 실패: {} - {}",
                                file_path, e
                            );
                            let err_resp = Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .header("x-cheolsu-map-local-error", e.to_string())
                                .body(Body::from(format!(
                                    "Map Local Error: file not found - {}",
                                    file_path
                                )))
                                .unwrap_or_else(|_| Response::new(Body::empty()));
                            return err_resp.into();
                        }
                    }
                }
                InterceptAction::MapRemote {
                    target_url,
                    preserve_path,
                } => {
                    info!(
                        "[Intercept] Map Remote: {} {} -> {} (규칙: {}, preserve_path={})",
                        method, url, target_url, rule.name, preserve_path
                    );
                    let new_url = if *preserve_path {
                        // 원본 URL에서 path + query 추출하여 target에 붙임
                        if let Ok(original) = url.parse::<proxyapi_v2::hyper::Uri>() {
                            let path_and_query = original
                                .path_and_query()
                                .map(|pq| pq.as_str())
                                .unwrap_or("/");
                            let base = target_url.trim_end_matches('/');
                            format!("{}{}", base, path_and_query)
                        } else {
                            target_url.clone()
                        }
                    } else {
                        target_url.clone()
                    };

                    if let Ok(new_uri) = new_url.parse::<proxyapi_v2::hyper::Uri>() {
                        *current_req.uri_mut() = new_uri;
                        // Host 헤더도 새 URL에 맞게 변경
                        if let Some(host) = new_url
                            .parse::<proxyapi_v2::hyper::Uri>()
                            .ok()
                            .and_then(|u| u.host().map(|h| h.to_string()))
                        {
                            if let Ok(host_value) = host.parse::<HeaderValue>() {
                                current_req
                                    .headers_mut()
                                    .insert(proxyapi_v2::hyper::header::HOST, host_value);
                            }
                        }
                        current_req.headers_mut().insert(
                            "x-cheolsu-map-remote-original",
                            url.parse()
                                .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
                        );
                    } else {
                        error!("[Intercept] Map Remote URL 파싱 실패: {}", new_url);
                    }
                }
            }
        }

        current_req.into()
    }

    /// 인터셉트 규칙에 따라 응답을 수정
    pub(crate) async fn apply_response_intercept(
        &self,
        mut res: Response<Body>,
        url: &str,
        method: &str,
    ) -> Response<Body> {
        let rules = self.find_matching_intercept_rules(url, method).await;

        for rule in &rules {
            if let InterceptAction::ModifyResponse {
                set_status,
                add_headers,
                remove_headers,
                set_body,
            } = &rule.action
            {
                info!(
                    "[Intercept] 응답 수정: {} {} (규칙: {})",
                    method, url, rule.name
                );

                // 상태 코드 변경
                if let Some(status) = set_status {
                    if let Ok(status_code) = StatusCode::from_u16(*status) {
                        *res.status_mut() = status_code;
                    }
                }

                apply_header_modifications(res.headers_mut(), remove_headers, add_headers);

                // 바디 변경
                if let Some(new_body) = set_body {
                    use http_body_util::Full;
                    // Content-Length 업데이트
                    let body_bytes = bytes::Bytes::from(new_body.clone());
                    crate::header_utils::clear_content_encoding_headers(res.headers_mut());
                    *res.body_mut() = Body::from(Full::new(body_bytes));
                }
            }

            if let InterceptAction::Rewrite {
                target,
                match_pattern,
                replace_with,
            } = &rule.action
            {
                if *target == RewriteTarget::ResponseHeader
                    || *target == RewriteTarget::ResponseBody
                {
                    apply_rewrite_action!(
                        res,
                        match_pattern,
                        replace_with,
                        *target == RewriteTarget::ResponseHeader,
                        "응답",
                        method,
                        url,
                        &rule.name
                    );
                }
            }
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{InterceptAction, InterceptRule};

    // --- rule_matches 테스트 ---

    fn make_rule(pattern: &str, method: Option<&str>, enabled: bool) -> InterceptRule {
        InterceptRule {
            id: "test".to_string(),
            name: "Test".to_string(),
            enabled,
            pattern: pattern.to_string(),
            method: method.map(|m| m.to_string()),
            action: InterceptAction::Block {
                status_code: 403,
                body: String::new(),
            },
        }
    }

    #[test]
    fn test_rule_matches_basic() {
        let rule = make_rule("*example.com*", None, true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://example.com/api",
            "GET"
        ));
    }

    #[test]
    fn test_rule_matches_with_method() {
        let rule = make_rule("*api.com*", Some("POST"), true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://api.com/v1",
            "POST"
        ));
        assert!(!LoggingHandler::rule_matches(
            &rule,
            "https://api.com/v1",
            "GET"
        ));
    }

    #[test]
    fn test_rule_matches_method_case_insensitive() {
        let rule = make_rule("*api.com*", Some("post"), true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://api.com/v1",
            "POST"
        ));
    }

    #[test]
    fn test_rule_matches_disabled() {
        let rule = make_rule("*example.com*", None, false);
        assert!(!LoggingHandler::rule_matches(
            &rule,
            "https://example.com/api",
            "GET"
        ));
    }

    #[test]
    fn test_rule_matches_no_method_filter_matches_all() {
        let rule = make_rule("*example.com*", None, true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://example.com",
            "GET"
        ));
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://example.com",
            "POST"
        ));
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://example.com",
            "DELETE"
        ));
    }

    #[test]
    fn test_rule_matches_complex_pattern() {
        let rule = make_rule("*.example.com/api/*/users", Some("GET"), true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://sub.example.com/api/v1/users",
            "GET"
        ));
        assert!(!LoggingHandler::rule_matches(
            &rule,
            "https://sub.example.com/api/v1/posts",
            "GET"
        ));
        assert!(!LoggingHandler::rule_matches(
            &rule,
            "https://sub.example.com/api/v1/users",
            "POST"
        ));
    }

    // --- rewrite_headers 테스트 ---

    #[test]
    fn test_rewrite_headers_simple_replacement() {
        let mut headers = HeaderMap::new();
        headers.insert("x-custom", HeaderValue::from_static("old-value"));
        headers.insert("x-other", HeaderValue::from_static("unchanged"));

        let re = Regex::new("old").unwrap();
        let replacements = rewrite_headers(&headers, &re, "new");

        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].0.as_str(), "x-custom");
        assert_eq!(replacements[0].1.to_str().unwrap(), "new-value");
    }

    #[test]
    fn test_rewrite_headers_multiple_matches() {
        let mut headers = HeaderMap::new();
        headers.insert("x-first", HeaderValue::from_static("foo-bar"));
        headers.insert("x-second", HeaderValue::from_static("foo-baz"));
        headers.insert("x-third", HeaderValue::from_static("no-match"));

        let re = Regex::new("foo").unwrap();
        let replacements = rewrite_headers(&headers, &re, "qux");

        assert_eq!(replacements.len(), 2);
        let values: Vec<String> = replacements
            .iter()
            .map(|(_, v)| v.to_str().unwrap().to_string())
            .collect();
        assert!(values.contains(&"qux-bar".to_string()));
        assert!(values.contains(&"qux-baz".to_string()));
    }

    #[test]
    fn test_rewrite_headers_regex_capture_groups() {
        let mut headers = HeaderMap::new();
        headers.insert("x-version", HeaderValue::from_static("v1.2.3"));

        let re = Regex::new(r"v(\d+)\.(\d+)\.(\d+)").unwrap();
        let replacements = rewrite_headers(&headers, &re, "version-$1-$2-$3");

        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].1.to_str().unwrap(), "version-1-2-3");
    }

    #[test]
    fn test_rewrite_headers_no_match() {
        let mut headers = HeaderMap::new();
        headers.insert("x-custom", HeaderValue::from_static("hello"));

        let re = Regex::new("xyz").unwrap();
        let replacements = rewrite_headers(&headers, &re, "replaced");

        assert!(replacements.is_empty());
    }

    // --- rewrite_body_bytes 테스트 ---

    #[tokio::test]
    async fn test_rewrite_body_simple_replacement() {
        let body_content = r#"{"premium": false}"#;
        let mut body = Body::from(body_content);

        let re = Regex::new(r#""premium": false"#).unwrap();
        let result = rewrite_body_bytes(&mut body, &re, r#""premium": true"#).await;

        assert!(result.is_some());
        let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
        assert_eq!(new_body, r#"{"premium": true}"#);
    }

    #[tokio::test]
    async fn test_rewrite_body_regex_capture_groups() {
        let body_content = "123-456";
        let mut body = Body::from(body_content);

        let re = Regex::new(r"(\d+)-(\d+)").unwrap();
        let result = rewrite_body_bytes(&mut body, &re, "$2-$1").await;

        assert!(result.is_some());
        let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
        assert_eq!(new_body, "456-123");
    }

    #[tokio::test]
    async fn test_rewrite_body_multiple_occurrences() {
        let body_content = "aaa-bbb ccc-ddd";
        let mut body = Body::from(body_content);

        let re = Regex::new(r"(\w+)-(\w+)").unwrap();
        let result = rewrite_body_bytes(&mut body, &re, "$2-$1").await;

        assert!(result.is_some());
        let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
        assert_eq!(new_body, "bbb-aaa ddd-ccc");
    }

    #[tokio::test]
    async fn test_rewrite_body_no_match() {
        let body_content = "hello world";
        let mut body = Body::from(body_content);

        let re = Regex::new("xyz").unwrap();
        let result = rewrite_body_bytes(&mut body, &re, "replaced").await;

        // 매칭이 없어도 원본 바디가 반환됨 (replace_all은 매칭 없으면 원본 반환)
        assert!(result.is_some());
        let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
        assert_eq!(new_body, "hello world");
    }

    // --- 잘못된 정규식 에러 처리 테스트 ---

    #[test]
    fn test_invalid_regex_does_not_crash() {
        let result = Regex::new("[invalid");
        assert!(result.is_err(), "잘못된 정규식은 Err를 반환해야 합니다");
    }

    #[test]
    fn test_invalid_regex_error_message() {
        let result = Regex::new("[invalid");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(!err_msg.is_empty(), "에러 메시지가 비어있지 않아야 합니다");
    }

    // --- Rewrite InterceptAction serde 테스트 ---

    #[test]
    fn test_rewrite_action_serde_roundtrip() {
        let action = InterceptAction::Rewrite {
            target: RewriteTarget::ResponseBody,
            match_pattern: r#""premium": false"#.to_string(),
            replace_with: r#""premium": true"#.to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"rewrite\""));
        assert!(json.contains("response_body"));

        let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            InterceptAction::Rewrite {
                target,
                match_pattern,
                replace_with,
            } => {
                assert_eq!(target, RewriteTarget::ResponseBody);
                assert_eq!(match_pattern, r#""premium": false"#);
                assert_eq!(replace_with, r#""premium": true"#);
            }
            _ => panic!("Expected Rewrite variant"),
        }
    }

    #[test]
    fn test_rewrite_action_all_targets() {
        let targets = vec![
            (RewriteTarget::RequestHeader, "request_header"),
            (RewriteTarget::ResponseHeader, "response_header"),
            (RewriteTarget::RequestBody, "request_body"),
            (RewriteTarget::ResponseBody, "response_body"),
        ];
        for (target, expected_str) in targets {
            let action = InterceptAction::Rewrite {
                target: target.clone(),
                match_pattern: "test".to_string(),
                replace_with: "replaced".to_string(),
            };
            let json = serde_json::to_string(&action).unwrap();
            assert!(
                json.contains(expected_str),
                "JSON should contain '{}' for target {:?}, got: {}",
                expected_str,
                target,
                json
            );
        }
    }

    #[test]
    fn test_rewrite_rule_display() {
        let rule = InterceptRule {
            id: "rw_1".to_string(),
            name: "Rewrite Test".to_string(),
            enabled: true,
            pattern: "*example.com*".to_string(),
            method: None,
            action: InterceptAction::Rewrite {
                target: RewriteTarget::ResponseBody,
                match_pattern: "old".to_string(),
                replace_with: "new".to_string(),
            },
        };
        let display = format!("{}", rule);
        assert!(display.contains("Rewrite"));
        assert!(display.contains("old"));
    }

    // --- apply_header_modifications ---

    #[test]
    fn test_apply_header_modifications_add_headers() {
        let mut headers = HeaderMap::new();
        let remove = vec![];
        let mut add = std::collections::HashMap::new();
        add.insert("x-added".to_string(), "value1".to_string());
        add.insert("x-another".to_string(), "value2".to_string());

        apply_header_modifications(&mut headers, &remove, &add);

        assert_eq!(headers.get("x-added").unwrap().to_str().unwrap(), "value1");
        assert_eq!(
            headers.get("x-another").unwrap().to_str().unwrap(),
            "value2"
        );
    }

    #[test]
    fn test_apply_header_modifications_remove_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-remove-me", HeaderValue::from_static("bye"));
        headers.insert("x-keep", HeaderValue::from_static("stay"));

        let remove = vec!["x-remove-me".to_string()];
        let add = std::collections::HashMap::new();

        apply_header_modifications(&mut headers, &remove, &add);

        assert!(headers.get("x-remove-me").is_none());
        assert_eq!(headers.get("x-keep").unwrap().to_str().unwrap(), "stay");
    }

    #[test]
    fn test_apply_header_modifications_add_and_remove() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer old"));

        let remove = vec!["authorization".to_string()];
        let mut add = std::collections::HashMap::new();
        add.insert("authorization".to_string(), "Bearer new".to_string());

        apply_header_modifications(&mut headers, &remove, &add);

        assert_eq!(
            headers.get("authorization").unwrap().to_str().unwrap(),
            "Bearer new"
        );
    }

    #[test]
    fn test_apply_header_modifications_invalid_header_name_ignored() {
        let mut headers = HeaderMap::new();
        let remove = vec!["invalid header name with spaces".to_string()];
        let add = std::collections::HashMap::new();

        apply_header_modifications(&mut headers, &remove, &add);
        assert!(headers.is_empty());
    }

    #[test]
    fn test_apply_header_modifications_empty_operations() {
        let mut headers = HeaderMap::new();
        headers.insert("x-existing", HeaderValue::from_static("value"));

        apply_header_modifications(&mut headers, &vec![], &std::collections::HashMap::new());

        assert_eq!(
            headers.get("x-existing").unwrap().to_str().unwrap(),
            "value"
        );
    }

    // --- guess_content_type ---

    #[test]
    fn test_guess_content_type_json() {
        assert_eq!(
            LoggingHandler::guess_content_type("data.json"),
            "application/json"
        );
    }

    #[test]
    fn test_guess_content_type_html() {
        assert_eq!(
            LoggingHandler::guess_content_type("index.html"),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            LoggingHandler::guess_content_type("page.htm"),
            "text/html; charset=utf-8"
        );
    }

    #[test]
    fn test_guess_content_type_javascript() {
        assert_eq!(
            LoggingHandler::guess_content_type("app.js"),
            "application/javascript"
        );
        assert_eq!(
            LoggingHandler::guess_content_type("module.mjs"),
            "application/javascript"
        );
    }

    #[test]
    fn test_guess_content_type_css() {
        assert_eq!(LoggingHandler::guess_content_type("style.css"), "text/css");
    }

    #[test]
    fn test_guess_content_type_images() {
        assert_eq!(LoggingHandler::guess_content_type("photo.png"), "image/png");
        assert_eq!(
            LoggingHandler::guess_content_type("photo.jpg"),
            "image/jpeg"
        );
        assert_eq!(
            LoggingHandler::guess_content_type("photo.jpeg"),
            "image/jpeg"
        );
        assert_eq!(LoggingHandler::guess_content_type("anim.gif"), "image/gif");
        assert_eq!(
            LoggingHandler::guess_content_type("icon.svg"),
            "image/svg+xml"
        );
        assert_eq!(
            LoggingHandler::guess_content_type("photo.webp"),
            "image/webp"
        );
    }

    #[test]
    fn test_guess_content_type_fonts() {
        assert_eq!(
            LoggingHandler::guess_content_type("font.woff"),
            "font/woff2"
        );
        assert_eq!(
            LoggingHandler::guess_content_type("font.woff2"),
            "font/woff2"
        );
    }

    #[test]
    fn test_guess_content_type_other() {
        assert_eq!(
            LoggingHandler::guess_content_type("file.bin"),
            "application/octet-stream"
        );
        assert_eq!(
            LoggingHandler::guess_content_type("noext"),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_guess_content_type_xml() {
        assert_eq!(
            LoggingHandler::guess_content_type("data.xml"),
            "application/xml"
        );
    }

    #[test]
    fn test_guess_content_type_txt() {
        assert_eq!(
            LoggingHandler::guess_content_type("readme.txt"),
            "text/plain; charset=utf-8"
        );
    }

    #[test]
    fn test_guess_content_type_pdf() {
        assert_eq!(
            LoggingHandler::guess_content_type("doc.pdf"),
            "application/pdf"
        );
    }

    #[test]
    fn test_guess_content_type_case_insensitive() {
        assert_eq!(
            LoggingHandler::guess_content_type("DATA.JSON"),
            "application/json"
        );
        assert_eq!(LoggingHandler::guess_content_type("STYLE.CSS"), "text/css");
    }

    #[test]
    fn test_guess_content_type_with_path() {
        assert_eq!(
            LoggingHandler::guess_content_type("/var/data/response.json"),
            "application/json"
        );
    }

    // --- rewrite_headers edge cases ---

    #[test]
    fn test_rewrite_headers_empty_headermap() {
        let headers = HeaderMap::new();
        let re = Regex::new("anything").unwrap();
        let replacements = rewrite_headers(&headers, &re, "replaced");
        assert!(replacements.is_empty());
    }

    #[test]
    fn test_rewrite_headers_replace_all_occurrences_in_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-data", HeaderValue::from_static("aaa-aaa-aaa"));

        let re = Regex::new("aaa").unwrap();
        let replacements = rewrite_headers(&headers, &re, "bbb");

        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].1.to_str().unwrap(), "bbb-bbb-bbb");
    }

    // --- rewrite_body_bytes edge cases ---

    #[tokio::test]
    async fn test_rewrite_body_empty_body() {
        let mut body = Body::from("");

        let re = Regex::new("anything").unwrap();
        let result = rewrite_body_bytes(&mut body, &re, "replaced").await;

        assert!(result.is_some());
        let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
        assert_eq!(new_body, "");
    }

    #[tokio::test]
    async fn test_rewrite_body_large_replace_all() {
        let body_content = "apple banana apple cherry apple";
        let mut body = Body::from(body_content);

        let re = Regex::new("apple").unwrap();
        let result = rewrite_body_bytes(&mut body, &re, "orange").await;

        assert!(result.is_some());
        let new_body = String::from_utf8(result.unwrap().to_vec()).unwrap();
        assert_eq!(new_body, "orange banana orange cherry orange");
    }

    // --- Block action serde ---

    #[test]
    fn test_block_action_serde_roundtrip() {
        let action = InterceptAction::Block {
            status_code: 403,
            body: "Forbidden".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"block\""));

        let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            InterceptAction::Block { status_code, body } => {
                assert_eq!(status_code, 403);
                assert_eq!(body, "Forbidden");
            }
            _ => panic!("Expected Block variant"),
        }
    }

    // --- ModifyRequest action serde ---

    #[test]
    fn test_modify_request_action_serde_roundtrip() {
        let mut add_headers = std::collections::HashMap::new();
        add_headers.insert("x-custom".to_string(), "value".to_string());
        let action = InterceptAction::ModifyRequest {
            add_headers,
            remove_headers: vec!["x-remove".to_string()],
            set_body: Some("new body".to_string()),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"modify_request\""));

        let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            InterceptAction::ModifyRequest {
                add_headers,
                remove_headers,
                set_body,
            } => {
                assert_eq!(add_headers.get("x-custom").unwrap(), "value");
                assert_eq!(remove_headers, vec!["x-remove"]);
                assert_eq!(set_body.unwrap(), "new body");
            }
            _ => panic!("Expected ModifyRequest variant"),
        }
    }

    // --- ModifyResponse action serde ---

    #[test]
    fn test_modify_response_action_serde_roundtrip() {
        let action = InterceptAction::ModifyResponse {
            set_status: Some(201),
            add_headers: std::collections::HashMap::new(),
            remove_headers: vec![],
            set_body: None,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"modify_response\""));

        let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            InterceptAction::ModifyResponse {
                set_status,
                set_body,
                ..
            } => {
                assert_eq!(set_status.unwrap(), 201);
                assert!(set_body.is_none());
            }
            _ => panic!("Expected ModifyResponse variant"),
        }
    }

    // --- MapLocal action serde ---

    #[test]
    fn test_map_local_action_serde_roundtrip() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("x-custom".to_string(), "val".to_string());
        let action = InterceptAction::MapLocal {
            file_path: "/tmp/mock.json".to_string(),
            status_code: 200,
            headers,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"map_local\""));

        let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            InterceptAction::MapLocal {
                file_path,
                status_code,
                headers,
            } => {
                assert_eq!(file_path, "/tmp/mock.json");
                assert_eq!(status_code, 200);
                assert_eq!(headers.get("x-custom").unwrap(), "val");
            }
            _ => panic!("Expected MapLocal variant"),
        }
    }

    // --- MapRemote action serde ---

    #[test]
    fn test_map_remote_action_serde_roundtrip() {
        let action = InterceptAction::MapRemote {
            target_url: "http://localhost:3000".to_string(),
            preserve_path: true,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"map_remote\""));
        assert!(json.contains("\"preserve_path\":true"));

        let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            InterceptAction::MapRemote {
                target_url,
                preserve_path,
            } => {
                assert_eq!(target_url, "http://localhost:3000");
                assert!(preserve_path);
            }
            _ => panic!("Expected MapRemote variant"),
        }
    }

    #[test]
    fn test_map_remote_action_preserve_path_false() {
        let action = InterceptAction::MapRemote {
            target_url: "http://mock.local/fixed".to_string(),
            preserve_path: false,
        };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: InterceptAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            InterceptAction::MapRemote { preserve_path, .. } => {
                assert!(!preserve_path);
            }
            _ => panic!("Expected MapRemote variant"),
        }
    }

    // --- ServerReplayEntry serde ---

    #[test]
    fn test_server_replay_entry_serde_roundtrip() {
        use crate::protocol::ServerReplayEntry;

        let mut headers = std::collections::HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let entry = ServerReplayEntry {
            id: "sr_1".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com/users".to_string(),
            status: 200,
            headers,
            body: Some(r#"[{"id": 1}]"#.to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ServerReplayEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "sr_1");
        assert_eq!(deserialized.method, "GET");
        assert_eq!(deserialized.url, "https://api.example.com/users");
        assert_eq!(deserialized.status, 200);
        assert_eq!(
            deserialized.headers.get("content-type").unwrap(),
            "application/json"
        );
        assert_eq!(deserialized.body.unwrap(), r#"[{"id": 1}]"#);
    }

    #[test]
    fn test_server_replay_entry_no_body() {
        use crate::protocol::ServerReplayEntry;

        let entry = ServerReplayEntry {
            id: "sr_2".to_string(),
            method: "DELETE".to_string(),
            url: "https://api.example.com/users/1".to_string(),
            status: 204,
            headers: std::collections::HashMap::new(),
            body: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ServerReplayEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.status, 204);
        assert!(deserialized.body.is_none());
    }

    // --- InterceptRule Display for various actions ---

    #[test]
    fn test_block_rule_display() {
        let rule = InterceptRule {
            id: "b_1".to_string(),
            name: "Block Ads".to_string(),
            enabled: true,
            pattern: "*ads*".to_string(),
            method: None,
            action: InterceptAction::Block {
                status_code: 403,
                body: String::new(),
            },
        };
        let display = format!("{}", rule);
        assert!(display.contains("Block"));
        assert!(display.contains("403"));
    }

    #[test]
    fn test_modify_request_rule_display() {
        let rule = InterceptRule {
            id: "mr_1".to_string(),
            name: "Add Auth".to_string(),
            enabled: true,
            pattern: "*api*".to_string(),
            method: Some("GET".to_string()),
            action: InterceptAction::ModifyRequest {
                add_headers: std::collections::HashMap::new(),
                remove_headers: vec![],
                set_body: None,
            },
        };
        let display = format!("{}", rule);
        assert!(display.contains("ModifyRequest"));
    }

    #[test]
    fn test_modify_response_rule_display() {
        let rule = InterceptRule {
            id: "mres_1".to_string(),
            name: "Change Status".to_string(),
            enabled: false,
            pattern: "*".to_string(),
            method: None,
            action: InterceptAction::ModifyResponse {
                set_status: Some(500),
                add_headers: std::collections::HashMap::new(),
                remove_headers: vec![],
                set_body: None,
            },
        };
        let display = format!("{}", rule);
        assert!(display.contains("ModifyResponse"));
        assert!(display.contains("500"));
    }

    #[test]
    fn test_map_local_rule_display() {
        let rule = InterceptRule {
            id: "ml_1".to_string(),
            name: "Local Mock".to_string(),
            enabled: true,
            pattern: "*api/users*".to_string(),
            method: None,
            action: InterceptAction::MapLocal {
                file_path: "/tmp/users.json".to_string(),
                status_code: 200,
                headers: std::collections::HashMap::new(),
            },
        };
        let display = format!("{}", rule);
        assert!(display.contains("MapLocal"));
        assert!(display.contains("/tmp/users.json"));
    }

    #[test]
    fn test_map_remote_rule_display() {
        let rule = InterceptRule {
            id: "mr_1".to_string(),
            name: "Remote Redirect".to_string(),
            enabled: true,
            pattern: "*prod.api.com*".to_string(),
            method: None,
            action: InterceptAction::MapRemote {
                target_url: "http://localhost:8080".to_string(),
                preserve_path: true,
            },
        };
        let display = format!("{}", rule);
        assert!(display.contains("MapRemote"));
        assert!(display.contains("localhost:8080"));
    }

    // --- rule_matches edge cases ---

    #[test]
    fn test_rule_matches_pattern_no_match() {
        let rule = make_rule("*totally-different.com*", None, true);
        assert!(!LoggingHandler::rule_matches(
            &rule,
            "https://example.com/api",
            "GET"
        ));
    }

    #[test]
    fn test_rule_matches_wildcard_question_mark() {
        let rule = make_rule("*api/v?/*", None, true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://example.com/api/v1/users",
            "GET"
        ));
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://example.com/api/v2/users",
            "GET"
        ));
        assert!(!LoggingHandler::rule_matches(
            &rule,
            "https://example.com/api/v10/users",
            "GET"
        ));
    }

    #[test]
    fn test_rule_matches_catch_all_pattern() {
        let rule = make_rule("*", None, true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://anything.com/any/path",
            "GET"
        ));
    }

    #[test]
    fn test_rule_matches_empty_url() {
        let rule = make_rule("*", None, true);
        assert!(LoggingHandler::rule_matches(&rule, "", "GET"));
    }

    #[test]
    fn test_rule_matches_method_put_and_patch() {
        let put_rule = make_rule("*api*", Some("PUT"), true);
        assert!(LoggingHandler::rule_matches(
            &put_rule,
            "https://api.com/resource",
            "PUT"
        ));
        assert!(!LoggingHandler::rule_matches(
            &put_rule,
            "https://api.com/resource",
            "PATCH"
        ));

        let patch_rule = make_rule("*api*", Some("PATCH"), true);
        assert!(LoggingHandler::rule_matches(
            &patch_rule,
            "https://api.com/resource",
            "PATCH"
        ));
    }

    // --- Block action defaults serde ---

    #[test]
    fn test_block_action_default_status_code() {
        let json = r#"{"type":"block"}"#;
        let action: InterceptAction = serde_json::from_str(json).unwrap();
        match action {
            InterceptAction::Block { status_code, body } => {
                assert_eq!(status_code, 403);
                assert!(body.is_empty());
            }
            _ => panic!("Expected Block variant"),
        }
    }

    // --- ModifyRequest defaults serde ---

    #[test]
    fn test_modify_request_defaults() {
        let json = r#"{"type":"modify_request"}"#;
        let action: InterceptAction = serde_json::from_str(json).unwrap();
        match action {
            InterceptAction::ModifyRequest {
                add_headers,
                remove_headers,
                set_body,
            } => {
                assert!(add_headers.is_empty());
                assert!(remove_headers.is_empty());
                assert!(set_body.is_none());
            }
            _ => panic!("Expected ModifyRequest variant"),
        }
    }

    // --- ModifyResponse defaults serde ---

    #[test]
    fn test_modify_response_defaults() {
        let json = r#"{"type":"modify_response"}"#;
        let action: InterceptAction = serde_json::from_str(json).unwrap();
        match action {
            InterceptAction::ModifyResponse {
                set_status,
                add_headers,
                remove_headers,
                set_body,
            } => {
                assert!(set_status.is_none());
                assert!(add_headers.is_empty());
                assert!(remove_headers.is_empty());
                assert!(set_body.is_none());
            }
            _ => panic!("Expected ModifyResponse variant"),
        }
    }
}
