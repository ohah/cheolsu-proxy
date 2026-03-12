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

pub(crate) const TTL_SECS: i64 = 365 * 24 * 60 * 60;
pub(crate) const CACHE_TTL: u64 = TTL_SECS as u64 / 2;
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
