use crate::protocol::SslProxyingEntry;

/// SSL Proxying 화이트리스트의 도메인 패턴이 주어진 호스트:포트와 매칭되는지 확인합니다.
///
/// 패턴 형식:
/// - 정확한 도메인: `example.com` (포트 무관)
/// - 와일드카드: `*.example.com` (서브도메인 매칭)
/// - 포트 지정: `example.com:443`
/// - 와일드카드 + 포트: `*.example.com:8443`
pub fn matches_ssl_pattern(pattern: &str, host: &str, port: Option<u16>) -> bool {
    // 빈 패턴이나 공백만 있는 패턴은 매칭에서 제외
    if pattern.trim().is_empty() {
        return false;
    }

    let (pattern_host, pattern_port) = parse_host_port(pattern);
    let (target_host, _) = parse_host_port(host);

    // 포트 필터링: 패턴에 포트가 지정되어 있으면 해당 포트만 매칭
    if let Some(p_port) = pattern_port {
        let actual_port = port.unwrap_or(443);
        if p_port != actual_port {
            return false;
        }
    }

    // 호스트 매칭
    match_host_pattern(pattern_host, target_host)
}

/// "host:port" 문자열을 분리합니다.
fn parse_host_port(s: &str) -> (&str, Option<u16>) {
    // IPv6 주소 처리 ([::1]:443 형태)
    if s.starts_with('[') {
        if let Some(bracket_end) = s.find(']') {
            let host = &s[..=bracket_end];
            if s.len() > bracket_end + 1 && s.as_bytes()[bracket_end + 1] == b':' {
                let port = s[bracket_end + 2..].parse::<u16>().ok();
                return (host, port);
            }
            return (host, None);
        }
    }

    if let Some(colon_pos) = s.rfind(':') {
        let maybe_port = &s[colon_pos + 1..];
        if let Ok(port) = maybe_port.parse::<u16>() {
            return (&s[..colon_pos], Some(port));
        }
    }
    (s, None)
}

/// 와일드카드 호스트 패턴 매칭 (대소문자 무시)
fn match_host_pattern(pattern: &str, host: &str) -> bool {
    let pattern_lower = pattern.to_ascii_lowercase();
    let host_lower = host.to_ascii_lowercase();

    if pattern_lower == host_lower {
        return true;
    }

    // *.example.com 패턴: 서브도메인 매칭
    if let Some(stripped) = pattern_lower.strip_prefix("*.") {
        // 정확히 example.com도 매칭 (*.example.com은 example.com도 포함)
        if host_lower == stripped {
            return true;
        }
        // sub.example.com 형태 매칭
        if host_lower.ends_with(stripped) {
            let prefix = &host_lower[..host_lower.len() - stripped.len()];
            return prefix.ends_with('.');
        }
    }

    false
}

