use crate::protocol::{InterceptAction, InterceptRule, ServerReplayEntry};
use bytes::Bytes;
use proxyapi_v2::hyper::Request;
use proxyapi_v2::{
    hyper::http::{HeaderName, HeaderValue, StatusCode},
    hyper::Response,
    Body, RequestOrResponse,
};
use regex::Regex;
use tracing::{error, info};

use super::handler::LoggingHandler;

impl LoggingHandler {
    /// 서버 리플레이 매칭: method + URL이 일치하는 엔트리 검색
    pub(crate) async fn find_server_replay_match(
        &self,
        url: &str,
        method: &str,
    ) -> Option<ServerReplayEntry> {
        let entries = self.server_replay_entries.lock().await;
        entries
            .iter()
            .find(|entry| entry.method.eq_ignore_ascii_case(method) && entry.url == url)
            .cloned()
    }

    /// 와일드카드 패턴 매칭 (* = 임의 문자열, ? = 단일 문자)
    pub(crate) fn wildcard_matches(pattern: &str, text: &str) -> bool {
        let regex_pattern = format!(
            "(?i){}",
            regex::escape(pattern)
                .replace("\\*", ".*")
                .replace("\\?", ".")
        );
        Regex::new(&regex_pattern)
            .map(|re| re.is_match(text))
            .unwrap_or(false)
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
        Self::wildcard_matches(&rule.pattern, url)
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
        let rules_guard = self.intercept_rules.lock().await;
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
                        .header("x-cheolsu-intercepted", "true")
                        .header("x-cheolsu-intercept-rule", &rule.id)
                        .body(Body::from(body.clone()))
                        .unwrap_or_else(|_| {
                            Response::builder()
                                .status(StatusCode::FORBIDDEN)
                                .body(Body::from("Blocked by intercept rule"))
                                .unwrap_or_else(|_| Response::new(Body::empty()))
                        });
                    // Content-Type 설정
                    if !body.is_empty() {
                        if body.starts_with('{') || body.starts_with('[') {
                            response
                                .headers_mut()
                                .insert("content-type", "application/json".parse().unwrap_or_else(|_| HeaderValue::from_static("application/json")));
                        } else {
                            response.headers_mut().insert(
                                "content-type",
                                "text/plain; charset=utf-8".parse().unwrap_or_else(|_| HeaderValue::from_static("text/plain; charset=utf-8")),
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
                    // 헤더 제거
                    for name in remove_headers {
                        if let Ok(header_name) = name.parse::<HeaderName>() {
                            current_req.headers_mut().remove(header_name);
                        }
                    }
                    // 헤더 추가
                    for (name, value) in add_headers {
                        if let (Ok(header_name), Ok(header_value)) =
                            (name.parse::<HeaderName>(), value.parse::<HeaderValue>())
                        {
                            current_req.headers_mut().insert(header_name, header_value);
                        }
                    }
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
                                .header("x-cheolsu-intercepted", "true")
                                .header("x-cheolsu-intercept-rule", &rule.id)
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

                            return response
                                .body(Body::from(http_body_util::Full::new(file_bytes)))
                                .unwrap_or_else(|_| {
                                    Response::builder()
                                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                                        .body(Body::from("Failed to build map local response"))
                                        .unwrap_or_else(|_| Response::new(Body::empty()))
                                })
                                .into();
                        }
                        Err(e) => {
                            error!(
                                "[Intercept] Map Local 파일 읽기 실패: {} - {}",
                                file_path, e
                            );
                            return Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .header("x-cheolsu-intercepted", "true")
                                .header("x-cheolsu-map-local-error", e.to_string())
                                .body(Body::from(format!(
                                    "Map Local Error: file not found - {}",
                                    file_path
                                )))
                                .unwrap_or_else(|_| Response::new(Body::empty()))
                                .into();
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
                        current_req
                            .headers_mut()
                            .insert("x-cheolsu-intercepted", HeaderValue::from_static("true"));
                        current_req.headers_mut().insert(
                            "x-cheolsu-map-remote-original",
                            url.parse().unwrap_or_else(|_| HeaderValue::from_static("unknown")),
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

                // 헤더 제거
                for name in remove_headers {
                    if let Ok(header_name) = name.parse::<HeaderName>() {
                        res.headers_mut().remove(header_name);
                    }
                }

                // 헤더 추가
                for (name, value) in add_headers {
                    if let (Ok(header_name), Ok(header_value)) =
                        (name.parse::<HeaderName>(), value.parse::<HeaderValue>())
                    {
                        res.headers_mut().insert(header_name, header_value);
                    }
                }

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

                res.headers_mut()
                    .insert("x-cheolsu-intercepted", HeaderValue::from_static("true"));
                res.headers_mut().insert(
                    "x-cheolsu-intercept-rule",
                    rule.id
                        .parse()
                        .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
                );
            }
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{InterceptAction, InterceptRule};

    // --- wildcard_matches 테스트 ---

    #[test]
    fn test_wildcard_star_prefix() {
        assert!(LoggingHandler::wildcard_matches(
            "*ads.example.com*",
            "https://ads.example.com/banner"
        ));
    }

    #[test]
    fn test_wildcard_subdomain_and_path() {
        assert!(LoggingHandler::wildcard_matches(
            "*.example.com/api/*",
            "https://sub.example.com/api/v1/users"
        ));
    }

    #[test]
    fn test_wildcard_no_match() {
        assert!(!LoggingHandler::wildcard_matches(
            "*ads.example.com*",
            "https://other.com/page"
        ));
    }

    #[test]
    fn test_wildcard_exact_domain() {
        assert!(LoggingHandler::wildcard_matches(
            "*example.com*",
            "https://example.com"
        ));
    }

    #[test]
    fn test_wildcard_path_only() {
        assert!(LoggingHandler::wildcard_matches(
            "*/api/v1/*",
            "https://any.com/api/v1/users"
        ));
        assert!(!LoggingHandler::wildcard_matches(
            "*/api/v1/*",
            "https://any.com/api/v2/users"
        ));
    }

    #[test]
    fn test_wildcard_question_mark() {
        assert!(LoggingHandler::wildcard_matches(
            "*api/v?/users*",
            "https://example.com/api/v1/users"
        ));
        assert!(LoggingHandler::wildcard_matches(
            "*api/v?/users*",
            "https://example.com/api/v2/users"
        ));
        assert!(!LoggingHandler::wildcard_matches(
            "*api/v?/users*",
            "https://example.com/api/v10/users"
        ));
    }

    #[test]
    fn test_wildcard_case_insensitive() {
        assert!(LoggingHandler::wildcard_matches(
            "*Example.COM*",
            "https://example.com/page"
        ));
    }

    #[test]
    fn test_wildcard_catch_all() {
        assert!(LoggingHandler::wildcard_matches(
            "*",
            "https://anything.com/any/path"
        ));
    }

    #[test]
    fn test_wildcard_no_wildcards_partial() {
        assert!(LoggingHandler::wildcard_matches(
            "example.com",
            "https://example.com/api"
        ));
    }

    #[test]
    fn test_wildcard_special_chars_escaped() {
        assert!(LoggingHandler::wildcard_matches(
            "*example.com/api?key=*",
            "https://example.com/api?key=value"
        ));
    }

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
}
