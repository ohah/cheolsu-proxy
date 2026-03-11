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
use tracing::{error, info};

use super::handler::LoggingHandler;

/// 인터셉트 마커 헤더(`x-cheolsu-intercepted`, `x-cheolsu-intercept-rule`)를 삽입하는 헬퍼
fn insert_intercept_marker(headers: &mut HeaderMap, rule_id: &str) {
    headers.insert("x-cheolsu-intercepted", HeaderValue::from_static("true"));
    headers.insert(
        "x-cheolsu-intercept-rule",
        rule_id
            .parse()
            .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );
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

/// Rewrite 액션의 정규식 컴파일 + 헤더/바디 치환 + 마커 헤더 삽입을 수행하는 공통 매크로.
/// Request와 Response 모두에서 사용 가능하도록 매크로로 구현한다.
/// `$msg`에 borrow 분리를 위해 `$msg.headers_mut()`, `$msg.body_mut()` 호출을 분리한다.
macro_rules! apply_rewrite_action {
    ($msg:expr, $match_pattern:expr, $replace_with:expr, $is_header:expr, $rule_id:expr, $log_label:expr, $method:expr, $url:expr, $rule_name:expr) => {
        match Regex::new($match_pattern) {
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
                        $msg.headers_mut().remove("content-length");
                        $msg.headers_mut().remove("content-encoding");
                        $msg.headers_mut().remove("transfer-encoding");
                        use http_body_util::Full;
                        *$msg.body_mut() = Body::from(Full::new(new_bytes));
                    }
                }
                insert_intercept_marker($msg.headers_mut(), $rule_id);
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
                    insert_intercept_marker(response.headers_mut(), &rule.id);
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
                        &rule.id,
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

                            let mut resp = response
                                .body(Body::from(http_body_util::Full::new(file_bytes)))
                                .unwrap_or_else(|_| Response::new(Body::empty()));
                            insert_intercept_marker(resp.headers_mut(), &rule.id);
                            return resp.into();
                        }
                        Err(e) => {
                            error!(
                                "[Intercept] Map Local 파일 읽기 실패: {} - {}",
                                file_path, e
                            );
                            let mut err_resp = Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .header("x-cheolsu-map-local-error", e.to_string())
                                .body(Body::from(format!(
                                    "Map Local Error: file not found - {}",
                                    file_path
                                )))
                                .unwrap_or_else(|_| Response::new(Body::empty()));
                            err_resp
                                .headers_mut()
                                .insert("x-cheolsu-intercepted", HeaderValue::from_static("true"));
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
                        insert_intercept_marker(current_req.headers_mut(), &rule.id);
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
                    res.headers_mut().remove("content-length");
                    res.headers_mut().remove("content-encoding");
                    res.headers_mut().remove("transfer-encoding");
                    *res.body_mut() = Body::from(Full::new(body_bytes));
                }

                insert_intercept_marker(res.headers_mut(), &rule.id);
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
                        &rule.id,
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
}
