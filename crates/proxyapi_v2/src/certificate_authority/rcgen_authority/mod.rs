mod cert_gen;
#[cfg(feature = "openssl-ca")]
mod openssl_context;
#[cfg(test)]
mod tests;
mod trait_impl;

use crate::certificate_authority::CACHE_TTL;
use http::uri::Authority;
use moka::future::Cache;
use rcgen::{Issuer, KeyPair};
use std::sync::Arc;
use tokio_rustls::rustls::pki_types::CertificateDer;
use tokio_rustls::rustls::{ServerConfig, crypto::CryptoProvider};
use tracing::info;

/// 클라이언트 인증서 요청 설정 (프록시 → 클라이언트)
#[derive(Debug, Clone)]
pub struct ClientCertVerifyConfig {
    /// 활성화 여부
    pub enabled: bool,
    /// 클라이언트 인증서 검증용 CA 인증서 (DER 형식)
    pub ca_certs: Vec<tokio_rustls::rustls::pki_types::CertificateDer<'static>>,
    /// 인증서 필수 여부
    pub required: bool,
}

/// Issues certificates for use when communicating with clients.
///
/// Issues certificates for communicating with clients over TLS. Certificates are cached in memory
/// up to a max size that is provided when creating the authority. Certificates are generated using
/// the `rcgen` crate.
///
/// # Examples
///
/// ```rust
/// use proxyapi_v2::{certificate_authority::RcgenAuthority, rustls::crypto::aws_lc_rs};
///
/// let key_pem = include_str!("../../../examples/ca/hudsucker.key");
/// let ca_pem = include_str!("../../../examples/ca/hudsucker.cer");
///
/// let ca = RcgenAuthority::from_pem(ca_pem, key_pem, 1_000, aws_lc_rs::default_provider())
///     .expect("Failed to create RcgenAuthority from PEM");
/// ```
pub struct RcgenAuthority {
    pub(super) issuer: Issuer<'static, KeyPair>,
    pub(super) ca_cert_der: CertificateDer<'static>,
    /// openssl_context에서 CA 인증서/키를 PEM으로 사용하기 위한 캐시
    pub(super) ca_cert_pem: String,
    pub(super) ca_key_pem: String,
    pub(super) cache: Cache<Authority, Arc<ServerConfig>>,
    #[cfg(feature = "openssl-ca")]
    pub(super) openssl_ctx_cache: Cache<Authority, Arc<openssl::ssl::SslContext>>,
    pub(super) provider: Arc<CryptoProvider>,
    /// 클라이언트 인증서 요청 설정
    pub(super) client_cert_verify: Arc<tokio::sync::RwLock<Option<ClientCertVerifyConfig>>>,
    /// 인증서 생성 메트릭
    pub(super) metrics: Arc<crate::certificate_authority::CertMetrics>,
}

impl RcgenAuthority {
    /// Creates a new rcgen authority.
    pub fn new(
        issuer: Issuer<'static, KeyPair>,
        ca_cert_der: CertificateDer<'static>,
        ca_cert_pem: String,
        ca_key_pem: String,
        cache_size: u64,
        provider: CryptoProvider,
    ) -> Self {
        Self {
            issuer,
            ca_cert_der,
            ca_cert_pem,
            ca_key_pem,
            cache: Cache::builder()
                .max_capacity(cache_size)
                .time_to_live(std::time::Duration::from_secs(CACHE_TTL))
                .build(),
            #[cfg(feature = "openssl-ca")]
            openssl_ctx_cache: Cache::builder()
                .max_capacity(cache_size)
                .time_to_live(std::time::Duration::from_secs(CACHE_TTL))
                .build(),
            provider: Arc::new(provider),
            client_cert_verify: Arc::new(tokio::sync::RwLock::new(None)),
            metrics: Arc::new(crate::certificate_authority::CertMetrics::new()),
        }
    }

    /// 인증서 메트릭 핸들을 반환합니다.
    pub fn get_metrics(&self) -> Arc<crate::certificate_authority::CertMetrics> {
        Arc::clone(&self.metrics)
    }

    /// PEM 문자열에서 RcgenAuthority를 생성하는 편의 생성자.
    ///
    /// CA 인증서 PEM과 개인키 PEM에서 Issuer를 파싱하고 DER 캐시를 구성합니다.
    pub fn from_pem(
        ca_cert_pem: &str,
        ca_key_pem: &str,
        cache_size: u64,
        provider: CryptoProvider,
    ) -> Result<Self, String> {
        let ca_cert_der_bytes = pem::parse(ca_cert_pem)
            .map_err(|e| format!("Failed to parse CA PEM: {}", e))?
            .into_contents();
        let ca_cert_der = CertificateDer::from(ca_cert_der_bytes);
        let key_pair =
            KeyPair::from_pem(ca_key_pem).map_err(|e| format!("Failed to parse key PEM: {}", e))?;
        let issuer = Issuer::from_ca_cert_pem(ca_cert_pem, key_pair)
            .map_err(|e| format!("Failed to create issuer: {}", e))?;

        Ok(Self::new(
            issuer,
            ca_cert_der,
            ca_cert_pem.to_string(),
            ca_key_pem.to_string(),
            cache_size,
            provider,
        ))
    }

    /// 클라이언트 인증서 요청 설정을 업데이트합니다.
    /// 설정 변경 시 캐시를 무효화하여 새 연결에 반영됩니다.
    pub async fn set_client_cert_verify(&self, config: Option<ClientCertVerifyConfig>) {
        *self.client_cert_verify.write().await = config;
        // 설정 변경 시 캐시를 무효화하여 새로운 ServerConfig가 생성되도록 함
        self.cache.invalidate_all();
        info!("클라이언트 인증서 요청 설정 업데이트 완료, 캐시 무효화됨");
    }

    /// 클라이언트 인증서 검증 설정의 Arc 핸들을 반환합니다.
    /// 외부에서 비동기적으로 설정을 업데이트할 때 사용합니다.
    pub fn get_client_cert_verify_handle(
        &self,
    ) -> Arc<tokio::sync::RwLock<Option<ClientCertVerifyConfig>>> {
        Arc::clone(&self.client_cert_verify)
    }

    /// ServerConfig 캐시 핸들을 반환합니다.
    /// 외부에서 설정 변경 시 캐시를 무효화할 때 사용합니다.
    pub fn get_cache_handle(&self) -> Cache<Authority, Arc<ServerConfig>> {
        self.cache.clone()
    }
}