/// SSL Proxying 화이트리스트를 기반으로 해당 호스트를 인터셉트해야 하는지 확인합니다.
///
/// - 화이트리스트가 비어있으면 (활성화된 엔트리 없음) → 모든 도메인 인터셉트 (true)
/// - 화이트리스트에 활성화된 엔트리가 있으면 → 매칭되는 도메인만 인터셉트
pub fn should_intercept_ssl(entries: &[SslProxyingEntry], host: &str, port: Option<u16>) -> bool {
    let enabled_entries: Vec<&SslProxyingEntry> = entries.iter().filter(|e| e.enabled).collect();

    // 활성화된 엔트리가 없으면 모든 도메인 인터셉트
    if enabled_entries.is_empty() {
        return true;
    }

    // 활성화된 엔트리 중 매칭되는 것이 있으면 인터셉트
    enabled_entries
        .iter()
        .any(|entry| matches_ssl_pattern(&entry.pattern, host, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_host_port 테스트 ---

    #[test]
    fn test_parse_host_port_with_port() {
        let (host, port) = parse_host_port("example.com:443");
        assert_eq!(host, "example.com");
        assert_eq!(port, Some(443));
    }

    #[test]
    fn test_parse_host_port_without_port() {
        let (host, port) = parse_host_port("example.com");
        assert_eq!(host, "example.com");
        assert_eq!(port, None);
    }

    #[test]
    fn test_parse_host_port_custom_port() {
        let (host, port) = parse_host_port("example.com:8443");
        assert_eq!(host, "example.com");
        assert_eq!(port, Some(8443));
    }

    // --- matches_ssl_pattern 테스트 ---

    #[test]
    fn test_exact_domain_match() {
        assert!(matches_ssl_pattern("example.com", "example.com", None));
    }

    #[test]
    fn test_exact_domain_no_match() {
        assert!(!matches_ssl_pattern("example.com", "other.com", None));
    }

    #[test]
    fn test_exact_domain_case_insensitive() {
        assert!(matches_ssl_pattern("Example.COM", "example.com", None));
        assert!(matches_ssl_pattern("example.com", "EXAMPLE.COM", None));
    }

    #[test]
    fn test_wildcard_subdomain_match() {
        assert!(matches_ssl_pattern(
            "*.example.com",
            "sub.example.com",
            None
        ));
        assert!(matches_ssl_pattern(
            "*.example.com",
            "deep.sub.example.com",
            None
        ));
    }

    #[test]
    fn test_wildcard_matches_base_domain() {
        // *.example.com도 example.com 자체를 매칭
        assert!(matches_ssl_pattern("*.example.com", "example.com", None));
    }

    #[test]
    fn test_wildcard_no_match_different_domain() {
        assert!(!matches_ssl_pattern(
            "*.example.com",
            "notexample.com",
            None
        ));
    }

    #[test]
    fn test_port_match() {
        assert!(matches_ssl_pattern(
            "example.com:443",
            "example.com",
            Some(443)
        ));
    }

    #[test]
    fn test_port_no_match() {
        assert!(!matches_ssl_pattern(
            "example.com:443",
            "example.com",
            Some(8443)
        ));
    }

    #[test]
    fn test_port_default_443() {
        // 포트가 None이면 기본 443으로 간주
        assert!(matches_ssl_pattern("example.com:443", "example.com", None));
    }

    #[test]
    fn test_wildcard_with_port() {
        assert!(matches_ssl_pattern(
            "*.example.com:443",
            "api.example.com",
            Some(443)
        ));
        assert!(!matches_ssl_pattern(
            "*.example.com:443",
            "api.example.com",
            Some(8443)
        ));
    }

    #[test]
    fn test_pattern_without_port_matches_any_port() {
        assert!(matches_ssl_pattern("example.com", "example.com", Some(443)));
        assert!(matches_ssl_pattern(
            "example.com",
            "example.com",
            Some(8443)
        ));
        assert!(matches_ssl_pattern("example.com", "example.com", None));
    }

    // --- should_intercept_ssl 테스트 ---

    #[test]
    fn test_empty_whitelist_intercepts_all() {
        let entries: Vec<SslProxyingEntry> = vec![];
        assert!(should_intercept_ssl(&entries, "any.domain.com", Some(443)));
    }

    #[test]
    fn test_all_disabled_intercepts_all() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com".to_string(),
            enabled: false,
        }];
        assert!(should_intercept_ssl(&entries, "any.domain.com", Some(443)));
    }

    #[test]
    fn test_whitelist_match_intercepts() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com".to_string(),
            enabled: true,
        }];
        assert!(should_intercept_ssl(&entries, "example.com", Some(443)));
    }

    #[test]
    fn test_whitelist_no_match_does_not_intercept() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com".to_string(),
            enabled: true,
        }];
        assert!(!should_intercept_ssl(&entries, "other.com", Some(443)));
    }

    #[test]
    fn test_whitelist_multiple_entries() {
        let entries = vec![
            SslProxyingEntry {
                pattern: "example.com".to_string(),
                enabled: true,
            },
            SslProxyingEntry {
                pattern: "*.api.io".to_string(),
                enabled: true,
            },
        ];
        assert!(should_intercept_ssl(&entries, "example.com", Some(443)));
        assert!(should_intercept_ssl(&entries, "v1.api.io", Some(443)));
        assert!(!should_intercept_ssl(&entries, "other.com", Some(443)));
    }

    #[test]
    fn test_whitelist_mixed_enabled_disabled() {
        let entries = vec![
            SslProxyingEntry {
                pattern: "example.com".to_string(),
                enabled: true,
            },
            SslProxyingEntry {
                pattern: "disabled.com".to_string(),
                enabled: false,
            },
        ];
        // example.com 매칭 (활성화)
        assert!(should_intercept_ssl(&entries, "example.com", Some(443)));
        // disabled.com 미매칭 (비활성화이므로 화이트리스트에 포함되지 않음)
        assert!(!should_intercept_ssl(&entries, "disabled.com", Some(443)));
    }

    #[test]
    fn test_whitelist_with_port_filter() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com:8443".to_string(),
            enabled: true,
        }];
        assert!(should_intercept_ssl(&entries, "example.com", Some(8443)));
        assert!(!should_intercept_ssl(&entries, "example.com", Some(443)));
    }

    #[test]
    fn test_host_with_port_in_host_string() {
        // 호스트 문자열에 포트가 포함된 경우 (authority 형식)
        assert!(matches_ssl_pattern("example.com", "example.com:443", None));
    }

    // --- 빈 패턴 방어 테스트 ---

    #[test]
    fn test_empty_pattern_does_not_match() {
        assert!(!matches_ssl_pattern("", "example.com", None));
        assert!(!matches_ssl_pattern("", "example.com", Some(443)));
    }

    #[test]
    fn test_whitespace_only_pattern_does_not_match() {
        assert!(!matches_ssl_pattern("   ", "example.com", None));
        assert!(!matches_ssl_pattern("\t", "example.com", Some(443)));
        assert!(!matches_ssl_pattern(" \t\n ", "example.com", None));
    }

    #[test]
    fn test_empty_pattern_entry_in_whitelist_ignored() {
        let entries = vec![
            SslProxyingEntry {
                pattern: "".to_string(),
                enabled: true,
            },
            SslProxyingEntry {
                pattern: "   ".to_string(),
                enabled: true,
            },
        ];
        // 빈 패턴만 있는 경우 어떤 도메인도 매칭되지 않아야 함
        assert!(!should_intercept_ssl(&entries, "example.com", Some(443)));
    }

    #[test]
    fn test_empty_pattern_mixed_with_valid_entry() {
        let entries = vec![
            SslProxyingEntry {
                pattern: "".to_string(),
                enabled: true,
            },
            SslProxyingEntry {
                pattern: "example.com".to_string(),
                enabled: true,
            },
        ];
        // 유효한 패턴은 정상 매칭되어야 함
        assert!(should_intercept_ssl(&entries, "example.com", Some(443)));
        assert!(!should_intercept_ssl(&entries, "other.com", Some(443)));
    }
}
