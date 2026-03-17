use super::InternalProxy;
use super::lazy_sniff_upstream;
use crate::{
    HttpHandler, WebSocketHandler,
    body::Body,
    certificate_authority::CertificateAuthority,
    hybrid_tls_handler::HybridTlsHandler,
    proxy::context::ConnectionStrategy,
    proxy::helpers::{bad_request, spawn_with_trace},
    rewind::Rewind,
    throttle,
    tls_event::{TlsEvent, emit_tls_event},
    tls_version_detector::TlsVersionDetector,
    upstream_cert::{UpstreamCertInfo, sniff_upstream_cert},
    upstream_proxy::connect_to_target,
};
use http::uri::{Authority, Scheme};
use hyper::{Request, Response, body::Bytes, upgrade::Upgraded};
use hyper_util::{client::legacy::connect::Connect, rt::TokioIo};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio_rustls::TlsAcceptor;
use tracing::{Instrument, debug, error, info, info_span, warn};

/// TLS 핸드셰이크 타임아웃 (초)
const TLS_HANDSHAKE_TIMEOUT_SECS: u64 = 30;

/// 스로틀 설정을 적용하여 양방향 복사를 수행하는 헬퍼
async fn copy_bidirectional_maybe_throttled<A, B>(
    a: &mut A,
    b: &mut B,
    throttle_config: Option<&throttle::ThrottleConfig>,
) -> Result<(u64, u64), std::io::Error>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    match throttle_config {
        Some(config) if config.enabled => {
            throttle::copy_bidirectional_throttled(a, b, config).await
        }
        _ => tokio::io::copy_bidirectional(a, b).await,
    }
}

/// TLS 에러 문자열에서 힌트 메시지를 추출하는 헬퍼
pub(super) fn tls_error_hint(error_str: &str) -> &'static str {
    if error_str.contains("SignatureAlgorithmsExtensionRequired") {
        "서버가 SignatureAlgorithmsExtension을 요구함 - TLS 1.2+ 클라이언트 사용 또는 서버 설정 확인"
    } else if error_str.contains("peer is incompatible") {
        "클라이언트-서버 호환성 문제 - 지원하지 않는 TLS 버전, 암호화 스위트, 또는 확장"
    } else if error_str.contains("certificate") {
        "인증서 관련 오류 - 인증서 검증 실패, 만료된 인증서, 또는 CA 신뢰 문제"
    } else if error_str.contains("handshake") {
        "핸드셰이크 프로토콜 오류 - 프로토콜 버전 불일치, 암호화 스위트 협상 실패"
    } else if error_str.contains("timeout") {
        "핸드셰이크 타임아웃 - 네트워크 지연, 서버 과부하, 또는 방화벽 차단"
    } else {
        ""
    }
}

/// TLS 에러 문자열에서 백엔드 종류를 판별하는 헬퍼
pub(super) fn detect_tls_backend(error_str: &str) -> &'static str {
    if error_str.contains("rustls") {
        "RUSTLS"
    } else if error_str.contains("native-tls") || error_str.contains("openssl") {
        "NATIVE-TLS"
    } else {
        "UNKNOWN"
    }
}

