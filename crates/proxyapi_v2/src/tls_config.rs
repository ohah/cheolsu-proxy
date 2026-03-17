//! TLS 버전/암호화 스위트 세분화 설정 모듈.
//!
//! 도메인별로 클라이언트↔프록시, 프록시↔서버 방향의 TLS 설정을 독립적으로 구성할 수 있습니다.
//! 현재 하드코딩된 Apple 서비스/특수 도메인 설정을 규칙 기반으로 전환합니다.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

/// TLS 버전 (설정용)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TlsVersionConfig {
    #[serde(rename = "1.0")]
    Tls10,
    #[serde(rename = "1.1")]
    Tls11,
    #[serde(rename = "1.2")]
    Tls12,
    #[serde(rename = "1.3")]
    Tls13,
}

impl std::fmt::Display for TlsVersionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsVersionConfig::Tls10 => write!(f, "TLS 1.0"),
            TlsVersionConfig::Tls11 => write!(f, "TLS 1.1"),
            TlsVersionConfig::Tls12 => write!(f, "TLS 1.2"),
            TlsVersionConfig::Tls13 => write!(f, "TLS 1.3"),
        }
    }
}

impl TlsVersionConfig {
    /// OpenSSL `SslVersion`으로 변환
    #[cfg(feature = "openssl-ca")]
    pub fn to_openssl_version(self) -> openssl::ssl::SslVersion {
        match self {
            TlsVersionConfig::Tls10 => openssl::ssl::SslVersion::TLS1,
            TlsVersionConfig::Tls11 => openssl::ssl::SslVersion::TLS1_1,
            TlsVersionConfig::Tls12 => openssl::ssl::SslVersion::TLS1_2,
            TlsVersionConfig::Tls13 => openssl::ssl::SslVersion::TLS1_3,
        }
    }

    /// rustls 지원 여부 (TLS 1.2, 1.3만 지원)
    pub fn is_rustls_supported(self) -> bool {
        matches!(self, TlsVersionConfig::Tls12 | TlsVersionConfig::Tls13)
    }
}

/// 방향별 TLS 설정
///
/// `version_min`/`version_max`는 현재 OpenSSL SslContext 레벨에서 사용됩니다.
/// 연결 레벨에서는 클라이언트 ClientHello 버전을 존중하여 고정합니다.
/// 향후 버전 범위 clamping이 구현되면 연결 레벨에서도 활용됩니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionalTlsConfig {
    /// 최소 TLS 버전
    pub version_min: TlsVersionConfig,
    /// 최대 TLS 버전
    pub version_max: TlsVersionConfig,
    /// OpenSSL cipher 문자열 (None이면 기본값 사용)
    pub cipher_list: Option<String>,
}

impl Default for DirectionalTlsConfig {
    fn default() -> Self {
        Self {
            version_min: TlsVersionConfig::Tls12,
            version_max: TlsVersionConfig::Tls13,
            cipher_list: None,
        }
    }
}

impl DirectionalTlsConfig {
    /// 레거시 호환용 (TLS 1.0~1.3, 관대한 cipher)
    pub fn legacy() -> Self {
        Self {
            version_min: TlsVersionConfig::Tls10,
            version_max: TlsVersionConfig::Tls13,
            cipher_list: Some("@SECLEVEL=0:ALL:!aNULL:!eNULL".to_string()),
        }
    }
}

/// 도메인별 TLS 설정 규칙
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfigRule {
    /// 도메인 패턴
    /// - 정확히 일치: `"api2.cursor.sh"`
    /// - 와일드카드 접미사: `"*.apple.com"` (`.apple.com`으로 끝나는 모든 도메인)
    pub domain_pattern: String,
    /// 클라이언트 → 프록시 방향 TLS 설정
    pub client_config: DirectionalTlsConfig,
    /// 프록시 → 서버 방향 TLS 설정
    pub server_config: DirectionalTlsConfig,
    /// OpenSSL 사용 강제 (rustls 대신)
    #[serde(default)]
    pub require_openssl: bool,
    /// 핸드셰이크 타임아웃 (초, None이면 기본값)
    #[serde(default)]
    pub handshake_timeout_secs: Option<u64>,
    /// 서버 인증서 검증 비활성화
    #[serde(default)]
    pub disable_cert_verify: bool,
    /// 우선순위 (낮을수록 먼저 매칭, 기본 100)
    #[serde(default = "default_priority")]
    pub priority: u32,
}

