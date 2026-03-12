use super::OpensslAuthority;
use crate::certificate_authority::CertificateAuthority;
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use std::sync::Arc;
use tokio_rustls::rustls::ServerConfig;
use tracing::{debug, error, info};

impl CertificateAuthority for OpensslAuthority {
    async fn gen_server_config(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<Arc<ServerConfig>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(server_cfg) = self.cache.get(authority).await {
            debug!("Using cached server config");
            return Ok(server_cfg);
        }
        debug!("Generating server config");

        let cert = match self.gen_cert(authority, upstream_cert) {
            Ok(cert) => cert,
            Err(e) => {
                error!(
                    "[SERVER-CONFIG] 인증서 생성 실패: {} - {:?}. upstream 정보 없이 재시도",
                    authority, e
                );
                // 폴백: upstream 정보 없이 재시도
                self.gen_cert(authority, None).map_err(|e2| {
                    let msg = format!(
                        "인증서 생성에 완전히 실패: authority={}, error={:?}",
                        authority, e2
                    );
                    Box::<dyn std::error::Error + Send + Sync>::from(msg)
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
            .with_protocol_versions(&supported_versions)?
            .with_no_client_auth()
            .with_single_cert(certs, self.private_key.clone_key())?;

        // ALPN 미러링: 상류 서버의 ALPN 협상 결과를 반영
        server_cfg.alpn_protocols = if let Some(ref upstream) = upstream_cert {
            if let Some(ref negotiated) = upstream.negotiated_alpn {
                let mut protocols = vec![negotiated.clone()];
                #[cfg(feature = "http2")]
                if negotiated != b"h2" {
                    protocols.push(b"h2".to_vec());
                }
                if negotiated != b"http/1.1" {
                    protocols.push(b"http/1.1".to_vec());
                }
                info!(
                    "[SERVER-CONFIG] ALPN 미러링 적용: {:?}",
                    protocols
                        .iter()
                        .map(|p| String::from_utf8_lossy(p).to_string())
                        .collect::<Vec<_>>()
                );
                protocols
            } else {
                vec![
                    #[cfg(feature = "http2")]
                    b"h2".to_vec(),
                    b"http/1.1".to_vec(),
                ]
            }
        } else {
            vec![
                #[cfg(feature = "http2")]
                b"h2".to_vec(),
                b"http/1.1".to_vec(),
            ]
        };

        let server_cfg = Arc::new(server_cfg);

        self.cache
            .insert(authority.clone(), Arc::clone(&server_cfg))
            .await;

        Ok(server_cfg)
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
