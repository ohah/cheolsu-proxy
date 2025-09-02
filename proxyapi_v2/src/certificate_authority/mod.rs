#[cfg(feature = "openssl-ca")]
mod openssl_authority;
#[cfg(feature = "rcgen-ca")]
mod rcgen_authority;

use http::uri::Authority;
use std::sync::Arc;
use tokio_rustls::rustls::ServerConfig;

#[cfg(feature = "openssl-ca")]
pub use openssl_authority::*;
#[cfg(feature = "rcgen-ca")]
pub use rcgen_authority::*;

const TTL_SECS: i64 = 365 * 24 * 60 * 60;
const CACHE_TTL: u64 = TTL_SECS as u64 / 2;
const NOT_BEFORE_OFFSET: i64 = 60;

/// 기존 인증서 파일을 사용하여 RcgenAuthority 생성
#[cfg(feature = "rcgen-ca")]
pub fn build_ca() -> Result<RcgenAuthority, String> {
    // 기존 인증서 파일에서 키 페어와 CA 인증서 로드
    let private_key_bytes: &[u8] = include_bytes!("proxelar.key");
    let ca_cert_bytes: &[u8] = include_bytes!("proxelar.cer");

    // PEM 형식의 키 페어 파싱
    let key_pair = rcgen::KeyPair::from_pem(
        std::str::from_utf8(private_key_bytes)
            .map_err(|e| format!("키 파일 인코딩 오류: {}", e))?,
    )
    .map_err(|e| format!("키 페어 파싱 실패: {}", e))?;

    // PEM 형식의 CA 인증서 파싱
    let ca_cert_params = rcgen::CertificateParams::from_ca_cert_pem(
        std::str::from_utf8(ca_cert_bytes)
            .map_err(|e| format!("인증서 파일 인코딩 오류: {}", e))?,
    )
    .map_err(|e| format!("CA 인증서 파싱 실패: {}", e))?;

    // CertificateParams를 Certificate로 변환
    let ca_cert = ca_cert_params
        .self_signed(&key_pair)
        .map_err(|e| format!("CA 인증서 서명 실패: {}", e))?;

    // RcgenAuthority 생성
    let ca = RcgenAuthority::new(
        key_pair,
        ca_cert,
        1_000,
        tokio_rustls::rustls::crypto::aws_lc_rs::default_provider(),
    );

    Ok(ca)
}

/// Issues certificates for use when communicating with clients.
///
/// Clients should be configured to either trust the provided root certificate, or to ignore
/// certificate errors.
pub trait CertificateAuthority: Send + Sync + 'static {
    /// Generate ServerConfig for use with rustls.
    fn gen_server_config(
        &self,
        authority: &Authority,
    ) -> impl Future<Output = Arc<ServerConfig>> + Send;
}
