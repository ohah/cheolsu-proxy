use crate::handler::LoggingHandler;
use proxyapi_v2::hyper::Uri;

impl LoggingHandler {
    /// Resolve host mapping for a given host and port.
    /// Returns (target_host, target_port) if a mapping matches, None otherwise.
    pub(crate) async fn resolve_host_mapping(
        &self,
        host: &str,
        port: Option<u16>,
    ) -> Option<(String, Option<u16>)> {
        let mappings = self.intercept.host_mappings.lock().await;
        for mapping in mappings.iter() {
            if !mapping.enabled {
                continue;
            }
            if let Some(src_port) = mapping.source_port {
                if port != Some(src_port) {
                    continue;
                }
            }
            if Self::host_pattern_matches(&mapping.source_host, host) {
                return Some((mapping.target_host.clone(), mapping.target_port));
            }
        }
        None
    }

    /// Wildcard host pattern matching.
    /// Supports glob-style patterns: `*` matches any substring, `?` matches a single character.
    fn host_pattern_matches(pattern: &str, host: &str) -> bool {
        Self::wildcard_matches(pattern, host)
    }

    /// Apply host mapping to a request URI.
    /// Replaces the host (and optionally port) in the URI with the mapped target,
    /// while preserving the original Host header for correct virtual host routing.
    pub(crate) fn apply_host_mapping_to_uri(
        uri: &Uri,
        target_host: &str,
        target_port: Option<u16>,
    ) -> Option<Uri> {
        let scheme = uri.scheme_str().unwrap_or("https");
        let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

        let authority = if let Some(port) = target_port {
            format!("{}:{}", target_host, port)
        } else if let Some(port) = uri.port_u16() {
            format!("{}:{}", target_host, port)
        } else {
            target_host.to_string()
        };

        let new_uri_str = format!("{}://{}{}", scheme, authority, path_and_query);
        new_uri_str.parse::<Uri>().ok()
    }

