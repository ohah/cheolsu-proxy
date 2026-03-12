use super::OpensslAuthority;
use crate::certificate_authority::CertificateAuthority;
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use std::sync::Arc;
use tokio_rustls::rustls::ServerConfig;
use tracing::{debug, info};

impl CertificateAuthority for OpensslAuthority {
    async fn gen_server_config(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Arc<ServerConfig> {
        if let Some(server_cfg) = self.cache.get(authority).await {
            debug!("Using cached server config");
            return server_cfg;
        }
        debug!("Generating server config");

        let certs = vec![
            self.gen_cert(authority, upstream_cert)
                .unwrap_or_else(|_| panic!("Failed to generate certificate for {}", authority)),
        ];

        // TLS 버전 설정: TLS 1.2부터 TLS 1.3까지 허용 (rustls 지원 범위)
        let supported_versions = vec![
            &tokio_rustls::rustls::version::TLS12,
            &tokio_rustls::rustls::version::TLS13,
        ];

        let mut server_cfg = ServerConfig::builder_with_provider(Arc::clone(&self.provider))
            .with_protocol_versions(&supported_versions)
            .expect("Failed to specify protocol versions")
            .with_no_client_auth()
            .with_single_cert(certs, self.private_key.clone_key())
            .expect("Failed to build ServerConfig");

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

        server_cfg
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
