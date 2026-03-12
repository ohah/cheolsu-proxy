use crate::protocol::{SslProxyingEntry, SslProxyingMode};

/// SSL/TLS 기본 포트 (HTTPS)
const DEFAULT_SSL_PORT: u16 = 443;

/// 블랙리스트 모드에서 기본적으로 패스스루할 OAuth/인증 관련 도메인 목록.
///
/// 이 도메인들은 프록시가 MITM(Man-in-the-Middle) 인터셉트를 하지 않고
/// 원본 TLS 연결을 그대로 통과(passthrough)시킵니다.
/// SSL 인터셉트 시 인증서 피닝(certificate pinning)이나 OAuth 토큰 교환이
/// 실패할 수 있는 도메인들을 보호하기 위한 기본값입니다.
///
/// 사용자가 별도로 설정하지 않아도 이 목록의 도메인은 항상 패스스루됩니다.
pub const DEFAULT_PASSTHROUGH_DOMAINS: &[&str] = &[
    // --- Google: OAuth 로그인 및 API 서비스 ---
    "accounts.google.com", // Google 계정 로그인/OAuth 인증
    "*.googleapis.com",    // Google API 서비스 (OAuth 토큰 교환 포함)
    //
    // --- Microsoft: Azure AD 및 Microsoft 계정 인증 ---
    "login.microsoftonline.com", // Azure AD / Microsoft 365 OAuth 로그인
    "*.live.com",                // Microsoft 계정 (Outlook, OneDrive 등)
    //
    // --- Apple: Apple ID 및 iCloud 인증 ---
    "appleid.apple.com", // Apple ID 로그인 (인증서 피닝 사용)
    "*.icloud.com",      // iCloud 서비스 인증
    //
    // --- GitHub: Git 및 OAuth 인증 ---
    "github.com",   // GitHub 메인 (Git over HTTPS, OAuth)
    "*.github.com", // GitHub 서브도메인 (API, 인증 콜백 등)
    //
    // --- Auth0: ID 플랫폼 (서드파티 OAuth 위임) ---
    "auth0.com",   // Auth0 메인 도메인
    "*.auth0.com", // Auth0 테넌트별 서브도메인 (*.us.auth0.com 등)
    //
    // --- Okta: 엔터프라이즈 SSO/MFA ---
    "*.okta.com", // Okta SSO 로그인 및 MFA 인증
    //
    // --- Duo Security: 이중 인증(MFA) ---
    "*.duosecurity.com", // Duo MFA 푸시 알림 및 인증
];

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
        let actual_port = port.unwrap_or(DEFAULT_SSL_PORT);
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