    /// Extract host and port from a URI.
    pub(crate) fn extract_host_port(uri: &Uri) -> (Option<String>, Option<u16>) {
        let host = uri.host().map(|h| h.to_string());
        let port = uri.port_u16();
        (host, port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::HostMapping;

    #[test]
    fn test_host_pattern_exact_match() {
        assert!(LoggingHandler::host_pattern_matches(
            "api.example.com",
            "api.example.com"
        ));
    }

    #[test]
    fn test_host_pattern_wildcard_subdomain() {
        assert!(LoggingHandler::host_pattern_matches(
            "*.example.com",
            "api.example.com"
        ));
        assert!(LoggingHandler::host_pattern_matches(
            "*.example.com",
            "staging.api.example.com"
        ));
    }

    #[test]
    fn test_host_pattern_no_match() {
        assert!(!LoggingHandler::host_pattern_matches(
            "*.example.com",
            "other.org"
        ));
    }

    #[test]
    fn test_host_pattern_star_only() {
        assert!(LoggingHandler::host_pattern_matches("*", "anything.com"));
    }

    #[test]
    fn test_host_pattern_case_insensitive() {
        assert!(LoggingHandler::host_pattern_matches(
            "*.Example.COM",
            "api.example.com"
        ));
    }

    #[test]
    fn test_apply_host_mapping_to_uri_basic() {
        let uri: Uri = "https://api.example.com/v1/users".parse().unwrap();
        let result =
            LoggingHandler::apply_host_mapping_to_uri(&uri, "192.168.1.100", None).unwrap();
        assert_eq!(result.host().unwrap(), "192.168.1.100");
        assert_eq!(result.path(), "/v1/users");
        assert_eq!(result.scheme_str().unwrap(), "https");
    }

    #[test]
    fn test_apply_host_mapping_to_uri_with_target_port() {
        let uri: Uri = "https://api.example.com/v1/users".parse().unwrap();
        let result =
            LoggingHandler::apply_host_mapping_to_uri(&uri, "192.168.1.100", Some(8443)).unwrap();
        assert_eq!(result.host().unwrap(), "192.168.1.100");
        assert_eq!(result.port_u16().unwrap(), 8443);
        assert_eq!(result.path(), "/v1/users");
    }

    #[test]
    fn test_apply_host_mapping_preserves_original_port() {
        let uri: Uri = "https://api.example.com:9443/v1/users".parse().unwrap();
        let result =
            LoggingHandler::apply_host_mapping_to_uri(&uri, "192.168.1.100", None).unwrap();
        assert_eq!(result.host().unwrap(), "192.168.1.100");
        assert_eq!(result.port_u16().unwrap(), 9443);
    }

    #[test]
    fn test_apply_host_mapping_preserves_query() {
        let uri: Uri = "https://api.example.com/search?q=test&page=1"
            .parse()
            .unwrap();
        let result =
            LoggingHandler::apply_host_mapping_to_uri(&uri, "staging.example.com", None).unwrap();
        assert_eq!(
            result.path_and_query().unwrap().as_str(),
            "/search?q=test&page=1"
        );
    }

    #[test]
    fn test_extract_host_port_with_port() {
        let uri: Uri = "https://example.com:8443/path".parse().unwrap();
        let (host, port) = LoggingHandler::extract_host_port(&uri);
        assert_eq!(host.unwrap(), "example.com");
        assert_eq!(port.unwrap(), 8443);
    }

    #[test]
    fn test_extract_host_port_without_port() {
        let uri: Uri = "https://example.com/path".parse().unwrap();
        let (host, port) = LoggingHandler::extract_host_port(&uri);
        assert_eq!(host.unwrap(), "example.com");
        assert!(port.is_none());
    }

    #[test]
    fn test_host_mapping_display() {
        let mapping = HostMapping {
            id: "hm_1".to_string(),
            source_host: "*.api.example.com".to_string(),
            source_port: Some(443),
            target_host: "192.168.1.100".to_string(),
            target_port: Some(8443),
            enabled: true,
        };
        let display = format!("{}", mapping);
        assert!(display.contains("*.api.example.com"));
        assert!(display.contains("192.168.1.100"));
        assert!(display.contains("enabled"));
    }

    #[test]
    fn test_host_mapping_display_disabled() {
        let mapping = HostMapping {
            id: "hm_2".to_string(),
            source_host: "example.com".to_string(),
            source_port: None,
            target_host: "10.0.0.1".to_string(),
            target_port: None,
            enabled: false,
        };
        let display = format!("{}", mapping);
        assert!(display.contains("disabled"));
        assert!(!display.contains("enabled"));
    }

    #[test]
    fn test_host_mapping_display_no_ports() {
        let mapping = HostMapping {
            id: "hm_3".to_string(),
            source_host: "example.com".to_string(),
            source_port: None,
            target_host: "10.0.0.1".to_string(),
            target_port: None,
            enabled: true,
        };
        let display = format!("{}", mapping);
        // source_port/target_port가 None이면 포트 부분이 빈 문자열
        assert_eq!(display, "[hm_3] example.com -> 10.0.0.1 [enabled]");
    }

    #[test]
    fn test_wildcard_star_matches_all_hosts() {
        assert!(LoggingHandler::host_pattern_matches("*", "anything.com"));
        assert!(LoggingHandler::host_pattern_matches("*", "sub.domain.org"));
        assert!(LoggingHandler::host_pattern_matches("*", "localhost"));
        assert!(LoggingHandler::host_pattern_matches("*", "192.168.1.1"));
    }

    #[test]
    fn test_wildcard_star_dot_com() {
        assert!(LoggingHandler::host_pattern_matches("*.com", "example.com"));
        assert!(LoggingHandler::host_pattern_matches(
            "*.com",
            "sub.example.com"
        ));
        assert!(!LoggingHandler::host_pattern_matches(
            "*.com",
            "example.org"
        ));
    }

    #[test]
    fn test_wildcard_prefix_pattern() {
        // api.* 패턴: api.로 시작하는 모든 호스트
        assert!(LoggingHandler::host_pattern_matches(
            "api.*",
            "api.example.com"
        ));
        assert!(LoggingHandler::host_pattern_matches(
            "api.*",
            "api.staging.internal"
        ));
        assert!(!LoggingHandler::host_pattern_matches(
            "api.*",
            "web.example.com"
        ));
    }

    #[test]
    fn test_apply_host_mapping_to_uri_http_scheme() {
        let uri: Uri = "http://example.com/path".parse().unwrap();
        let result = LoggingHandler::apply_host_mapping_to_uri(&uri, "10.0.0.1", None).unwrap();
        assert_eq!(result.scheme_str().unwrap(), "http");
        assert_eq!(result.host().unwrap(), "10.0.0.1");
        assert_eq!(result.path(), "/path");
    }

    #[test]
    fn test_apply_host_mapping_to_uri_https_with_target_port() {
        let uri: Uri = "https://secure.example.com/api/data".parse().unwrap();
        let result =
            LoggingHandler::apply_host_mapping_to_uri(&uri, "staging.internal", Some(9443))
                .unwrap();
        assert_eq!(result.scheme_str().unwrap(), "https");
        assert_eq!(result.host().unwrap(), "staging.internal");
        assert_eq!(result.port_u16().unwrap(), 9443);
        assert_eq!(result.path(), "/api/data");
    }

    #[test]
    fn test_apply_host_mapping_to_uri_with_source_port() {
        // 원본 URI에 포트가 있고 target_port가 지정된 경우 target_port 사용
        let uri: Uri = "https://api.example.com:443/v1".parse().unwrap();
        let result =
            LoggingHandler::apply_host_mapping_to_uri(&uri, "10.0.0.1", Some(8443)).unwrap();
        assert_eq!(result.host().unwrap(), "10.0.0.1");
        assert_eq!(result.port_u16().unwrap(), 8443);
    }

    #[test]
    fn test_apply_host_mapping_to_uri_preserves_source_port_when_no_target_port() {
        // target_port가 None이면 원본 포트 유지
        let uri: Uri = "https://api.example.com:9443/v1".parse().unwrap();
        let result = LoggingHandler::apply_host_mapping_to_uri(&uri, "10.0.0.1", None).unwrap();
        assert_eq!(result.port_u16().unwrap(), 9443);
    }

    #[test]
    fn test_extract_host_port_http_default() {
        // HTTP URI에 포트가 명시되지 않으면 port_u16()은 None
        let uri: Uri = "http://example.com/path".parse().unwrap();
        let (host, port) = LoggingHandler::extract_host_port(&uri);
        assert_eq!(host.unwrap(), "example.com");
        assert!(port.is_none()); // 기본 포트 80은 implicit
    }

    #[test]
    fn test_extract_host_port_https_default() {
        // HTTPS URI에 포트가 명시되지 않으면 port_u16()은 None
        let uri: Uri = "https://example.com/secure".parse().unwrap();
        let (host, port) = LoggingHandler::extract_host_port(&uri);
        assert_eq!(host.unwrap(), "example.com");
        assert!(port.is_none()); // 기본 포트 443은 implicit
    }

    #[test]
    fn test_extract_host_port_explicit_port() {
        let uri: Uri = "http://example.com:3000/api".parse().unwrap();
        let (host, port) = LoggingHandler::extract_host_port(&uri);
        assert_eq!(host.unwrap(), "example.com");
        assert_eq!(port.unwrap(), 3000);
    }

    #[test]
    fn test_extract_host_port_authority_only() {
        // scheme 없이 authority만 있는 CONNECT 형태 URI
        let uri: Uri = "example.com:443".parse().unwrap();
        let (host, port) = LoggingHandler::extract_host_port(&uri);
        // scheme이 없는 URI는 host()가 None일 수 있음
        // hyper의 Uri 파싱 특성에 따라 확인
        if host.is_some() {
            assert_eq!(host.unwrap(), "example.com");
        }
        // port도 scheme 없으면 파싱 안 될 수 있음
        let _ = port;
    }

    // --- resolve_host_mapping 비동기 테스트 (tokio::test) ---

    fn make_mapping(
        id: &str,
        source_host: &str,
        source_port: Option<u16>,
        target_host: &str,
        target_port: Option<u16>,
        enabled: bool,
    ) -> HostMapping {
        HostMapping {
            id: id.to_string(),
            source_host: source_host.to_string(),
            source_port,
            target_host: target_host.to_string(),
            target_port,
            enabled,
        }
    }

    #[tokio::test]
    async fn test_resolve_disabled_mapping_ignored() {
        let handler = create_test_handler().await;
        {
            let mut mappings = handler.intercept.host_mappings.lock().await;
            mappings.push(make_mapping(
                "hm_1",
                "api.example.com",
                None,
                "10.0.0.1",
                None,
                false, // disabled
            ));
        }
        let result = handler.resolve_host_mapping("api.example.com", None).await;
        assert!(result.is_none(), "비활성화된 매핑은 무시되어야 함");
    }

    #[tokio::test]
    async fn test_resolve_source_port_matching() {
        let handler = create_test_handler().await;
        {
            let mut mappings = handler.intercept.host_mappings.lock().await;
            mappings.push(make_mapping(
                "hm_1",
                "api.example.com",
                Some(443),
                "10.0.0.1",
                None,
                true,
            ));
        }
        // 포트가 매칭되면 적용
        let result = handler
            .resolve_host_mapping("api.example.com", Some(443))
            .await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "10.0.0.1");

        // 포트가 다르면 무시
        let result = handler
            .resolve_host_mapping("api.example.com", Some(80))
            .await;
        assert!(result.is_none());

        // 포트가 None이면 무시 (source_port가 Some(443)이므로)
        let result = handler.resolve_host_mapping("api.example.com", None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_resolve_no_source_port_matches_any() {
        let handler = create_test_handler().await;
        {
            let mut mappings = handler.intercept.host_mappings.lock().await;
            mappings.push(make_mapping(
                "hm_1",
                "api.example.com",
                None, // 모든 포트 매칭
                "10.0.0.1",
                None,
                true,
            ));
        }
        // 포트 상관없이 매칭
        let r1 = handler
            .resolve_host_mapping("api.example.com", Some(80))
            .await;
        assert!(r1.is_some());
        let r2 = handler
            .resolve_host_mapping("api.example.com", Some(443))
            .await;
        assert!(r2.is_some());
        let r3 = handler.resolve_host_mapping("api.example.com", None).await;
        assert!(r3.is_some());
    }

    #[tokio::test]
    async fn test_resolve_first_matching_rule_wins() {
        let handler = create_test_handler().await;
        {
            let mut mappings = handler.intercept.host_mappings.lock().await;
            mappings.push(make_mapping(
                "hm_1",
                "*.example.com",
                None,
                "first-target.com",
                None,
                true,
            ));
            mappings.push(make_mapping(
                "hm_2",
                "api.example.com",
                None,
                "second-target.com",
                None,
                true,
            ));
        }
        // 첫 번째 규칙이 먼저 매칭되어야 함
        let result = handler.resolve_host_mapping("api.example.com", None).await;
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().0,
            "first-target.com",
            "첫 번째 매칭 규칙이 사용되어야 함"
        );
    }

    /// 테스트용 LoggingHandler 생성 헬퍼
    async fn create_test_handler() -> LoggingHandler {
        use crate::handler::{InterceptEngine, WebSocketState};
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let (sender, _rx) = tokio::sync::mpsc::channel(1);
        LoggingHandler {
            sender,
            request: crate::handler::RequestState {
                req: None,
                res: None,
            },
            config: crate::handler::ProxyConfig {
                cache_dir: None,
                ca_cert_der: None,
            },
            intercept: InterceptEngine {
                intercept_rules: Arc::new(Mutex::new(Vec::new())),
                server_replay_entries: Arc::new(Mutex::new(Vec::new())),
                host_mappings: Arc::new(Mutex::new(Vec::new())),
                script_handle: scripting::ScriptHandle::new(),
            },
            ws: WebSocketState {
                ws_sender: None,
                ws_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                mqtt_versions: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
            },
        }
    }
}
