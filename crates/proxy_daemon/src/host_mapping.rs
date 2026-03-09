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
        let path_and_query = uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");

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
}