/// SSL Proxying 모드와 목록을 기반으로 해당 호스트를 인터셉트해야 하는지 확인합니다.
///
/// ## Blacklist 모드 (기본)
/// - 모든 도메인을 인터셉트하되, 목록에 매칭되는 도메인은 패스스루
/// - 기본 OAuth 도메인(DEFAULT_PASSTHROUGH_DOMAINS)도 항상 패스스루
///
/// ## Whitelist 모드
/// - 목록이 비어있으면 모든 도메인 인터셉트
/// - 목록에 활성화된 엔트리가 있으면 매칭되는 도메인만 인터셉트
pub fn should_intercept_ssl(
    mode: &SslProxyingMode,
    entries: &[SslProxyingEntry],
    host: &str,
    port: Option<u16>,
) -> bool {
    let enabled_entries: Vec<&SslProxyingEntry> = entries.iter().filter(|e| e.enabled).collect();

    match mode {
        SslProxyingMode::Blacklist => {
            // 기본 OAuth 도메인 체크
            for pattern in DEFAULT_PASSTHROUGH_DOMAINS {
                if matches_ssl_pattern(pattern, host, port) {
                    return false;
                }
            }
            // 사용자 지정 목록 체크
            if enabled_entries.is_empty() {
                return true;
            }
            // 매칭되면 패스스루 (인터셉트 안 함)
            !enabled_entries
                .iter()
                .any(|entry| matches_ssl_pattern(&entry.pattern, host, port))
        }
        SslProxyingMode::Whitelist => {
            // 활성화된 엔트리가 없으면 모든 도메인 인터셉트
            if enabled_entries.is_empty() {
                return true;
            }
            // 활성화된 엔트리 중 매칭되는 것이 있으면 인터셉트
            enabled_entries
                .iter()
                .any(|entry| matches_ssl_pattern(&entry.pattern, host, port))
        }
    }
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
        // 포트가 None이면 기본 DEFAULT_SSL_PORT(443)로 간주
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

    // --- should_intercept_ssl 테스트 (Whitelist 모드) ---

    #[test]
    fn test_whitelist_empty_intercepts_all() {
        let entries: Vec<SslProxyingEntry> = vec![];
        assert!(should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "any.domain.com",
            Some(443)
        ));
    }

    #[test]
    fn test_whitelist_all_disabled_intercepts_all() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com".to_string(),
            enabled: false,
        }];
        assert!(should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "any.domain.com",
            Some(443)
        ));
    }

    #[test]
    fn test_whitelist_match_intercepts() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com".to_string(),
            enabled: true,
        }];
        assert!(should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "example.com",
            Some(443)
        ));
    }

    #[test]
    fn test_whitelist_no_match_does_not_intercept() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com".to_string(),
            enabled: true,
        }];
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "other.com",
            Some(443)
        ));
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
        assert!(should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "example.com",
            Some(443)
        ));
        assert!(should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "v1.api.io",
            Some(443)
        ));
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "other.com",
            Some(443)
        ));
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
        assert!(should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "example.com",
            Some(443)
        ));
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "disabled.com",
            Some(443)
        ));
    }

    #[test]
    fn test_whitelist_with_port_filter() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com:8443".to_string(),
            enabled: true,
        }];
        assert!(should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "example.com",
            Some(8443)
        ));
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "example.com",
            Some(443)
        ));
    }

    #[test]
    fn test_host_with_port_in_host_string() {
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
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "example.com",
            Some(443)
        ));
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
        assert!(should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "example.com",
            Some(443)
        ));
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Whitelist,
            &entries,
            "other.com",
            Some(443)
        ));
    }

    // --- should_intercept_ssl 테스트 (Blacklist 모드) ---

    #[test]
    fn test_blacklist_empty_intercepts_all() {
        let entries: Vec<SslProxyingEntry> = vec![];
        assert!(should_intercept_ssl(
            &SslProxyingMode::Blacklist,
            &entries,
            "any.domain.com",
            Some(443)
        ));
    }

    #[test]
    fn test_blacklist_match_passthrough() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com".to_string(),
            enabled: true,
        }];
        // 매칭되면 패스스루 (인터셉트 안 함)
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Blacklist,
            &entries,
            "example.com",
            Some(443)
        ));
    }

    #[test]
    fn test_blacklist_no_match_intercepts() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com".to_string(),
            enabled: true,
        }];
        // 매칭 안 되면 인터셉트
        assert!(should_intercept_ssl(
            &SslProxyingMode::Blacklist,
            &entries,
            "other.com",
            Some(443)
        ));
    }

    #[test]
    fn test_blacklist_default_oauth_passthrough() {
        let entries: Vec<SslProxyingEntry> = vec![];
        // 기본 OAuth 도메인은 항상 패스스루
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Blacklist,
            &entries,
            "accounts.google.com",
            Some(443)
        ));
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Blacklist,
            &entries,
            "login.microsoftonline.com",
            Some(443)
        ));
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Blacklist,
            &entries,
            "api.googleapis.com",
            Some(443)
        ));
    }

    #[test]
    fn test_blacklist_disabled_entry_ignored() {
        let entries = vec![SslProxyingEntry {
            pattern: "example.com".to_string(),
            enabled: false,
        }];
        // 비활성화된 엔트리는 무시 → 인터셉트
        assert!(should_intercept_ssl(
            &SslProxyingMode::Blacklist,
            &entries,
            "example.com",
            Some(443)
        ));
    }

    #[test]
    fn test_blacklist_wildcard_passthrough() {
        let entries = vec![SslProxyingEntry {
            pattern: "*.example.com".to_string(),
            enabled: true,
        }];
        assert!(!should_intercept_ssl(
            &SslProxyingMode::Blacklist,
            &entries,
            "api.example.com",
            Some(443)
        ));
        assert!(should_intercept_ssl(
            &SslProxyingMode::Blacklist,
            &entries,
            "other.com",
            Some(443)
        ));
    }
}
