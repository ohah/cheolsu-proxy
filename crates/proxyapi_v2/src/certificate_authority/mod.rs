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
    if cn.len() <= 64 {
        cn.to_string()
    } else {
        cn.chars().take(64).collect()
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
    ) -> impl Future<Output = Arc<ServerConfig>> + Send;

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
