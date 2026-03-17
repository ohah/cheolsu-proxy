//! Hybrid TLS 핸들러 모듈
//!
//! rustls와 OpenSSL을 사용하여 TLS 연결을 처리하는 하이브리드 핸들러입니다.
//! 연결 분석 결과에 따라 적절한 TLS 라이브러리를 선택합니다.

mod analysis;
#[cfg(feature = "openssl-ca")]
mod openssl_handler;
mod rustls_handler;
mod stream;
#[cfg(test)]
mod tests;
mod types;

pub(crate) use analysis::{analyze_tls_connection, determine_tls_strategy};
// calculate_complexity_score, get_extension_name, is_openssl_required_domain are
// used internally in analysis.rs and re-exported for tests below.
pub(crate) use stream::HybridTlsStream;
pub use types::{
    LearnedTlsStrategy, TlsConnectionInfo, TlsExtension, TlsStrategy, TlsStrategyCache,
};

use crate::certificate_authority::CertificateAuthority;
use crate::rewind::Rewind;
use crate::tls_config::SharedTlsConfig;
use crate::tls_event::{TlsEvent, TlsEventSender, emit_tls_event};
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tracing::{error, info, warn};

/// TLS 핸들러 - rustls 사용 (Hudsucker 방식으로 단순화)
pub(crate) struct HybridTlsHandler<CA: CertificateAuthority> {
    ca: Arc<CA>,
    tls_event_sender: Option<TlsEventSender>,
    tls_config: Option<SharedTlsConfig>,
    /// 도메인별 학습된 TLS 전략 캐시
    strategy_cache: Option<TlsStrategyCache>,
}

