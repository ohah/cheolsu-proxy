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
        let mappings = self.intercept.host_mappings.read().await;
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
    /// DNS 호스트명은 대소문자를 구분하지 않으므로 (RFC 4343), 비교 전 소문자로 정규화합니다.
    fn host_pattern_matches(pattern: &str, host: &str) -> bool {
        crate::pattern_utils::wildcard_matches(&pattern.to_lowercase(), &host.to_lowercase())
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

        // effective_port: target_port가 우선, 없으면 원본 URI 포트 유지
        let effective_port = target_port.or(uri.port_u16());

        // IPv6 주소 판별: bracket이 이미 있으면 제거 후 처리
        let is_ipv6 = target_host.starts_with('[')
            || (target_host.contains(':') && !target_host.starts_with('['));
        let bare_host = target_host
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(target_host);

        let authority = if is_ipv6 {
            // IPv6 주소는 brackets로 감싸야 합니다 (e.g., [::1]:8080)
            if let Some(port) = effective_port {
                format!("[{}]:{}", bare_host, port)
            } else {
                format!("[{}]", bare_host)
            }
        } else {
            // IPv4 또는 호스트명
            if let Some(port) = effective_port {
                format!("{}:{}", target_host, port)
            } else {
                target_host.to_string()
            }
        };

        let new_uri_str = format!("{}://{}{}", scheme, authority, path_and_query);
        new_uri_str.parse::<Uri>().ok()
    }

    /// Extract host and port from a URI.
    /// IPv6 주소의 경우 bracket을 제거하여 순수 주소만 반환합니다.
    /// (http 1.x의 Uri::host()는 IPv6 bracket을 포함하여 반환하므로 strip 필요)
    pub(crate) fn extract_host_port(uri: &Uri) -> (Option<String>, Option<u16>) {
        let host = uri.host().map(|h| {
            h.strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .unwrap_or(h)
                .to_string()
        });
        let port = uri.port_u16();
        (host, port)
    }
}

#[cfg(test)]
mod tests;