fn default_priority() -> u32 {
    100
}

/// 도메인 매칭 결과 — 해당 도메인에 적용할 설정
#[derive(Debug, Clone)]
pub struct ResolvedTlsConfig {
    pub client_config: DirectionalTlsConfig,
    pub server_config: DirectionalTlsConfig,
    pub require_openssl: bool,
    pub handshake_timeout_secs: Option<u64>,
    pub disable_cert_verify: bool,
    /// 매칭된 규칙의 도메인 패턴 (디버깅용)
    pub matched_pattern: Option<String>,
}

/// TLS 설정 관리자
///
/// 도메인 패턴 기반으로 TLS 설정을 조회합니다.
/// 규칙은 우선순위 순으로 정렬되어 첫 번째 매칭 규칙이 적용됩니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfigManager {
    rules: Vec<TlsConfigRule>,
    /// 규칙에 매칭되지 않는 도메인의 기본 클라이언트 방향 설정
    pub default_client: DirectionalTlsConfig,
    /// 규칙에 매칭되지 않는 도메인의 기본 서버 방향 설정
    pub default_server: DirectionalTlsConfig,
}

impl Default for TlsConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TlsConfigManager {
    /// 기본 규칙 없이 빈 설정 관리자 생성
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            default_client: DirectionalTlsConfig::default(),
            default_server: DirectionalTlsConfig::default(),
        }
    }

    /// 기존 하드코딩된 설정을 규칙으로 포함하는 설정 관리자 생성
    pub fn with_builtin_rules() -> Self {
        let mut manager = Self::new();

        // Apple 서비스 규칙 — 기존 하드코딩을 규칙으로 전환
        let apple_cipher =
            "ECDHE+AESGCM:ECDHE+CHACHA20:DHE+AESGCM:DHE+CHACHA20:!aNULL:!MD5:!DSS".to_string();
        let apple_client_config = DirectionalTlsConfig {
            version_min: TlsVersionConfig::Tls12,
            version_max: TlsVersionConfig::Tls13,
            cipher_list: Some(apple_cipher.clone()),
        };

        manager.add_rule(TlsConfigRule {
            domain_pattern: "*.apple.com".to_string(),
            client_config: apple_client_config.clone(),
            server_config: DirectionalTlsConfig::default(),
            require_openssl: false,
            handshake_timeout_secs: Some(15),
            disable_cert_verify: true,
            priority: 50,
        });

        manager.add_rule(TlsConfigRule {
            domain_pattern: "*.icloud.com".to_string(),
            client_config: apple_client_config,
            server_config: DirectionalTlsConfig::default(),
            require_openssl: false,
            handshake_timeout_secs: Some(15),
            disable_cert_verify: true,
            priority: 50,
        });

        // OpenSSL 필수 도메인 규칙
        let openssl_required_domains = [
            "api2.cursor.sh",
            "wps.apple.com",
            "gdmf.apple.com",
            "fbs.smoot.apple.com",
            "gateway.icloud.com",
        ];
        for domain in openssl_required_domains {
            manager.add_rule(TlsConfigRule {
                domain_pattern: domain.to_string(),
                client_config: DirectionalTlsConfig::default(),
                server_config: DirectionalTlsConfig::default(),
                require_openssl: true,
                handshake_timeout_secs: None,
                disable_cert_verify: false,
                priority: 10, // 정확한 도메인 매칭은 높은 우선순위
            });
        }

        manager
    }

    /// 규칙을 추가합니다 (우선순위 순으로 자동 정렬)
    pub fn add_rule(&mut self, rule: TlsConfigRule) {
        self.rules.push(rule);
        self.rules.sort_by_key(|r| r.priority);
    }

    /// 규칙을 제거합니다 (도메인 패턴으로 검색)
    pub fn remove_rule(&mut self, domain_pattern: &str) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.domain_pattern != domain_pattern);
        self.rules.len() < before
    }

    /// 모든 규칙을 반환합니다
    pub fn rules(&self) -> &[TlsConfigRule] {
        &self.rules
    }

    /// 도메인에 매칭되는 설정을 조회합니다
    pub fn resolve(&self, host: &str) -> ResolvedTlsConfig {
        for rule in &self.rules {
            if matches_domain_pattern(&rule.domain_pattern, host) {
                debug!(
                    "[TLS-CONFIG] 도메인 '{}' → 규칙 '{}' 매칭 (priority={})",
                    host, rule.domain_pattern, rule.priority
                );
                return ResolvedTlsConfig {
                    client_config: rule.client_config.clone(),
                    server_config: rule.server_config.clone(),
                    require_openssl: rule.require_openssl,
                    handshake_timeout_secs: rule.handshake_timeout_secs,
                    disable_cert_verify: rule.disable_cert_verify,
                    matched_pattern: Some(rule.domain_pattern.clone()),
                };
            }
        }

        // 매칭 규칙 없음 → 기본값 사용
        ResolvedTlsConfig {
            client_config: self.default_client.clone(),
            server_config: self.default_server.clone(),
            require_openssl: false,
            handshake_timeout_secs: None,
            disable_cert_verify: false,
            matched_pattern: None,
        }
    }

    /// 도메인이 OpenSSL을 필요로 하는지 확인
    ///
    /// `resolve()`에 위임하여 우선순위 기반 매칭과 일관된 결과를 보장합니다.
    pub fn requires_openssl(&self, host: &str) -> bool {
        self.resolve(host).require_openssl
    }

    /// 도메인의 핸드셰이크 타임아웃을 조회합니다
    ///
    /// `resolve()`에 위임하여 우선순위 기반 매칭과 일관된 결과를 보장합니다.
    pub fn handshake_timeout(&self, host: &str) -> Option<u64> {
        self.resolve(host).handshake_timeout_secs
    }
}