impl<C, CA, H, W> InternalProxy<C, CA, H, W>
where
    C: Connect + Clone + Send + Sync + 'static,
    CA: CertificateAuthority,
    H: HttpHandler,
    W: WebSocketHandler,
{
    pub(crate) fn process_connect(mut self, mut req: Request<Body>) -> Response<Body> {
        let Some(authority) = req.uri().authority().cloned() else {
            return bad_request();
        };

        // 자동 학습 바이패스 체크 (이전에 TLS 핸드셰이크 실패한 도메인)
        // should_intercept()가 true를 반환하면(사용자가 SSL Proxying으로 명시적 인터셉트 요청)
        // 자동 바이패스를 무시합니다.
        let has_prior_failure = self
            .ctx
            .tls_passthrough
            .as_ref()
            .and_then(|pt| pt.failures_ref().try_read().ok())
            .is_some_and(|failures| failures.contains_key(authority.host()));

        let span = info_span!("process_connect");
        let fut = async move {
            // should_intercept 결과를 먼저 확인하여 자동 바이패스 적용 여부를 결정
            let should_intercept = self
                .http_handler
                .should_intercept(&self.context(), &req)
                .await;

            if has_prior_failure && !should_intercept {
                debug!(
                    "[TLS-PASSTHROUGH] 자동 바이패스 적용 (이전 실패 기록): {}",
                    authority
                );
            } else if has_prior_failure && should_intercept {
                info!(
                    "[SSLProxying] 이전 실패 기록 무시, 명시적 인터셉트 적용: {}",
                    authority
                );
            }

            let upgraded = match hyper::upgrade::on(&mut req).await {
                Ok(u) => u,
                Err(e) => {
                    error!(error = %e, "연결 업그레이드 실패");
                    return;
                }
            };

            // 이전 TLS 실패 기록이 있고, 사용자가 인터셉트를 요청하지 않은 경우 바이패스
            // ClientHello를 읽지 않고 바로 패스스루
            if has_prior_failure && !should_intercept {
                let upgraded = TokioIo::new(upgraded);
                let upgraded = Rewind::new(upgraded, Bytes::new());
                self.tunnel_passthrough(upgraded, &authority).await;
                return;
            }

            let mut upgraded = TokioIo::new(upgraded);

            let full_buffer = match Self::read_client_hello(&mut upgraded).await {
                Some(buf) => buf,
                None => return,
            };

            let upgraded = Rewind::new(upgraded, Bytes::copy_from_slice(&full_buffer));

            // Eager 전략: 캐시 미스 시 백그라운드 스니핑 시작
            let eager_handle = self.maybe_start_eager_sniffing(&authority).await;

            if should_intercept {
                if full_buffer.len() >= 4 && full_buffer[..4] == *b"GET " {
                    if let Err(e) = self
                        .serve_stream(TokioIo::new(upgraded), Scheme::HTTP, authority)
                        .await
                    {
                        error!("WebSocket connect error: {}", e);
                    }
                    return;
                }

                if full_buffer.len() >= 2 && full_buffer[..2] == *b"\x16\x03" {
                    self.handle_tls_intercept(upgraded, &full_buffer, &authority, eager_handle)
                        .await;
                    return;
                }

                warn!(
                    "Unknown protocol, read '{:02X?}' from upgraded connection",
                    &full_buffer[..full_buffer.len().min(16)]
                );
            }

            // intercept하지 않거나 알 수 없는 프로토콜인 경우
            if let Some(handle) = eager_handle {
                handle.abort();
            }
            self.tunnel_passthrough(upgraded, &authority).await;
        };

        spawn_with_trace(fut, span);
        Response::new(Body::empty())
    }

    /// 클라이언트로부터 TLS ClientHello 메시지를 읽는 헬퍼
    async fn read_client_hello<S: AsyncRead + Unpin>(stream: &mut S) -> Option<Vec<u8>> {
        let mut header_buffer = [0; 5];
        let header_bytes_read = match stream.read(&mut header_buffer).await {
            Ok(n) => n,
            Err(e) => {
                error!("Failed to read TLS header from upgraded connection: {}", e);
                return None;
            }
        };

        if header_bytes_read < 5 {
            error!("TLS header too short: {} bytes", header_bytes_read);
            return None;
        }

        let record_length = u16::from_be_bytes([header_buffer[3], header_buffer[4]]) as usize;
        let total_expected_length = 5 + record_length;

        debug!(
            record_type = format_args!("0x{:02x}", header_buffer[0]),
            record_version = format_args!("0x{:02x}{:02x}", header_buffer[1], header_buffer[2]),
            record_length,
            total_expected_length,
            "[BUFFER-READ] TLS 레코드 분석"
        );

        let mut full_buffer = vec![0; total_expected_length];
        full_buffer[..5].copy_from_slice(&header_buffer);

        let remaining_bytes = total_expected_length - 5;
        if remaining_bytes > 0 {
            let remaining_bytes_read = match stream.read(&mut full_buffer[5..]).await {
                Ok(n) => n,
                Err(e) => {
                    error!("Failed to read remaining TLS data: {}", e);
                    return None;
                }
            };

            debug!(
                header_bytes = 5,
                remaining_bytes_read,
                remaining_bytes_expected = remaining_bytes,
                total_bytes_read = 5 + remaining_bytes_read,
                "[BUFFER-READ] 데이터 읽기 완료"
            );

            if remaining_bytes_read < remaining_bytes {
                warn!(
                    remaining_bytes_read,
                    remaining_bytes_expected = remaining_bytes,
                    "전체 ClientHello가 완전히 읽히지 않음 — 실제 수신 크기로 버퍼 절삭"
                );
                full_buffer.truncate(5 + remaining_bytes_read);
            }
        }

        Some(full_buffer)
    }

    /// 캐시 미스 시 Eager 전략으로 백그라운드 upstream cert 스니핑을 시작
    async fn maybe_start_eager_sniffing(
        &self,
        authority: &Authority,
    ) -> Option<tokio::task::JoinHandle<Option<UpstreamCertInfo>>> {
        if self.ca.is_config_cached(authority).await {
            debug!(
                "[UPSTREAM-CERT] 캐시 히트 - Eager 스니핑 스킵: {}",
                authority
            );
            return None;
        }

        match self.ctx.connection_strategy() {
            ConnectionStrategy::Eager | ConnectionStrategy::EagerWithFallback => {
                let authority_clone = authority.clone();
                let upstream_proxy_clone = self.ctx.upstream_proxy.clone();
                let tls_event_sender = self.ctx.tls_event_sender.clone();
                let is_fallback =
                    self.ctx.connection_strategy() == ConnectionStrategy::EagerWithFallback;

                emit_tls_event(
                    &tls_event_sender,
                    TlsEvent::EagerSniffingStarted {
                        authority: authority_clone.clone(),
                    },
                );

                let start_time = std::time::Instant::now();
                Some(tokio::spawn(async move {
                    let cert =
                        sniff_upstream_cert(&authority_clone, upstream_proxy_clone.as_ref()).await;
                    let duration = start_time.elapsed();
                    let success = cert.is_some();
                    emit_tls_event(
                        &tls_event_sender,
                        TlsEvent::EagerSniffingCompleted {
                            authority: authority_clone,
                            success,
                            duration,
                            fallback_to_lazy: !success && is_fallback,
                        },
                    );
                    cert
                }))
            }
            ConnectionStrategy::Lazy => None,
        }
    }

    /// 연결 전략(Eager/Lazy)에 따라 upstream cert를 획득
    async fn resolve_upstream_cert(
        &self,
        authority: &Authority,
        eager_handle: Option<tokio::task::JoinHandle<Option<UpstreamCertInfo>>>,
    ) -> Option<UpstreamCertInfo> {
        if self.ca.is_config_cached(authority).await {
            debug!("[UPSTREAM-CERT] 캐시 히트 - 스니핑 스킵: {}", authority);
            return None;
        }

        let Some(handle) = eager_handle else {
            // Lazy 전략
            return lazy_sniff_upstream(
                authority,
                self.ctx.upstream_proxy.as_ref(),
                &self.ctx.tls_event_sender,
            )
            .await;
        };

        match handle.await {
            Ok(cert) => {
                emit_tls_event(
                    &self.ctx.tls_event_sender,
                    TlsEvent::UpstreamCertSniffed {
                        authority: authority.clone(),
                        cert_info: cert.clone(),
                    },
                );
                if cert.is_none()
                    && self.ctx.connection_strategy() == ConnectionStrategy::EagerWithFallback
                {
                    debug!("[UPSTREAM-CERT] Eager 실패 → Lazy 폴백: {}", authority);
                    lazy_sniff_upstream(
                        authority,
                        self.ctx.upstream_proxy.as_ref(),
                        &self.ctx.tls_event_sender,
                    )
                    .await
                } else {
                    cert
                }
            }
            Err(e) => {
                warn!("[UPSTREAM-CERT] Eager 태스크 실패: {} - {}", authority, e);
                if self.ctx.connection_strategy() == ConnectionStrategy::EagerWithFallback {
                    lazy_sniff_upstream(
                        authority,
                        self.ctx.upstream_proxy.as_ref(),
                        &self.ctx.tls_event_sender,
                    )
                    .await
                } else {
                    None
                }
            }
        }
    }

    /// TLS 인터셉트 처리: 하이브리드 핸들러 또는 rustls 폴백
    async fn handle_tls_intercept(
        mut self,
        upgraded: Rewind<TokioIo<Upgraded>>,
        full_buffer: &[u8],
        authority: &Authority,
        eager_handle: Option<tokio::task::JoinHandle<Option<UpstreamCertInfo>>>,
    ) {
        let tls_version = TlsVersionDetector::detect_tls_version(full_buffer);

        match tls_version {
            Some(version) => {
                debug!(%version, "TLS 버전 감지 - 하이브리드 핸들러 사용");

                let upstream_cert = self.resolve_upstream_cert(authority, eager_handle).await;
                // 인증서 정보를 InternalProxy에 저장 (이후 context()에서 사용)
                self.upstream_cert_der = upstream_cert.as_ref().and_then(|c| c.cert_der.clone());
                self.upstream_cert_info = upstream_cert.clone();

                let hybrid_handler = match HybridTlsHandler::new(
                    Arc::clone(&self.ca),
                    self.ctx.tls_event_sender.clone(),
                    self.ctx.tls_config.clone(),
                )
                .await
                {
                    Ok(h) => h,
                    Err(e) => {
                        error!(error = %e, "HybridTlsHandler 생성 실패");
                        return;
                    }
                };

                let tls_result = tokio::time::timeout(
                    std::time::Duration::from_secs(TLS_HANDSHAKE_TIMEOUT_SECS),
                    hybrid_handler.handle_tls_connection_upgraded(
                        authority,
                        upgraded,
                        full_buffer,
                        upstream_cert.as_ref(),
                    ),
                )
                .await;

                match tls_result {
                    Err(_) => {
                        error!(
                            authority = %authority,
                            timeout_secs = TLS_HANDSHAKE_TIMEOUT_SECS,
                            "TLS 핸드셰이크 타임아웃"
                        );
                        self.record_tls_failure(authority).await;
                        return;
                    }
                    Ok(Err(e)) => {
                        let error_str = e.to_string();
                        error!(
                            authority = %authority,
                            tls_version = %version,
                            tls_backend = detect_tls_backend(&error_str),
                            error = %e,
                            error_type = ?e,
                            tls_hint = tls_error_hint(&error_str),
                            "하이브리드 TLS 연결 실패"
                        );
                        self.record_tls_failure(authority).await;
                    }
                    Ok(Ok(hybrid_stream)) => {
                        info!(%version, "하이브리드 TLS 연결 성공");
                        self.record_tls_success(authority).await;
                        if let Err(e) = self
                            .serve_stream(
                                TokioIo::new(hybrid_stream),
                                Scheme::HTTPS,
                                authority.clone(),
                            )
                            .await
                        {
                            error!("HTTPS connect error: {}", e);
                        }
                    }
                }
            }
            None => {
                warn!("TLS 버전을 감지할 수 없음, 기존 rustls로 시도");
                self.handle_rustls_fallback(upgraded, authority).await;
            }
        }
    }

    /// rustls 폴백 TLS 핸드셰이크 처리
    async fn handle_rustls_fallback(
        self,
        upgraded: Rewind<TokioIo<Upgraded>>,
        authority: &Authority,
    ) {
        let server_config = match self
            .ca
            .gen_server_config(authority, None)
            .instrument(info_span!("gen_server_config"))
            .await
        {
            Ok(cfg) => cfg,
            Err(e) => {
                error!(authority = %authority, error = %e, "서버 설정 생성 실패");
                return;
            }
        };

        let stream = match TlsAcceptor::from(server_config).accept(upgraded).await {
            Ok(stream) => TokioIo::new(stream),
            Err(e) => {
                let error_str = e.to_string();
                error!(
                    authority = %authority,
                    error = %e,
                    error_type = ?e,
                    tls_hint = tls_error_hint(&error_str),
                    "TLS 핸드셰이크 실패"
                );
                self.record_tls_failure(authority).await;
                return;
            }
        };

        self.record_tls_success(authority).await;

        if let Err(e) = self
            .serve_stream(stream, Scheme::HTTPS, authority.clone())
            .await
        {
            error!("HTTPS connect error: {}", e);
        }
    }

    /// TLS 실패 도메인을 자동 학습 바이패스에 기록
    async fn record_tls_failure(&self, authority: &Authority) {
        if let Some(ref passthrough) = self.ctx.tls_passthrough {
            passthrough.record_failure(authority).await;
        }
    }

    /// TLS 성공 시 이전 실패 기록을 제거하여 바이패스 해제
    async fn record_tls_success(&self, authority: &Authority) {
        if let Some(ref passthrough) = self.ctx.tls_passthrough {
            passthrough.record_success(authority).await;
        }
    }

    /// intercept하지 않는 경우 단순 터널링
    async fn tunnel_passthrough(
        self,
        mut upgraded: Rewind<TokioIo<Upgraded>>,
        authority: &Authority,
    ) {
        let mut server =
            match connect_to_target(authority.as_ref(), self.ctx.upstream_proxy.as_ref()).await {
                Ok(s) => s,
                Err(e) => {
                    error!(authority = %authority, error = %e, "업스트림 서버 연결 실패");
                    return;
                }
            };

        let throttle_config = self
            .ctx
            .throttle_rx
            .as_ref()
            .and_then(|rx| rx.borrow().clone());
        let copy_fut = copy_bidirectional_maybe_throttled(
            &mut upgraded,
            &mut server,
            throttle_config.as_ref(),
        );
        // shutdown 신호 수신 시 터널링을 조기 종료
        if let Some(mut rx) = self.ctx.shutdown_rx.clone() {
            tokio::select! {
                result = copy_fut => {
                    if let Err(e) = result {
                        error!(authority = %authority, error = %e, "터널링 실패");
                    }
                }
                _ = rx.wait_for(|&v| v) => {
                    debug!(authority = %authority, "shutdown 신호 수신, 터널링 종료");
                }
            }
        } else if let Err(e) = copy_fut.await {
            error!(authority = %authority, error = %e, "터널링 실패");
        }
    }
}
