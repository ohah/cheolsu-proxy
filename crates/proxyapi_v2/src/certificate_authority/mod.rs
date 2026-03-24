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

/// CA 인증서 유효기간 (10년)
/// mitmproxy와 동일하게 장기간 설정하여 사용자가 자주 재설치하지 않아도 됨
pub(crate) const CA_TTL_SECS: i64 = 10 * 365 * 24 * 60 * 60;
/// Leaf 인증서 유효기간 (1년)
pub(crate) const LEAF_TTL_SECS: i64 = 365 * 24 * 60 * 60;
pub(crate) const CACHE_TTL: u64 = LEAF_TTL_SECS as u64 / 2;
/// 인증서 not_before 오프셋 (2일 = 172800초)
/// 클라이언트 시계 오차를 대비하여 mitmproxy와 동일하게 -2일로 설정
pub(crate) const NOT_BEFORE_OFFSET: i64 = 172_800;

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
        // 한글 20자 = 60바이트이지만 문자 수는 20
        let cn: String = "가".repeat(20);
        assert_eq!(truncate_cn(&cn), cn);
    }

    #[test]
    fn truncate_cn_multibyte_over_limit() {
        // 한글 70자 → 64자로 truncate
        let cn: String = "가".repeat(70);
        let result = truncate_cn(&cn);
        assert_eq!(result.chars().count(), 64);
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
