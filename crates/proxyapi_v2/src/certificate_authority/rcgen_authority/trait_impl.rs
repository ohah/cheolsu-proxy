use super::RcgenAuthority;
use crate::certificate_authority::CertificateAuthority;
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use std::sync::Arc;
use tokio_rustls::rustls::ServerConfig;
use tracing::{debug, error, info, warn};

impl RcgenAuthority {
    /// ServerConfig를 빌드하는 내부 메서드.
    /// Thundering herd 방지를 위해 `gen_server_config`에서 `try_get_with`를 통해 호출됩니다.
    async fn build_server_config(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<Arc<ServerConfig>, String> {
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
                    format!(
                        "인증서 생성에 완전히 실패: authority={}, error={:?}",
                        authority, e2
                    )
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
                            format!(
                                "Failed to build WebPkiClientVerifier (allow_unauthenticated): {e}"
                            )
                        })?;

                        info!("[SERVER-CONFIG] 클라이언트 인증서 선택적 요청 (required=false)");
                        ServerConfig::builder_with_provider(Arc::clone(&self.provider))
                            .with_protocol_versions(&supported_versions)
                            .map_err(|e| e.to_string())?
                            .with_client_cert_verifier(verifier)
                            .with_single_cert(certs, leaf_private_key.clone_key())
                            .map_err(|e| e.to_string())?
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
                            format!("Failed to build WebPkiClientVerifier (required): {e}")
                        })?;

                        info!(
                            "[SERVER-CONFIG] 클라이언트 인증서 필수 요청 (CA 검증, {} CA certs)",
                            verify_config.ca_certs.len()
                        );
                        ServerConfig::builder_with_provider(Arc::clone(&self.provider))
                            .with_protocol_versions(&supported_versions)
                            .map_err(|e| e.to_string())?
                            .with_client_cert_verifier(verifier)
                            .with_single_cert(certs, leaf_private_key.clone_key())
                            .map_err(|e| e.to_string())?
                    }
                } else {
                    // 비활성화 - 기존 동작
                    ServerConfig::builder_with_provider(Arc::clone(&self.provider))
                        .with_protocol_versions(&supported_versions)
                        .map_err(|e| e.to_string())?
                        .with_no_client_auth()
                        .with_single_cert(certs, leaf_private_key.clone_key())
                        .map_err(|e| e.to_string())?
                }
            } else {
                // 설정 없음 - 기존 동작
                ServerConfig::builder_with_provider(Arc::clone(&self.provider))
                    .with_protocol_versions(&supported_versions)
                    .map_err(|e| e.to_string())?
                    .with_no_client_auth()
                    .with_single_cert(certs, leaf_private_key.clone_key())
                    .map_err(|e| e.to_string())?
            }
        };

        info!("[SERVER-CONFIG] ServerConfig 빌더 생성 완료");

        server_cfg.alpn_protocols =
            crate::certificate_authority::build_alpn_protocols(upstream_cert);
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

        Ok(server_cfg)
    }
}

impl CertificateAuthority for RcgenAuthority {
    async fn gen_server_config(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<Arc<ServerConfig>, Box<dyn std::error::Error + Send + Sync>> {
        // 캐시 히트: 정확히 일치하는 authority
        if let Some(cfg) = self.cache.get(authority).await {
            self.metrics.record_cache_hit();
            return Ok(cfg);
        }

        // 캐시 히트: 와일드카드 authority (*.example.com)로 서브도메인 공유
        if let Some(wildcard) = crate::certificate_authority::wildcard_authority(authority) {
            if let Some(cfg) = self.cache.get(&wildcard).await {
                // 와일드카드 캐시 히트 → 원본 authority에도 저장
                self.cache.insert(authority.clone(), Arc::clone(&cfg)).await;
                self.metrics.record_cache_hit();
                return Ok(cfg);
            }
        }

        let start = std::time::Instant::now();
        let timeout_duration =
            std::time::Duration::from_secs(crate::certificate_authority::CERT_GEN_TIMEOUT_SECS);

        let result = tokio::time::timeout(
            timeout_duration,
            self.cache.try_get_with(
                authority.clone(),
                self.build_server_config(authority, upstream_cert),
            ),
        )
        .await;

        match result {
            Ok(Ok(cfg)) => {
                self.metrics.record_gen(start.elapsed());
                // 와일드카드 키에도 저장하여 같은 루트 도메인의 서브도메인이 공유
                if let Some(wildcard) = crate::certificate_authority::wildcard_authority(authority)
                {
                    self.cache.insert(wildcard, Arc::clone(&cfg)).await;
                }
                Ok(cfg)
            }
            Ok(Err(e)) => {
                self.metrics.record_failure();
                Err(Box::from(e.to_string()))
            }
            Err(_) => {
                self.metrics.record_failure();
                warn!(
                    "[SERVER-CONFIG] 인증서 생성 타임아웃 ({}초): {}. upstream 정보 없이 재시도",
                    crate::certificate_authority::CERT_GEN_TIMEOUT_SECS,
                    authority
                );
                // 타임아웃 시 upstream 정보 없이 ECDSA(기본)로 폴백
                self.cache
                    .try_get_with(authority.clone(), self.build_server_config(authority, None))
                    .await
                    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                        Box::from(format!("타임아웃 후 폴백도 실패: {}", e))
                    })
            }
        }
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
