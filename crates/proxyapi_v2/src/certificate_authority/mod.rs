mod generator;
#[cfg(feature = "openssl-ca")]
mod openssl_authority;
#[cfg(feature = "rcgen-ca")]
mod rcgen_authority;
mod storage;

use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use std::sync::Arc;
use tokio_rustls::rustls::ServerConfig;

#[cfg(feature = "openssl-ca")]
use openssl::ssl::SslContext;

#[cfg(feature = "openssl-ca")]
pub use openssl_authority::*;
#[cfg(feature = "rcgen-ca")]
pub use rcgen_authority::*;

pub use generator::*;
pub use storage::*;

/// 인증서 관련 이벤트
#[derive(Debug, Clone, PartialEq)]
pub enum CertEvent {
    /// CA 인증서가 새로 생성됨
    CaGenerated,
    /// CA 인증서가 만료 임박으로 재생성됨 (남은 일수)
    CaRegeneratedExpiringSoon(i64),
    /// CA 인증서가 만료되어 재생성됨
    CaRegeneratedExpired,
    /// CA 인증서 로드 실패로 재생성됨 (에러 메시지)
    CaRegeneratedCorrupted(String),
    /// CA 인증서가 정상 로드됨
    CaLoaded,
}

/// CA 인증서 유효기간 (10년)
/// mitmproxy와 동일하게 장기간 설정하여 사용자가 자주 재설치하지 않아도 됨
pub(crate) const CA_TTL_SECS: i64 = 10 * 365 * 24 * 60 * 60;
/// Leaf 인증서 유효기간 (90일)
/// Apple/Chrome은 398일 이상 인증서를 거부하고, 업계 트렌드는 90일로 수렴 중.
/// 짧은 유효기간은 키 노출 시 위험 노출 기간을 줄이고 캐시 갱신 주기를 단축합니다.
pub(crate) const LEAF_TTL_SECS: i64 = 90 * 24 * 60 * 60;
pub(crate) const CACHE_TTL: u64 = LEAF_TTL_SECS as u64 / 2;
/// 인증서 not_before 오프셋 (2일 = 172800초)
/// 클라이언트 시계 오차를 대비하여 mitmproxy와 동일하게 -2일로 설정
pub(crate) const NOT_BEFORE_OFFSET: i64 = 172_800;
/// 인증서 생성 타임아웃 (초)
/// RSA 키 생성이 시스템 부하로 비정상적으로 오래 걸릴 때를 대비
pub(crate) const CERT_GEN_TIMEOUT_SECS: u64 = 10;

/// 인증서 생성 메트릭 수집기
///
/// Atomic 카운터 기반으로 lock-free하게 인증서 생성 통계를 수집합니다.
#[derive(Debug)]
pub struct CertMetrics {
    /// 인증서 생성 횟수
    pub gen_count: std::sync::atomic::AtomicU64,
    /// 캐시 히트 횟수
    pub cache_hit_count: std::sync::atomic::AtomicU64,
    /// 인증서 생성 실패 횟수
    pub gen_failure_count: std::sync::atomic::AtomicU64,
    /// 인증서 생성 총 소요시간 (마이크로초)
    pub total_gen_duration_us: std::sync::atomic::AtomicU64,
}

impl CertMetrics {
    pub fn new() -> Self {
        use std::sync::atomic::AtomicU64;
        Self {
            gen_count: AtomicU64::new(0),
            cache_hit_count: AtomicU64::new(0),
            gen_failure_count: AtomicU64::new(0),
            total_gen_duration_us: AtomicU64::new(0),
        }
    }

