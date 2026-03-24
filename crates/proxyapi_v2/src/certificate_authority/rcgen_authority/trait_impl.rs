use super::RcgenAuthority;
use crate::certificate_authority::CertificateAuthority;
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use std::sync::Arc;
use tokio_rustls::rustls::ServerConfig;
use tracing::{debug, error, info, warn};

impl CertificateAuthority for RcgenAuthority {
    async fn gen_server_config(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<Arc<ServerConfig>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(server_cfg) = self.cache.get(authority).await {
            debug!("Using cached server config for {}", authority);
            return Ok(server_cfg);
        }

        info!("[SERVER-CONFIG] 서버 설정 생성 시작: {}", authority);
        let start_time = std::time::Instant::now();

        info!("[SERVER-CONFIG] 인증서 생성 중: {}", authority);
        let (cert, leaf_private_key) = match self.gen_cert(authority, upstream_cert) {
            Ok(result) => result,
            Err(e) => {
                error!(
                    "[SERVER-CONFIG] 인증서 생성 실패: {} - {:?}. 기본 인증서로 폴백",
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
        info!("[SERVER-CONFIG] 인증서 생성 완료: {} bytes", certs[0].len());

        info!("[SERVER-CONFIG] ServerConfig 빌더 생성 중");

        // TLS 버전 설정: TLS 1.2부터 TLS 1.3까지 허용 (rustls 지원 범위)
        let supported_versions = vec![
            &tokio_rustls::rustls::version::TLS12,
            &tokio_rustls::rustls::version::TLS13,
        ];

        let mut server_cfg = {
            let verify_config_guard = self.client_cert_verify.read().await;
            if let Some(ref verify_config) = *verify_config_guard {
                if verify_config.enabled {
                    if verify_config.ca_certs.is_empty() || !verify_config.required {
                        // 모든 인증서 수락 (선택적 요청) - 인증서 없어도 연결 허용
                        let verifier = tokio_rustls::rustls::server::WebPkiClientVerifier::builder(
                            Arc::new(tokio_rustls::rustls::RootCertStore::empty()),
                        )
                        .allow_unauthenticated()
                        .build()
                        .map_err(|e| {
                            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                                "Failed to build WebPkiClientVerifier (allow_unauthenticated): {e}"
                            ))
                        })?;

                        info!("[SERVER-CONFIG] 클라이언트 인증서 선택적 요청 (required=false)");
                        ServerConfig::builder_with_provider(Arc::clone(&self.provider))
                            .with_protocol_versions(&supported_versions)?
                            .with_client_cert_verifier(verifier)
                            .with_single_cert(certs, leaf_private_key.clone_key())?
                    } else {
                        // CA 기반 검증 (필수)
                        let mut root_store = tokio_rustls::rustls::RootCertStore::empty();
                        for ca_cert in &verify_config.ca_certs {
                            if let Err(e) = root_store.add(ca_cert.clone()) {
                                warn!("Failed to add CA cert to root store: {:?}", e);
                            }
                        }
                        let verifier = tokio_rustls::rustls::server::WebPkiClientVerifier::builder(
                            Arc::new(root_store),
                        )
                        .build()
                        .map_err(|e| {
                            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                                "Failed to build WebPkiClientVerifier (required): {e}"
                            ))
                        })?;

                        info!(
                            "[SERVER-CONFIG] 클라이언트 인증서 필수 요청 (CA 검증, {} CA certs)",
                            verify_config.ca_certs.len()
                        );
                        ServerConfig::builder_with_provider(Arc::clone(&self.provider))
                            .with_protocol_versions(&supported_versions)?
                            .with_client_cert_verifier(verifier)
                            .with_single_cert(certs, leaf_private_key.clone_key())?
                    }
                } else {
                    // 비활성화 - 기존 동작
                    ServerConfig::builder_with_provider(Arc::clone(&self.provider))
                        .with_protocol_versions(&supported_versions)?
                        .with_no_client_auth()
                        .with_single_cert(certs, leaf_private_key.clone_key())?
                }
            } else {
                // 설정 없음 - 기존 동작
                ServerConfig::builder_with_provider(Arc::clone(&self.provider))
                    .with_protocol_versions(&supported_versions)?
                    .with_no_client_auth()
                    .with_single_cert(certs, leaf_private_key.clone_key())?
            }
        };

        info!("[SERVER-CONFIG] ServerConfig 빌더 생성 완료");

        // ALPN 미러링: 상류 서버의 ALPN 협상 결과를 반영
        server_cfg.alpn_protocols = if let Some(ref upstream) = upstream_cert {
            if let Some(ref negotiated) = upstream.negotiated_alpn {
                // 상류 서버가 협상한 프로토콜 우선, 나머지도 포함
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

        info!(
            "[SERVER-CONFIG] ALPN 프로토콜 설정: {:?}",
            server_cfg.alpn_protocols
        );

        // 지원되는 TLS 버전 로깅 (rustls 0.23+에서는 다른 방법으로 확인)
        info!("[SERVER-CONFIG] rustls ServerConfig 생성 완료");

        let server_cfg = Arc::new(server_cfg);
        let duration = start_time.elapsed();

        info!(
            "[SERVER-CONFIG] 서버 설정 생성 완료: {} (소요시간: {:?})",
            authority, duration
        );

        self.cache
            .insert(authority.clone(), Arc::clone(&server_cfg))
            .await;

        Ok(server_cfg)
    }

    fn get_ca_cert_der(&self) -> Option<Vec<u8>> {
        let der_bytes = self.ca_cert_der.to_vec();
        debug!(
            "Successfully extracted CA certificate DER ({} bytes)",
            der_bytes.len()
        );
        Some(der_bytes)
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
