use super::OpensslAuthority;
use crate::certificate_authority::CertificateAuthority;
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use std::sync::Arc;
use tokio_rustls::rustls::ServerConfig;
use tracing::{debug, error};

impl OpensslAuthority {
    /// ServerConfig를 빌드하는 내부 메서드.
    /// Thundering herd 방지를 위해 `gen_server_config`에서 `try_get_with`를 통해 호출됩니다.
    async fn build_server_config(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<Arc<ServerConfig>, String> {
        debug!("Generating server config");

        let (cert, leaf_private_key) = match self.gen_cert(authority, upstream_cert) {
            Ok(result) => result,
            Err(e) => {
                error!(
                    "[SERVER-CONFIG] 인증서 생성 실패: {} - {:?}. upstream 정보 없이 재시도",
                    authority, e
                );
                // 폴백: upstream 정보 없이 재시도
                self.gen_cert(authority, None).map_err(|e2| {
                    format!(
                        "인증서 생성에 완전히 실패: authority={}, error={:?}",
                        authority, e2
                    )
                })?
            }
        };
        let certs = vec![cert];

        // TLS 버전 설정: TLS 1.2부터 TLS 1.3까지 허용 (rustls 지원 범위)
        let supported_versions = vec![
            &tokio_rustls::rustls::version::TLS12,
            &tokio_rustls::rustls::version::TLS13,
        ];

        let mut server_cfg = ServerConfig::builder_with_provider(Arc::clone(&self.provider))
            .with_protocol_versions(&supported_versions)
            .map_err(|e| e.to_string())?
            .with_no_client_auth()
            .with_single_cert(certs, leaf_private_key)
            .map_err(|e| e.to_string())?;

        server_cfg.alpn_protocols =
            crate::certificate_authority::build_alpn_protocols(upstream_cert);

        Ok(Arc::new(server_cfg))
    }
}

impl CertificateAuthority for OpensslAuthority {
    async fn gen_server_config(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<Arc<ServerConfig>, Box<dyn std::error::Error + Send + Sync>> {
        // Thundering herd 방지: 동일 authority에 대해 동시 요청이 오면 하나만 생성하고 나머지는 대기
        self.cache
            .try_get_with(
                authority.clone(),
                self.build_server_config(authority, upstream_cert),
            )
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::from(e.to_string()) })
    }

    fn get_ca_cert_der(&self) -> Option<Vec<u8>> {
        // OpenSSL X509 인증서를 DER 형식으로 변환
        self.ca_cert.to_der().ok()
    }

    async fn is_config_cached(&self, authority: &Authority) -> bool {
        self.cache.get(authority).await.is_some()
    }

    #[cfg(feature = "openssl-ca")]
    async fn gen_openssl_context(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
        super::openssl_context::gen_openssl_context(self, authority, upstream_cert).await
    }
}