impl<CA: CertificateAuthority> HybridTlsHandler<CA> {
    /// 새로운 TLS 핸들러를 생성합니다
    pub async fn new(
        ca: Arc<CA>,
        tls_event_sender: Option<TlsEventSender>,
        tls_config: Option<SharedTlsConfig>,
        strategy_cache: Option<TlsStrategyCache>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            ca,
            tls_event_sender,
            tls_config,
            strategy_cache,
        })
    }

    /// 학습된 전략이 있으면 반환합니다
    async fn get_learned_strategy(&self, host: &str) -> Option<TlsStrategy> {
        if let Some(cache) = &self.strategy_cache {
            let guard = cache.read().await;
            guard.get(host).copied()
        } else {
            None
        }
    }

    /// 성공한 전략을 학습합니다 (이미 동일한 전략이 캐시되어 있으면 skip)
    async fn learn_strategy(&self, host: &str, strategy: TlsStrategy) {
        if let Some(cache) = &self.strategy_cache {
            // read lock으로 먼저 확인하여 불필요한 write lock 방지
            {
                let guard = cache.read().await;
                if guard.get(host) == Some(&strategy) {
                    return;
                }
            }
            let mut guard = cache.write().await;
            guard.insert(host.to_string(), strategy);
        }
    }

    /// TLS 연결을 상세 분석합니다
    fn analyze_tls_connection(
        &self,
        initial_buffer: &[u8],
    ) -> Result<TlsConnectionInfo, Box<dyn std::error::Error + Send + Sync>> {
        analyze_tls_connection(initial_buffer)
    }

    /// TLS 처리 전략을 결정합니다
    async fn determine_tls_strategy(
        &self,
        authority: &Authority,
        tls_info: &TlsConnectionInfo,
    ) -> TlsStrategy {
        if let Some(config) = &self.tls_config {
            let guard = config.read().await;
            determine_tls_strategy(authority, tls_info, Some(&guard))
        } else {
            determine_tls_strategy(authority, tls_info, None)
        }
    }

    /// TLS 버전을 감지하고 적절한 TLS 핸들러를 선택합니다 (Upgraded 스트림 전용)
    ///
    /// 반환값: `(TLS 스트림, 폴백 사용 여부)`
    pub(crate) async fn handle_tls_connection_upgraded(
        &self,
        authority: &Authority,
        upgraded: Rewind<TokioIo<Upgraded>>,
        initial_buffer: &[u8],
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<(HybridTlsStream, bool), Box<dyn std::error::Error + Send + Sync>> {
        info!("🔍 [TLS-NEGOTIATION] 새로운 TLS 협상 시작: {}", authority);
        let handshake_start = std::time::Instant::now();

        // 1단계: TLS 연결 유효성 검사
        let tls_info = self.analyze_tls_connection(initial_buffer)?;
        info!("📊 [TLS-INFO] 연결 분석 완료: {:?}", tls_info);

        emit_tls_event(
            &self.tls_event_sender,
            TlsEvent::ClientHelloAnalyzed {
                authority: authority.clone(),
                tls_info: tls_info.clone(),
            },
        );

        // 2단계: 전략 선택 (학습된 전략 우선)
        let host = authority.host();
        let analyzed_strategy = self.determine_tls_strategy(authority, &tls_info).await;
        let learned = self.get_learned_strategy(host).await;
        let strategy = if let Some(learned_strategy) = learned {
            if learned_strategy != analyzed_strategy {
                info!(
                    "🎯 [TLS-STRATEGY] 학습된 전략 사용: {:?} (분석 결과: {:?})",
                    learned_strategy, analyzed_strategy
                );
            }
            learned_strategy
        } else {
            analyzed_strategy
        };
        let fallback_used = learned.is_some_and(|s| s != analyzed_strategy);

        info!("🎯 [TLS-STRATEGY] 선택된 전략: {:?}", strategy);

        emit_tls_event(
            &self.tls_event_sender,
            TlsEvent::StrategySelected {
                authority: authority.clone(),
                strategy,
                tls_info,
            },
        );

        // 3단계: 선택된 전략으로 연결 시도
        let result = match strategy {
            TlsStrategy::OpenSslOnly => {
                info!("🔧 [OPENSSL-ONLY] OpenSSL 전용 처리: {}", authority);
                #[cfg(feature = "openssl-ca")]
                {
                    self.handle_with_openssl_upgraded(
                        authority,
                        upgraded,
                        initial_buffer,
                        upstream_cert,
                    )
                    .await
                }
                #[cfg(not(feature = "openssl-ca"))]
                {
                    error!(
                        "❌ OpenSSL 전용 도메인은 openssl-ca feature가 필요합니다: {}",
                        authority
                    );
                    Err("OpenSSL-only domain requires openssl-ca feature".into())
                }
            }
            TlsStrategy::RustlsOnly => {
                info!("🔧 [RUSTLS-ONLY] Rustls 전용 처리: {}", authority);
                self.handle_with_rustls_upgraded(authority, upgraded, initial_buffer, upstream_cert)
                    .await
            }
        };

        let duration = handshake_start.elapsed();
        match &result {
            Ok(_) => {
                // 성공한 전략을 학습
                self.learn_strategy(host, strategy).await;
                info!(
                    "✅ [TLS] 핸드셰이크 성공: {} ({:?}){}",
                    authority,
                    duration,
                    if fallback_used { " [폴백]" } else { "" }
                );
                emit_tls_event(
                    &self.tls_event_sender,
                    TlsEvent::HandshakeCompleted {
                        authority: authority.clone(),
                        strategy,
                        duration,
                        fallback_used,
                    },
                );
            }
            Err(e) => {
                // 실패 시 반대 전략을 학습 (다음 연결에서 시도)
                let opposite = strategy.opposite();
                warn!(
                    "❌ [TLS] 핸드셰이크 실패: {} - {}. 다음 연결에서 {:?} 전략 시도 예정",
                    authority, e, opposite
                );
                self.learn_strategy(host, opposite).await;
                emit_tls_event(
                    &self.tls_event_sender,
                    TlsEvent::HandshakeFailed {
                        authority: authority.clone(),
                        strategy,
                        error: e.to_string(),
                        duration,
                    },
                );
            }
        }

        result.map(|stream| (stream, fallback_used))
    }

    /// TLS 버전을 감지하고 적절한 TLS 핸들러를 선택합니다
    #[allow(dead_code)]
    pub(crate) async fn handle_tls_connection<R, W>(
        &self,
        authority: &Authority,
        stream: (R, W),
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (mut read_stream, write_stream) = stream;

        // TLS 버전 감지를 위한 버퍼
        let mut buffer = [0u8; 5];
        let bytes_read = read_stream.read(&mut buffer).await?;

        if bytes_read < 5 {
            return Err("TLS handshake data too short".into());
        }

        // Hudsucker 방식: 간단한 TLS 감지
        let is_tls = buffer.len() >= 2 && buffer[..2] == *b"\x16\x03";

        if is_tls {
            info!("✅ TLS 감지됨, rustls로 처리");
            self.handle_with_rustls(authority, (read_stream, write_stream), &buffer)
                .await
        } else {
            error!("❌ TLS가 감지되지 않음");
            Err("TLS not detected".into())
        }
    }
}