    pub fn record_gen(&self, duration: std::time::Duration) {
        use std::sync::atomic::Ordering::Relaxed;
        self.gen_count.fetch_add(1, Relaxed);
        self.total_gen_duration_us
            .fetch_add(duration.as_micros() as u64, Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hit_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.gen_failure_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 평균 생성 시간 (마이크로초). 생성 횟수가 0이면 0 반환.
    pub fn avg_gen_duration_us(&self) -> u64 {
        use std::sync::atomic::Ordering::Relaxed;
        let count = self.gen_count.load(Relaxed);
        if count == 0 {
            return 0;
        }
        self.total_gen_duration_us.load(Relaxed) / count
    }

    /// 캐시 히트율 (0.0~1.0). 요청이 없으면 0.0 반환.
    pub fn cache_hit_rate(&self) -> f64 {
        use std::sync::atomic::Ordering::Relaxed;
        let hits = self.cache_hit_count.load(Relaxed);
        let gens = self.gen_count.load(Relaxed);
        let total = hits + gens;
        if total == 0 {
            return 0.0;
        }
        hits as f64 / total as f64
    }

    pub fn snapshot(&self) -> CertMetricsSnapshot {
        use std::sync::atomic::Ordering::Relaxed;
        CertMetricsSnapshot {
            gen_count: self.gen_count.load(Relaxed),
            cache_hit_count: self.cache_hit_count.load(Relaxed),
            gen_failure_count: self.gen_failure_count.load(Relaxed),
            avg_gen_duration_us: self.avg_gen_duration_us(),
            cache_hit_rate: self.cache_hit_rate(),
        }
    }
}

impl Default for CertMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// 메트릭 스냅샷 (직렬화/로깅용)
#[derive(Debug, Clone, serde::Serialize)]
pub struct CertMetricsSnapshot {
    pub gen_count: u64,
    pub cache_hit_count: u64,
    pub gen_failure_count: u64,
    pub avg_gen_duration_us: u64,
    pub cache_hit_rate: f64,
}

/// 호스트에서 와일드카드 Authority를 추출합니다.
/// `api.example.com:443` → `*.example.com:443`
/// IP 주소나 TLD는 None을 반환합니다.
// 현재 프로덕션 경로에서는 사용하지 않지만(와일드카드 캐시 공유가 SAN 불일치를 유발해 제거됨),
// 재사용 가능성을 위해 검증된 유틸리티로 보존한다.
#[allow(dead_code)]
pub(crate) fn wildcard_authority(authority: &Authority) -> Option<Authority> {
    let host = authority.host();

    // IP 주소는 와일드카드 불가
    if host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }

    // 이미 와일드카드이면 그대로
    if host.starts_with("*.") {
        return Some(authority.clone());
    }

    // sub.example.com → *.example.com (최소 서브도메인이 있어야 함)
    let dot_pos = host.find('.')?;
    let root = &host[dot_pos..]; // .example.com

    // root에 추가 도트가 없으면 TLD만 남은 것 (예: .com → 불가)
    if !root[1..].contains('.') {
        return None;
    }

    let wildcard_host = format!("*{}", root);
    let wildcard_str = if let Some(port) = authority.port() {
        format!("{}:{}", wildcard_host, port)
    } else {
        wildcard_host
    };

    Authority::try_from(wildcard_str).ok()
}

/// upstream 서버의 ALPN 협상 결과를 기반으로 ALPN 프로토콜 리스트를 구성합니다.
/// 협상된 프로토콜을 우선 배치하고, 나머지 지원 프로토콜을 폴백으로 추가합니다.
pub(crate) fn build_alpn_protocols(upstream_cert: Option<&UpstreamCertInfo>) -> Vec<Vec<u8>> {
    let default_protocols = || {
        vec![
            #[cfg(feature = "http2")]
            b"h2".to_vec(),
            b"http/1.1".to_vec(),
        ]
    };

    let Some(upstream) = upstream_cert else {
        return default_protocols();
    };
    let Some(ref negotiated) = upstream.negotiated_alpn else {
        return default_protocols();
    };

    let mut protocols = vec![negotiated.clone()];
    #[cfg(feature = "http2")]
    if negotiated != b"h2" {
        protocols.push(b"h2".to_vec());
    }
    if negotiated != b"http/1.1" {
        protocols.push(b"http/1.1".to_vec());
    }
    protocols
}

/// 128비트 난수 시리얼 번호를 생성합니다.
/// RFC 5280: serial number는 양수 INTEGER이므로 최상위 비트를 클리어합니다.
#[cfg(feature = "rcgen-ca")]
pub(crate) fn generate_serial_number() -> rcgen::SerialNumber {
    use rand::Rng;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    bytes[0] &= 0x7F;
    rcgen::SerialNumber::from_slice(&bytes)
}

/// CN(Common Name)을 RFC 5280 제한인 64자로 truncate합니다.
/// char 경계를 존중하여 안전하게 자릅니다.
pub(crate) fn truncate_cn(cn: &str) -> String {
    if cn.chars().count() <= 64 {
        cn.to_string()
    } else {
        cn.chars().take(64).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_cn;

    #[test]
    fn truncate_cn_short_ascii() {
        assert_eq!(truncate_cn("example.com"), "example.com");
    }

    #[test]
    fn truncate_cn_exactly_64() {
        let cn: String = "a".repeat(64);
        assert_eq!(truncate_cn(&cn), cn);
    }

    #[test]
    fn truncate_cn_over_64() {
        let cn: String = "a".repeat(100);
        assert_eq!(truncate_cn(&cn).chars().count(), 64);
    }

    #[test]
    fn truncate_cn_empty() {
        assert_eq!(truncate_cn(""), "");
    }

    #[test]
    fn truncate_cn_multibyte_under_limit() {
        let cn: String = "가".repeat(20);
        assert_eq!(truncate_cn(&cn), cn);
    }

    #[test]
    fn truncate_cn_multibyte_over_limit() {
        let cn: String = "가".repeat(70);
        let result = truncate_cn(&cn);
        assert_eq!(result.chars().count(), 64);
    }

    // --- wildcard_authority 테스트 ---

    use super::wildcard_authority;
    use http::uri::Authority;

    #[test]
    fn wildcard_subdomain() {
        let auth = Authority::from_static("api.example.com");
        let wc = wildcard_authority(&auth).unwrap();
        assert_eq!(wc.host(), "*.example.com");
    }

    #[test]
    fn wildcard_deep_subdomain() {
        let auth = Authority::from_static("a.b.example.com");
        let wc = wildcard_authority(&auth).unwrap();
        assert_eq!(wc.host(), "*.b.example.com");
    }

    #[test]
    fn wildcard_preserves_port() {
        let auth = Authority::from_static("api.example.com:8443");
        let wc = wildcard_authority(&auth).unwrap();
        assert_eq!(wc.to_string(), "*.example.com:8443");
    }

    #[test]
    fn wildcard_root_domain_returns_none() {
        let auth = Authority::from_static("example.com");
        assert!(wildcard_authority(&auth).is_none());
    }

    #[test]
    fn wildcard_ip_returns_none() {
        let auth = Authority::from_static("192.168.1.1:443");
        assert!(wildcard_authority(&auth).is_none());
    }

    #[test]
    fn wildcard_already_wildcard() {
        let auth = Authority::from_static("*.example.com");
        let wc = wildcard_authority(&auth).unwrap();
        assert_eq!(wc.host(), "*.example.com");
    }

    #[test]
    fn wildcard_localhost_returns_none() {
        let auth = Authority::from_static("localhost");
        assert!(wildcard_authority(&auth).is_none());
    }

    // --- build_alpn_protocols 테스트 ---

    use super::build_alpn_protocols;
    use crate::upstream_cert::UpstreamCertInfo;

    #[test]
    fn alpn_no_upstream() {
        let protocols = build_alpn_protocols(None);
        assert!(protocols.contains(&b"http/1.1".to_vec()));
    }

    #[test]
    fn alpn_with_negotiated_h2() {
        let upstream = UpstreamCertInfo {
            negotiated_alpn: Some(b"h2".to_vec()),
            ..Default::default()
        };
        let protocols = build_alpn_protocols(Some(&upstream));
        assert_eq!(protocols[0], b"h2");
        assert!(protocols.contains(&b"http/1.1".to_vec()));
    }

    #[test]
    fn alpn_with_negotiated_http11() {
        let upstream = UpstreamCertInfo {
            negotiated_alpn: Some(b"http/1.1".to_vec()),
            ..Default::default()
        };
        let protocols = build_alpn_protocols(Some(&upstream));
        assert_eq!(protocols[0], b"http/1.1");
        // http/1.1이 이미 있으므로 중복 추가 안 됨
        assert_eq!(
            protocols
                .iter()
                .filter(|p| *p == &b"http/1.1".to_vec())
                .count(),
            1
        );
    }

    #[test]
    fn alpn_no_negotiated() {
        let upstream = UpstreamCertInfo {
            negotiated_alpn: None,
            ..Default::default()
        };
        let protocols = build_alpn_protocols(Some(&upstream));
        assert!(protocols.contains(&b"http/1.1".to_vec()));
    }

    // --- CertMetrics 테스트 ---

    use super::CertMetrics;

    #[test]
    fn metrics_initial_state() {
        let m = CertMetrics::new();
        assert_eq!(m.avg_gen_duration_us(), 0);
        assert_eq!(m.cache_hit_rate(), 0.0);
        let snap = m.snapshot();
        assert_eq!(snap.gen_count, 0);
        assert_eq!(snap.cache_hit_count, 0);
        assert_eq!(snap.gen_failure_count, 0);
    }

    #[test]
    fn metrics_record_gen() {
        let m = CertMetrics::new();
        m.record_gen(std::time::Duration::from_millis(100));
        m.record_gen(std::time::Duration::from_millis(200));
        let snap = m.snapshot();
        assert_eq!(snap.gen_count, 2);
        assert!(snap.avg_gen_duration_us >= 100_000); // 최소 100ms
    }

    #[test]
    fn metrics_cache_hit_rate() {
        let m = CertMetrics::new();
        m.record_cache_hit();
        m.record_cache_hit();
        m.record_cache_hit();
        m.record_gen(std::time::Duration::from_millis(1));
        // 3 hits / (3 hits + 1 gen) = 0.75
        assert!((m.cache_hit_rate() - 0.75).abs() < 0.01);
    }

    #[test]
    fn metrics_record_failure() {
        let m = CertMetrics::new();
        m.record_failure();
        m.record_failure();
        assert_eq!(m.snapshot().gen_failure_count, 2);
    }

    #[test]
    fn metrics_snapshot_serializable() {
        let m = CertMetrics::new();
        m.record_gen(std::time::Duration::from_millis(50));
        let snap = m.snapshot();
        let json = serde_json::to_string(&snap);
        assert!(json.is_ok(), "CertMetricsSnapshot은 직렬화 가능해야 함");
    }
}

/// upstream 키 타입을 정규화합니다 (캐시 키 용).
/// RSA 비트 수를 2048/4096으로 정규화하고, Unknown은 ECDSA P-256으로 변환합니다.
fn normalize_key_type(
    upstream_cert: Option<&UpstreamCertInfo>,
) -> crate::upstream_cert::UpstreamKeyType {
    use crate::upstream_cert::{EcCurve, UpstreamKeyType};

    match upstream_cert.map(|u| &u.key_type) {
        Some(UpstreamKeyType::Rsa(bits)) => {
            let bits = match *bits {
                b if b >= 4096 => 4096,
                _ => 2048,
            };
            UpstreamKeyType::Rsa(bits)
        }
        Some(UpstreamKeyType::Ecdsa(EcCurve::P521)) => {
            // rcgen이 P-521 미지원이므로 P-384로 통일
            UpstreamKeyType::Ecdsa(EcCurve::P384)
        }
        Some(UpstreamKeyType::Ecdsa(curve)) => UpstreamKeyType::Ecdsa(curve.clone()),
        Some(UpstreamKeyType::Ed25519) => UpstreamKeyType::Ed25519,
        Some(UpstreamKeyType::Unknown) | None => UpstreamKeyType::Ecdsa(EcCurve::P256),
    }
}

/// 키 타입별로 캐시된 OpenSSL leaf 키를 반환합니다.
/// 동일 키 타입이면 이전에 생성한 키를 재사용하여 RSA 키 생성 비용(~100ms)을 회피합니다.
#[cfg(feature = "openssl-ca")]
pub(crate) fn generate_openssl_leaf_pkey(
    upstream_cert: Option<&UpstreamCertInfo>,
) -> Result<openssl::pkey::PKey<openssl::pkey::Private>, openssl::error::ErrorStack> {
    use crate::upstream_cert::UpstreamKeyType;
    use std::collections::HashMap;
    use std::sync::Mutex;

    static CACHE: std::sync::LazyLock<
        Mutex<HashMap<UpstreamKeyType, openssl::pkey::PKey<openssl::pkey::Private>>>,
    > = std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

    let normalized = normalize_key_type(upstream_cert);

    if let Ok(cache) = CACHE.lock() {
        if let Some(cached) = cache.get(&normalized) {
            return Ok(cached.clone());
        }
    }

    let pkey = generate_openssl_leaf_pkey_uncached(&normalized)?;

    if let Ok(mut cache) = CACHE.lock() {
        cache.entry(normalized).or_insert_with(|| pkey.clone());
    }

    Ok(pkey)
}

#[cfg(feature = "openssl-ca")]
fn generate_openssl_leaf_pkey_uncached(
    key_type: &crate::upstream_cert::UpstreamKeyType,
) -> Result<openssl::pkey::PKey<openssl::pkey::Private>, openssl::error::ErrorStack> {
    use crate::upstream_cert::{EcCurve, UpstreamKeyType};
    use openssl::{
        ec::{EcGroup, EcKey},
        nid::Nid,
        pkey::PKey,
    };

    match key_type {
        UpstreamKeyType::Rsa(bits) => {
            let rsa = openssl::rsa::Rsa::generate(*bits)?;
            PKey::from_rsa(rsa)
        }
        UpstreamKeyType::Ecdsa(curve) => {
            let nid = match curve {
                EcCurve::P256 => Nid::X9_62_PRIME256V1,
                EcCurve::P384 => Nid::SECP384R1,
                EcCurve::P521 => Nid::SECP521R1,
            };
            let group = EcGroup::from_curve_name(nid)?;
            let ec_key = EcKey::generate(&group)?;
            PKey::from_ec_key(ec_key)
        }
        UpstreamKeyType::Ed25519 => PKey::generate_ed25519(),
        // normalize_key_type이 Unknown을 Ecdsa(P256)으로 변환하므로 여기 도달하지 않음
        UpstreamKeyType::Unknown => unreachable!("normalize_key_type handles Unknown"),
    }
}

/// 키 타입별로 캐시된 rcgen leaf 키를 반환합니다.
#[cfg(feature = "rcgen-ca")]
pub(crate) fn generate_rcgen_leaf_key_pair(
    upstream_cert: Option<&UpstreamCertInfo>,
) -> Result<rcgen::KeyPair, rcgen::Error> {
    use crate::upstream_cert::UpstreamKeyType;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // rcgen::KeyPair는 Clone을 구현하지 않으므로 DER 바이트를 캐시
    static CACHE: std::sync::LazyLock<
        Mutex<HashMap<UpstreamKeyType, (Vec<u8>, &'static rcgen::SignatureAlgorithm)>>,
    > = std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

    let normalized = normalize_key_type(upstream_cert);

    // 캐시 히트 — DER에서 KeyPair 복원
    if let Ok(cache) = CACHE.lock() {
        if let Some((der_bytes, alg)) = cache.get(&normalized) {
            use tokio_rustls::rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
            let pk_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(der_bytes.clone()));
            if let Ok(kp) = rcgen::KeyPair::from_der_and_sign_algo(&pk_der, alg) {
                return Ok(kp);
            }
        }
    }

    // 캐시 미스 — 새 키 생성
    let (kp, alg) = generate_rcgen_leaf_key_pair_uncached(&normalized)?;
    let der_bytes = kp.serialize_der();

    // 캐시에 저장
    if let Ok(mut cache) = CACHE.lock() {
        cache.entry(normalized).or_insert((der_bytes.clone(), alg));
    }

    Ok(kp)
}

#[cfg(feature = "rcgen-ca")]
fn generate_rcgen_leaf_key_pair_uncached(
    key_type: &crate::upstream_cert::UpstreamKeyType,
) -> Result<(rcgen::KeyPair, &'static rcgen::SignatureAlgorithm), rcgen::Error> {
    use crate::upstream_cert::{EcCurve, UpstreamKeyType};

    let alg: &'static rcgen::SignatureAlgorithm = match key_type {
        UpstreamKeyType::Rsa(_) => &rcgen::PKCS_RSA_SHA256,
        UpstreamKeyType::Ecdsa(EcCurve::P384) => &rcgen::PKCS_ECDSA_P384_SHA384,
        UpstreamKeyType::Ed25519 => &rcgen::PKCS_ED25519,
        UpstreamKeyType::Ecdsa(EcCurve::P256) => &rcgen::PKCS_ECDSA_P256_SHA256,
        // normalize_key_type이 P521→P384, Unknown→P256으로 변환하므로 여기 도달하지 않음
        UpstreamKeyType::Ecdsa(EcCurve::P521) | UpstreamKeyType::Unknown => {
            unreachable!("normalize_key_type handles P521 and Unknown")
        }
    };

    let kp = rcgen::KeyPair::generate_for(alg)?;
    Ok((kp, alg))
}

/// Issues certificates for use when communicating with clients.
///
/// Clients should be configured to either trust the provided root certificate, or to ignore
/// certificate errors.
pub trait CertificateAuthority: Send + Sync + 'static {
    /// Generate ServerConfig for use with rustls.
    ///
    /// `upstream_cert`가 Some이면 상류 서버 인증서의 CN, SAN, Organization을 복제합니다.
    fn gen_server_config(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> impl Future<Output = Result<Arc<ServerConfig>, Box<dyn std::error::Error + Send + Sync>>> + Send;

    /// Get the CA certificate in DER format for adding to client trust store.
    /// Returns None if the CA certificate is not available in DER format.
    fn get_ca_cert_der(&self) -> Option<Vec<u8>>;

    /// 주어진 authority에 대한 인증서가 캐시에 있는지 확인합니다.
    /// 캐시 히트 시 불필요한 upstream cert sniffing을 건너뛸 수 있습니다.
    fn is_config_cached(&self, authority: &Authority) -> impl Future<Output = bool> + Send;

    #[cfg(feature = "openssl-ca")]
    /// Generate OpenSSL SslContext for use with openssl.
    ///
    /// `upstream_cert`가 Some이면 상류 서버 인증서의 CN, SAN, Organization을 복제합니다.
    fn gen_openssl_context(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> impl Future<Output = Result<SslContext, Box<dyn std::error::Error + Send + Sync>>> + Send;
}
