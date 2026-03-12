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
    upstream_cert::sniff_upstream_cert,
    upstream_proxy::connect_to_target,
};
use http::uri::Scheme;
use hyper::{Request, Response, body::Bytes};
use hyper_util::{client::legacy::connect::Connect, rt::TokioIo};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio_rustls::TlsAcceptor;
use tracing::{Instrument, debug, error, info, info_span, warn};

impl<C, CA, H, W> InternalProxy<C, CA, H, W>
where
    C: Connect + Clone + Send + Sync + 'static,
    CA: CertificateAuthority,
    H: HttpHandler,
    W: WebSocketHandler,
{
    pub(crate) fn process_connect(mut self, mut req: Request<Body>) -> Response<Body> {
        match req.uri().authority().cloned() {
            Some(authority) => {
                // 자동 학습 바이패스 체크 (이전에 TLS 핸드셰이크 실패한 도메인)
                let should_bypass = if let Some(ref passthrough) = self.ctx.tls_passthrough {
                    match passthrough.failures_ref().try_read() {
                        Ok(failures) => failures.contains_key(authority.host()),
                        Err(_) => false,
                    }
                } else {
                    false
                };

                if should_bypass {
                    debug!(
                        "[TLS-PASSTHROUGH] 자동 바이패스 적용 (이전 실패 기록): {}",
                        authority
                    );
                    let response = match Response::builder()
                        .status(200)
                        .header("Connection", "keep-alive")
                        .body(Body::empty())
                    {
                        Ok(resp) => resp,
                        Err(e) => {
                            error!("바이패스 응답 생성 실패: {}", e);
                            return bad_request();
                        }
                    };

                    let authority_clone = authority.clone();
                    let _tunnel_sender = self.ctx.tunnel_event_sender.clone();
                    let _client_addr = self.client_addr;
                    tokio::spawn(async move {
                        match hyper::upgrade::on(&mut req).await {
                            Ok(upgraded) => {
                                let mut client_stream = TokioIo::new(upgraded);
                                let target_addr = format!(
                                    "{}:{}",
                                    authority_clone.host(),
                                    authority_clone
                                        .port()
                                        .map(|p| p.to_string())
                                        .unwrap_or_else(|| "443".to_string())
                                );
                                match connect_to_target(
                                    &target_addr,
                                    self.ctx.upstream_proxy.as_ref(),
                                )
                                .await
                                {
                                    Ok(mut server_stream) => {
                                        let throttle_config = self
                                            .ctx
                                            .throttle_rx
                                            .as_ref()
                                            .and_then(|rx| rx.borrow().clone());
                                        let _ = if let Some(ref config) = throttle_config {
                                            if config.enabled {
                                                throttle::copy_bidirectional_throttled(
                                                    &mut client_stream,
                                                    &mut server_stream,
                                                    config,
                                                )
                                                .await
                                            } else {
                                                tokio::io::copy_bidirectional(
                                                    &mut client_stream,
                                                    &mut server_stream,
                                                )
                                                .await
                                            }
                                        } else {
                                            tokio::io::copy_bidirectional(
                                                &mut client_stream,
                                                &mut server_stream,
                                            )
                                            .await
                                        };
                                    }
                                    Err(e) => {
                                        error!(
                                            "[TLS-PASSTHROUGH] 서버 연결 실패: {} - {}",
                                            target_addr, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                error!("[TLS-PASSTHROUGH] 업그레이드 실패: {}", e);
                            }
                        }
                    });

                    return response;
                }

                let span = info_span!("process_connect");
                let fut = async move {
                    match hyper::upgrade::on(&mut req).await {
                        Ok(upgraded) => {
                            let mut upgraded = TokioIo::new(upgraded);

                            // 먼저 TLS 레코드 헤더를 읽어서 전체 길이를 파악
                            let mut header_buffer = [0; 5]; // TLS 레코드 헤더 (5 bytes)
                            let header_bytes_read = match upgraded.read(&mut header_buffer).await {
                                Ok(bytes_read) => bytes_read,
                                Err(e) => {
                                    error!(
                                        "Failed to read TLS header from upgraded connection: {}",
                                        e
                                    );
                                    return;
                                }
                            };

                            if header_bytes_read < 5 {
                                error!("TLS header too short: {} bytes", header_bytes_read);
                                return;
                            }

                            // 레코드 길이 계산 (bytes 3-4)
                            let record_length =
                                u16::from_be_bytes([header_buffer[3], header_buffer[4]]) as usize;
                            let total_expected_length = 5 + record_length; // 헤더(5) + 레코드 길이

                            debug!(
                                record_type = format_args!("0x{:02x}", header_buffer[0]),
                                record_version = format_args!(
                                    "0x{:02x}{:02x}",
                                    header_buffer[1], header_buffer[2]
                                ),
                                record_length,
                                total_expected_length,
                                "[BUFFER-READ] TLS 레코드 분석"
                            );

                            // 전체 ClientHello 메시지를 읽기 위한 버퍼 생성
                            let mut full_buffer = vec![0; total_expected_length];
                            full_buffer[..5].copy_from_slice(&header_buffer);

                            // 나머지 데이터 읽기
                            let remaining_bytes = total_expected_length - 5;
                            if remaining_bytes > 0 {
                                let remaining_bytes_read =
                                    match upgraded.read(&mut full_buffer[5..]).await {
                                        Ok(bytes_read) => bytes_read,
                                        Err(e) => {
                                            error!("Failed to read remaining TLS data: {}", e);
                                            return;
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
                                        "전체 ClientHello가 완전히 읽히지 않음"
                                    );
                                }
                            }

                            let mut upgraded =
                                Rewind::new(upgraded, Bytes::copy_from_slice(&full_buffer));

                            // Eager 전략: 캐시 미스 시 ClientHello 감지 전에 백그라운드 스니핑 시작
                            let eager_handle = if !self.ca.is_config_cached(&authority).await {
                                match self.ctx.connection_strategy() {
                                    ConnectionStrategy::Eager
                                    | ConnectionStrategy::EagerWithFallback => {
                                        let authority_clone = authority.clone();
                                        let upstream_proxy_clone = self.ctx.upstream_proxy.clone();
                                        let tls_event_sender = self.ctx.tls_event_sender.clone();

                                        emit_tls_event(
                                            &tls_event_sender,
                                            TlsEvent::EagerSniffingStarted {
                                                authority: authority_clone.clone(),
                                            },
                                        );

                                        let start_time = std::time::Instant::now();
                                        let is_fallback = self.ctx.connection_strategy()
                                            == ConnectionStrategy::EagerWithFallback;

                                        Some(tokio::spawn(async move {
                                            let cert = sniff_upstream_cert(
                                                &authority_clone,
                                                upstream_proxy_clone.as_ref(),
                                            )
                                            .await;
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
                            } else {
                                debug!(
                                    "[UPSTREAM-CERT] 캐시 히트 - Eager 스니핑 스킵: {}",
                                    authority
                                );
                                None
                            };

                            if self
                                .http_handler
                                .should_intercept(&self.context(), &req)
                                .await
                            {
                                if full_buffer.len() >= 4 && full_buffer[..4] == *b"GET " {
                                    if let Err(e) = self
                                        .serve_stream(
                                            TokioIo::new(upgraded),
                                            Scheme::HTTP,
                                            authority,
                                        )
                                        .await
                                    {
                                        error!("WebSocket connect error: {}", e);
                                    }

                                    return;
                                } else if full_buffer.len() >= 2 && full_buffer[..2] == *b"\x16\x03"
                                {
                                    // TLS 버전 감지
                                    let tls_version =
                                        TlsVersionDetector::detect_tls_version(&full_buffer);

                                    match tls_version {
                                        Some(version) => {
                                            debug!(
                                                %version,
                                                "TLS 버전 감지 - 하이브리드 핸들러 사용"
                                            );

                                            // 연결 전략에 따라 upstream cert 획득
                                            let upstream_cert = if self
                                                .ca
                                                .is_config_cached(&authority)
                                                .await
                                            {
                                                debug!(
                                                    "[UPSTREAM-CERT] 캐시 히트 - 스니핑 스킵: {}",
                                                    authority
                                                );
                                                None
                                            } else if let Some(handle) = eager_handle {
                                                // Eager 핸들이 있으면 결과를 기다림
                                                match handle.await {
                                                    Ok(cert) => {
                                                        emit_tls_event(
                                                            &self.ctx.tls_event_sender,
                                                            TlsEvent::UpstreamCertSniffed {
                                                                authority: authority.clone(),
                                                                cert_info: cert.clone(),
                                                            },
                                                        );
                                                        // EagerWithFallback: eager 실패 시 lazy 폴백
                                                        if cert.is_none()
                                                            && self.ctx.connection_strategy()
                                                                == ConnectionStrategy::EagerWithFallback
                                                        {
                                                            debug!(
                                                                "[UPSTREAM-CERT] Eager 실패 → Lazy 폴백: {}",
                                                                authority
                                                            );
                                                            lazy_sniff_upstream(
                                                                &authority,
                                                                self.ctx.upstream_proxy.as_ref(),
                                                                &self.ctx.tls_event_sender,
                                                            )
                                                            .await
                                                        } else {
                                                            cert
                                                        }
                                                    }
                                                    Err(e) => {
                                                        warn!(
                                                            "[UPSTREAM-CERT] Eager 태스크 실패: {} - {}",
                                                            authority, e
                                                        );
                                                        // EagerWithFallback: spawn 실패 시 lazy 폴백
                                                        if self.ctx.connection_strategy()
                                                            == ConnectionStrategy::EagerWithFallback
                                                        {
                                                            lazy_sniff_upstream(
                                                                &authority,
                                                                self.ctx.upstream_proxy.as_ref(),
                                                                &self.ctx.tls_event_sender,
                                                            )
                                                            .await
                                                        } else {
                                                            None
                                                        }
                                                    }
                                                }
                                            } else {
                                                // Lazy 전략: 기존 동작
                                                lazy_sniff_upstream(
                                                    &authority,
                                                    self.ctx.upstream_proxy.as_ref(),
                                                    &self.ctx.tls_event_sender,
                                                )
                                                .await
                                            };

                                            // HybridTlsHandler 생성
                                            let hybrid_handler = match HybridTlsHandler::new(
                                                Arc::clone(&self.ca),
                                                self.ctx.tls_event_sender.clone(),
                                                self.ctx.tls_config.clone(),
                                            )
                                            .await
                                            {
                                                Ok(handler) => handler,
                                                Err(e) => {
                                                    error!(
                                                        error = %e,
                                                        "HybridTlsHandler 생성 실패"
                                                    );
                                                    return;
                                                }
                                            };

                                            // 하이브리드 TLS 연결 처리
                                            match hybrid_handler
                                                .handle_tls_connection_upgraded(
                                                    &authority,
                                                    upgraded,
                                                    &full_buffer,
                                                    upstream_cert.as_ref(),
                                                )
                                                .await
                                            {
                                                Ok(hybrid_stream) => {
                                                    info!(
                                                        %version,
                                                        "하이브리드 TLS 연결 성공"
                                                    );
                                                    let stream = TokioIo::new(hybrid_stream);

                                                    if let Err(e) = self
                                                        .serve_stream(
                                                            stream,
                                                            Scheme::HTTPS,
                                                            authority.clone(),
                                                        )
                                                        .await
                                                    {
                                                        error!("HTTPS connect error: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    // 오류 메시지에서 TLS 백엔드 확인
                                                    let error_str = e.to_string();
                                                    let tls_backend =
                                                        if error_str.contains("rustls") {
                                                            "RUSTLS"
                                                        } else if error_str.contains("native-tls")
                                                            || error_str.contains("openssl")
                                                        {
                                                            "NATIVE-TLS"
                                                        } else {
                                                            "UNKNOWN"
                                                        };

                                                    let tls_hint = if error_str.contains(
                                                        "SignatureAlgorithmsExtensionRequired",
                                                    ) {
                                                        "서버가 SignatureAlgorithmsExtension을 요구함 - TLS 1.2+ 클라이언트 사용 또는 서버 설정 확인"
                                                    } else if error_str
                                                        .contains("peer is incompatible")
                                                    {
                                                        "클라이언트-서버 호환성 문제 - 지원하지 않는 TLS 버전, 암호화 스위트, 또는 확장"
                                                    } else if error_str.contains("certificate") {
                                                        "인증서 관련 오류 - 인증서 검증 실패, 만료된 인증서, 또는 CA 신뢰 문제"
                                                    } else {
                                                        ""
                                                    };

                                                    error!(
                                                        authority = %authority,
                                                        tls_version = %version,
                                                        tls_backend,
                                                        error = %e,
                                                        error_type = ?e,
                                                        tls_hint,
                                                        "하이브리드 TLS 연결 실패"
                                                    );

                                                    // 자동 학습: 실패한 도메인 기록
                                                    if let Some(ref passthrough) =
                                                        self.ctx.tls_passthrough
                                                    {
                                                        passthrough
                                                            .record_failure(&authority)
                                                            .await;
                                                    }

                                                    return;
                                                }
                                            }
                                        }
                                        None => {
                                            warn!("TLS 버전을 감지할 수 없음, 기존 rustls로 시도");

                                            // 기존 rustls 로직 사용
                                            let server_config = match self
                                                .ca
                                                .gen_server_config(&authority, None)
                                                .instrument(info_span!("gen_server_config"))
                                                .await
                                            {
                                                Ok(cfg) => cfg,
                                                Err(e) => {
                                                    error!(
                                                        authority = %authority,
                                                        error = %e,
                                                        "서버 설정 생성 실패"
                                                    );
                                                    return;
                                                }
                                            };

                                            let stream = match TlsAcceptor::from(server_config)
                                                .accept(upgraded)
                                                .await
                                            {
                                                Ok(stream) => TokioIo::new(stream),
                                                Err(e) => {
                                                    let error_str = e.to_string();
                                                    let tls_hint = if error_str.contains(
                                                        "SignatureAlgorithmsExtensionRequired",
                                                    ) {
                                                        "서버가 SignatureAlgorithmsExtension을 요구함 - TLS 1.2+ 클라이언트 사용 또는 서버 설정 확인"
                                                    } else if error_str
                                                        .contains("peer is incompatible")
                                                    {
                                                        "클라이언트-서버 호환성 문제 - 지원하지 않는 TLS 버전, 암호화 스위트, 또는 확장"
                                                    } else if error_str.contains("certificate") {
                                                        "인증서 관련 오류 - 인증서 검증 실패, 만료된 인증서, 또는 CA 신뢰 문제"
                                                    } else if error_str.contains("handshake") {
                                                        "핸드셰이크 프로토콜 오류 - 프로토콜 버전 불일치, 암호화 스위트 협상 실패"
                                                    } else if error_str.contains("timeout") {
                                                        "핸드셰이크 타임아웃 - 네트워크 지연, 서버 과부하, 또는 방화벽 차단"
                                                    } else {
                                                        ""
                                                    };

                                                    error!(
                                                        authority = %authority,
                                                        error = %e,
                                                        error_type = ?e,
                                                        tls_hint,
                                                        "TLS 핸드셰이크 실패"
                                                    );

                                                    // 자동 학습: 실패한 도메인 기록
                                                    if let Some(ref passthrough) =
                                                        self.ctx.tls_passthrough
                                                    {
                                                        passthrough
                                                            .record_failure(&authority)
                                                            .await;
                                                    }

                                                    return;
                                                }
                                            };

                                            if let Err(e) = self
                                                .serve_stream(
                                                    stream,
                                                    Scheme::HTTPS,
                                                    authority.clone(),
                                                )
                                                .await
                                            {
                                                error!("HTTPS connect error: {}", e);
                                            }
                                        }
                                    }

                                    return;
                                } else {
                                    warn!(
                                        "Unknown protocol, read '{:02X?}' from upgraded connection",
                                        &full_buffer[..full_buffer.len().min(16)]
                                    );
                                }
                            }

                            // intercept하지 않거나 알 수 없는 프로토콜인 경우
                            // 백그라운드 Eager 스니핑 태스크가 있으면 중단하여 리소스 낭비 방지
                            if let Some(handle) = eager_handle {
                                handle.abort();
                            }

                            let mut server = match connect_to_target(
                                authority.as_ref(),
                                self.ctx.upstream_proxy.as_ref(),
                            )
                            .await
                            {
                                Ok(server) => server,
                                Err(e) => {
                                    error!(
                                        authority = %authority,
                                        error = %e,
                                        "업스트림 서버 연결 실패"
                                    );
                                    return;
                                }
                            };

                            let throttle_config = self
                                .ctx
                                .throttle_rx
                                .as_ref()
                                .and_then(|rx| rx.borrow().clone());
                            let tunnel_result = if let Some(ref config) = throttle_config {
                                if config.enabled {
                                    throttle::copy_bidirectional_throttled(
                                        &mut upgraded,
                                        &mut server,
                                        config,
                                    )
                                    .await
                                } else {
                                    tokio::io::copy_bidirectional(&mut upgraded, &mut server).await
                                }
                            } else {
                                tokio::io::copy_bidirectional(&mut upgraded, &mut server).await
                            };
                            if let Err(e) = tunnel_result {
                                error!(
                                    authority = %authority,
                                    error = %e,
                                    "터널링 실패"
                                );
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "연결 업그레이드 실패");
                        }
                    };
                };

                spawn_with_trace(fut, span);
                Response::new(Body::empty())
            }
            None => bad_request(),
        }
    }
}