/// 도메인 패턴 매칭
///
/// - `"*.apple.com"` → `apple.com` 자체 및 `sub.apple.com` 등 매칭
/// - `"api2.cursor.sh"` → 정확히 일치만 매칭
fn matches_domain_pattern(pattern: &str, host: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("*.") {
        // 와일드카드: host가 suffix와 같거나 .suffix로 끝남
        // format! 대신 바이트 경계 비교로 힙 할당 회피
        host == suffix
            || (host.len() > suffix.len()
                && host.ends_with(suffix)
                && host.as_bytes()[host.len() - suffix.len() - 1] == b'.')
    } else {
        // 정확한 매칭
        host == pattern
    }
}

/// `Arc`로 감싼 `TlsConfigManager` 타입 alias
pub type SharedTlsConfig = Arc<TlsConfigManager>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_domain_pattern_exact() {
        assert!(matches_domain_pattern("api2.cursor.sh", "api2.cursor.sh"));
        assert!(!matches_domain_pattern("api2.cursor.sh", "other.cursor.sh"));
        assert!(!matches_domain_pattern("api2.cursor.sh", "cursor.sh"));
    }

    #[test]
    fn test_matches_domain_pattern_wildcard() {
        assert!(matches_domain_pattern("*.apple.com", "wps.apple.com"));
        assert!(matches_domain_pattern("*.apple.com", "fbs.smoot.apple.com"));
        assert!(matches_domain_pattern("*.apple.com", "apple.com"));
        assert!(!matches_domain_pattern("*.apple.com", "notapple.com"));
        assert!(!matches_domain_pattern("*.apple.com", "fakeapple.com"));
    }

    #[test]
    fn test_matches_domain_pattern_icloud() {
        assert!(matches_domain_pattern("*.icloud.com", "gateway.icloud.com"));
        assert!(matches_domain_pattern("*.icloud.com", "icloud.com"));
        assert!(!matches_domain_pattern("*.icloud.com", "fakeicloud.com"));
    }

    #[test]
    fn test_builtin_rules_apple_resolve() {
        let manager = TlsConfigManager::with_builtin_rules();

        // 정확 매칭 규칙이 없는 Apple 도메인 → 와일드카드 규칙 매칭 (cipher + 15초 타임아웃)
        let resolved = manager.resolve("maps.apple.com");
        assert!(resolved.client_config.cipher_list.is_some());
        assert_eq!(resolved.handshake_timeout_secs, Some(15));
        assert!(resolved.disable_cert_verify);

        // Apple 도메인 중 OpenSSL 필수 (정확 매칭 우선)
        assert!(manager.requires_openssl("wps.apple.com"));
        assert!(manager.requires_openssl("gateway.icloud.com"));
    }

    #[test]
    fn test_builtin_rules_openssl_required() {
        let manager = TlsConfigManager::with_builtin_rules();

        assert!(manager.requires_openssl("api2.cursor.sh"));
        assert!(!manager.requires_openssl("jamf.payhere.in"));
        assert!(!manager.requires_openssl("example.com"));
    }

    #[test]
    fn test_builtin_rules_priority() {
        let manager = TlsConfigManager::with_builtin_rules();

        // wps.apple.com은 OpenSSL 필수 (priority=10)가 Apple cipher (priority=50)보다 우선
        let resolved = manager.resolve("wps.apple.com");
        assert!(resolved.require_openssl);
        assert_eq!(resolved.matched_pattern.as_deref(), Some("wps.apple.com"));
        // 정확 매칭 규칙에는 cipher_list가 None
        assert!(resolved.client_config.cipher_list.is_none());
    }

    #[test]
    fn test_requires_openssl_consistent_with_resolve() {
        let manager = TlsConfigManager::with_builtin_rules();

        // requires_openssl()과 resolve().require_openssl이 항상 일치하는지 검증
        let test_domains = [
            "wps.apple.com",
            "maps.apple.com",
            "gateway.icloud.com",
            "api2.cursor.sh",
            "example.com",
        ];
        for domain in test_domains {
            assert_eq!(
                manager.requires_openssl(domain),
                manager.resolve(domain).require_openssl,
                "불일치: {}",
                domain
            );
        }
    }

    #[test]
    fn test_default_fallback() {
        let manager = TlsConfigManager::with_builtin_rules();

        let resolved = manager.resolve("example.com");
        assert!(resolved.matched_pattern.is_none());
        assert!(!resolved.require_openssl);
        assert!(resolved.client_config.cipher_list.is_none());
        assert_eq!(resolved.client_config.version_min, TlsVersionConfig::Tls12);
    }

    #[test]
    fn test_custom_rule_add_remove() {
        let mut manager = TlsConfigManager::new();
        assert!(manager.rules().is_empty());

        manager.add_rule(TlsConfigRule {
            domain_pattern: "*.internal.corp".to_string(),
            client_config: DirectionalTlsConfig::legacy(),
            server_config: DirectionalTlsConfig::default(),
            require_openssl: true,
            handshake_timeout_secs: Some(30),
            disable_cert_verify: false,
            priority: 20,
        });

        assert_eq!(manager.rules().len(), 1);
        assert!(manager.requires_openssl("app.internal.corp"));

        let resolved = manager.resolve("app.internal.corp");
        assert_eq!(resolved.client_config.version_min, TlsVersionConfig::Tls10);
        assert_eq!(resolved.handshake_timeout_secs, Some(30));

        assert!(manager.remove_rule("*.internal.corp"));
        assert!(manager.rules().is_empty());
    }

    #[test]
    fn test_rule_priority_ordering() {
        let mut manager = TlsConfigManager::new();

        // 우선순위가 높은 규칙이 먼저 매칭
        manager.add_rule(TlsConfigRule {
            domain_pattern: "*.example.com".to_string(),
            client_config: DirectionalTlsConfig::default(),
            server_config: DirectionalTlsConfig::default(),
            require_openssl: false,
            handshake_timeout_secs: Some(20),
            disable_cert_verify: false,
            priority: 100,
        });

        manager.add_rule(TlsConfigRule {
            domain_pattern: "api.example.com".to_string(),
            client_config: DirectionalTlsConfig::default(),
            server_config: DirectionalTlsConfig::default(),
            require_openssl: true,
            handshake_timeout_secs: Some(5),
            disable_cert_verify: false,
            priority: 10,
        });

        let resolved = manager.resolve("api.example.com");
        assert!(resolved.require_openssl);
        assert_eq!(resolved.handshake_timeout_secs, Some(5));
    }

    #[test]
    fn test_tls_version_config_ordering() {
        assert!(TlsVersionConfig::Tls10 < TlsVersionConfig::Tls11);
        assert!(TlsVersionConfig::Tls11 < TlsVersionConfig::Tls12);
        assert!(TlsVersionConfig::Tls12 < TlsVersionConfig::Tls13);
    }

    #[test]
    fn test_tls_version_rustls_support() {
        assert!(!TlsVersionConfig::Tls10.is_rustls_supported());
        assert!(!TlsVersionConfig::Tls11.is_rustls_supported());
        assert!(TlsVersionConfig::Tls12.is_rustls_supported());
        assert!(TlsVersionConfig::Tls13.is_rustls_supported());
    }

    #[test]
    fn test_serde_roundtrip() {
        let manager = TlsConfigManager::with_builtin_rules();
        let json = serde_json::to_string(&manager).unwrap();
        let deserialized: TlsConfigManager = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.rules().len(), manager.rules().len());
    }
}
