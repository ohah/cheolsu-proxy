mod analysis;
mod stream;
mod types;

pub use types::{TlsConnectionInfo, TlsExtension, TlsStrategy};
// 테스트 및 크레이트 내부에서 순수 함수에 직접 접근할 수 있도록 re-export
#[allow(unused_imports)]
pub(crate) use analysis::{
    analyze_tls_connection, calculate_complexity_score, determine_tls_strategy, get_extension_name,
    is_openssl_required_domain,
};
pub(crate) use stream::HybridTlsStream;

use crate::certificate_authority::CertificateAuthority;
use crate::rewind::Rewind;
use crate::tls_config::SharedTlsConfig;
use crate::tls_event::{TlsEvent, TlsEventSender, emit_tls_event};
use crate::tls_version_detector::TlsVersion;
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_openssl::SslStream;
use tokio_rustls::TlsAcceptor;
use tracing::{error, info};

#[cfg(feature = "openssl-ca")]
use {std::pin::Pin, tracing::warn};

/// TLS 핸들러 - rustls 사용 (Hudsucker 방식으로 단순화)
pub(crate) struct HybridTlsHandler<CA: CertificateAuthority> {
    ca: Arc<CA>,
    tls_event_sender: Option<TlsEventSender>,
    tls_config: Option<SharedTlsConfig>,
}

impl<CA: CertificateAuthority> HybridTlsHandler<CA> {
    /// 새로운 TLS 핸들러를 생성합니다
    pub async fn new(
        ca: Arc<CA>,
        tls_event_sender: Option<TlsEventSender>,
        tls_config: Option<SharedTlsConfig>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            ca,
            tls_event_sender,
            tls_config,
        })
    }

    /// TLS 연결을 상세 분석합니다
    fn analyze_tls_connection(
        &self,
        initial_buffer: &[u8],
    ) -> Result<TlsConnectionInfo, Box<dyn std::error::Error + Send + Sync>> {
        analysis::analyze_tls_connection(initial_buffer)
    }

    /// TLS 처리 전략을 결정합니다
    fn determine_tls_strategy(
        &self,
        authority: &Authority,
        tls_info: &TlsConnectionInfo,
    ) -> TlsStrategy {
        analysis::determine_tls_strategy(authority, tls_info, self.tls_config.as_deref())
    }

    /// TLS 버전을 감지하고 적절한 TLS 핸들러를 선택합니다 (Upgraded 스트림 전용)
    pub(crate) async fn handle_tls_connection_upgraded(
        &self,
        authority: &Authority,
        upgraded: Rewind<TokioIo<Upgraded>>,
        initial_buffer: &[u8],
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>> {
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

        // 2단계: 결정적 라이브러리 선택
        let strategy = self.determine_tls_strategy(authority, &tls_info);
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
                info!("✅ [TLS] 핸드셰이크 성공: {} ({:?})", authority, duration);
                emit_tls_event(
                    &self.tls_event_sender,
                    TlsEvent::HandshakeCompleted {
                        authority: authority.clone(),
                        strategy,
                        duration,
                    },
                );
            }
            Err(e) => {
                error!("❌ [TLS] 핸드셰이크 실패: {} - {}", authority, e);
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

        result
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

    /// rustls를 사용하여 TLS 연결을 처리합니다
    #[allow(dead_code)]
    async fn handle_with_rustls<R, W>(
        &self,
        authority: &Authority,
        stream: (R, W),
        initial_data: &[u8],
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (_read_stream, _write_stream) = stream;

        // 내부 버퍼를 사용하여 초기 데이터를 다시 읽을 수 있게 함
        let (client_read, client_write) = tokio::io::duplex(8192);

        // 초기 데이터를 내부 버퍼에 써넣기
        let mut client_write = client_write;
        client_write.write_all(initial_data).await?;
        client_write.flush().await?;
        drop(client_write);

        // Rewind 스트림 생성 - 초기 데이터를 먼저 읽을 수 있게 함
        let rewind_stream =
            Rewind::new(client_read, hyper::body::Bytes::from(initial_data.to_vec()));

        // 서버 설정 생성
        let server_config = self.ca.gen_server_config(authority, None).await;
        let acceptor = TlsAcceptor::from(server_config);

        // TLS 핸드셰이크 수행
        match acceptor.accept(rewind_stream).await {
            Ok(tls_stream) => {
                info!("✅ rustls 핸드셰이크 성공: {}", authority);
                Ok(HybridTlsStream::RustlsGeneric(
                    tokio_rustls::TlsStream::Server(tls_stream),
                ))
            }
            Err(e) => {
                error!("❌ rustls 핸드셰이크 실패: {} - {}", authority, e);
                Err(format!("rustls handshake failed: {}", e).into())
            }
        }
    }

    /// rustls로 Upgraded 스트림을 처리합니다
    async fn handle_with_rustls_upgraded(
        &self,
        authority: &Authority,
        upgraded: Rewind<TokioIo<Upgraded>>,
        _initial_buffer: &[u8],
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>> {
        emit_tls_event(
            &self.tls_event_sender,
            TlsEvent::FakeCertGenerating {
                authority: authority.clone(),
                has_upstream_cert: upstream_cert.is_some(),
            },
        );
        info!("🔧 [RUSTLS] 서버 설정 생성 시작: {}", authority);
        let server_config = self.ca.gen_server_config(authority, upstream_cert).await;
        let acceptor = TlsAcceptor::from(server_config);
        info!("🔧 [RUSTLS] TlsAcceptor 생성 완료: {}", authority);

        info!("🔧 [RUSTLS] TLS 핸드셰이크 시작: {}", authority);
        let start_time = std::time::Instant::now();

        // rustls는 Rewind가 필요하므로 그대로 사용
        match acceptor.accept(upgraded).await {
            Ok(tls_stream) => {
                let duration = start_time.elapsed();
                info!(
                    "✅ [RUSTLS] 핸드셰이크 성공: {} (소요시간: {:?})",
                    authority, duration
                );

                // TLS 연결 정보 로깅
                if let Some(peer_cert) = tls_stream.get_ref().1.peer_certificates() {
                    info!("🔍 [RUSTLS] 피어 인증서 개수: {}", peer_cert.len());
                }

                Ok(HybridTlsStream::Rustls(tokio_rustls::TlsStream::Server(
                    tls_stream,
                )))
            }
            Err(e) => {
                let duration = start_time.elapsed();
                error!(
                    "❌ [RUSTLS] 핸드셰이크 실패: {} (소요시간: {:?})",
                    authority, duration
                );
                error!("❌ [RUSTLS] 오류 상세: {}", e);

                // 오류 타입별 상세 분석
                let error_str = e.to_string();
                if error_str.contains("eof") {
                    error!("🔍 [RUSTLS] EOF 오류 - 클라이언트가 연결을 끊었거나 예상치 못한 종료");
                } else if error_str.contains("alert") {
                    error!("🔍 [RUSTLS] TLS Alert 오류 - 프로토콜 위반 또는 보안 문제");
                } else if error_str.contains("certificate") {
                    error!("🔍 [RUSTLS] 인증서 관련 오류");
                } else if error_str.contains("cipher") {
                    error!("🔍 [RUSTLS] 암호화 스위트 관련 오류");
                } else {
                    error!("🔍 [RUSTLS] 기타 TLS 오류: {}", error_str);
                }

                Err(format!("rustls handshake failed: {}", e).into())
            }
        }
    }

    /// OpenSSL로 Upgraded 스트림을 처리합니다 (개선된 버전 협상)
    #[cfg(feature = "openssl-ca")]
    async fn handle_with_openssl_upgraded(
        &self,
        authority: &Authority,
        upgraded: Rewind<TokioIo<Upgraded>>,
        initial_buffer: &[u8],
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "🔧 [OPENSSL-IMPROVED] 개선된 OpenSSL 처리 시작: {}",
            authority
        );

        // TLS 정보 분석
        let tls_info = self.analyze_tls_connection(initial_buffer)?;
        info!("📊 [OPENSSL-IMPROVED] TLS 정보: {:?}", tls_info);

        // CA에서 OpenSSL 컨텍스트 생성
        emit_tls_event(
            &self.tls_event_sender,
            TlsEvent::FakeCertGenerating {
                authority: authority.clone(),
                has_upstream_cert: upstream_cert.is_some(),
            },
        );
        let ctx = self
            .ca
            .gen_openssl_context(authority, upstream_cert)
            .await?;

        info!("🔧 [OPENSSL-IMPROVED] TLS 핸드셰이크 시작: {}", authority);
        let start_time = std::time::Instant::now();

        // SslStream 생성
        let mut ssl = openssl::ssl::Ssl::new(&ctx)?;

        // 개선된 SSL 설정
        self.configure_ssl_for_connection(&mut ssl, &tls_info, authority)?;

        let mut stream = SslStream::new(ssl, upgraded)?;

        // 연결 유효성 검사
        self.validate_connection_before_handshake(&stream, authority)?;

        info!("🔧 [OPENSSL-IMPROVED] accept() 호출 시작...");

        // 도메인별 핸드셰이크 타임아웃 설정
        let timeout_secs = if let Some(ref tls_config) = self.tls_config {
            tls_config.handshake_timeout(authority.host()).unwrap_or(10)
        } else if authority.as_str().contains("apple.com")
            || authority.as_str().contains("icloud.com")
        {
            15 // Apple 서비스용 15초 (하드코딩 fallback)
        } else {
            10 // 일반 서비스용 10초
        };
        let timeout_duration = std::time::Duration::from_secs(timeout_secs);

        info!(
            "🔧 [OPENSSL-IMPROVED] 핸드셰이크 타임아웃 설정: {:?}",
            timeout_duration
        );

        // 타임아웃과 함께 핸드셰이크 수행
        let handshake_result =
            tokio::time::timeout(timeout_duration, Pin::new(&mut stream).accept()).await;

        match handshake_result {
            Ok(Ok(())) => {
                let duration = start_time.elapsed();
                info!(
                    "✅ [OPENSSL-IMPROVED] 핸드셰이크 성공: {} (소요시간: {:?})",
                    authority, duration
                );

                // 핸드셰이크 성공 후 정보 로깅
                self.log_handshake_success(&stream, authority);

                Ok(HybridTlsStream::OpenSsl(stream))
            }
            Ok(Err(e)) => {
                let duration = start_time.elapsed();
                error!(
                    "❌ [OPENSSL-IMPROVED] 핸드셰이크 실패: {} (소요시간: {:?})",
                    authority, duration
                );

                // 상세한 오류 분석
                self.analyze_handshake_failure(&stream, &e, authority);

                Err(e.into())
            }
            Err(_timeout) => {
                error!("❌ [OPENSSL-IMPROVED] 핸드셰이크 타임아웃: {}", authority);
                Err("TLS handshake timeout".into())
            }
        }
    }

    /// SSL 객체를 연결 특성에 맞게 설정합니다
    #[cfg(feature = "openssl-ca")]
    fn configure_ssl_for_connection(
        &self,
        ssl: &mut openssl::ssl::Ssl,
        tls_info: &TlsConnectionInfo,
        authority: &Authority,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🔧 [SSL-CONFIG] SSL 객체 설정 시작: {}", authority);

        // TlsConfigManager가 있으면 규칙 기반 설정, 없으면 기존 하드코딩 동작
        if let Some(ref tls_config) = self.tls_config {
            let resolved = tls_config.resolve(authority.host());
            let client_cfg = &resolved.client_config;

            // 클라이언트가 요청한 TLS 버전에 맞춰 설정 (클라이언트 요청 버전을 존중)
            self.apply_tls_version_for_client(ssl, tls_info)?;

            // 규칙에서 cipher_list가 지정된 경우 적용
            let is_legacy_tls = matches!(
                tls_info.version,
                TlsVersion::Tls10 | TlsVersion::Ssl30 | TlsVersion::Tls11
            );
            if let Some(ref cipher_list) = client_cfg.cipher_list {
                // 레거시 TLS + 사용자 cipher에 @SECLEVEL=0이 없으면 자동 추가
                let effective_cipher = if is_legacy_tls && !cipher_list.contains("@SECLEVEL=0") {
                    let prefixed = format!("@SECLEVEL=0:{}", cipher_list);
                    info!(
                        "🔧 [SSL-CONFIG] 레거시 TLS용 @SECLEVEL=0 자동 추가: {}",
                        prefixed
                    );
                    prefixed
                } else {
                    cipher_list.clone()
                };
                ssl.set_cipher_list(&effective_cipher)?;
                info!(
                    "🔧 [SSL-CONFIG] 규칙 기반 암호화 스위트 적용: {} (패턴: {:?})",
                    effective_cipher, resolved.matched_pattern
                );
            } else if is_legacy_tls {
                // 레거시 TLS 버전은 SECLEVEL=0이 필요
                ssl.set_cipher_list("@SECLEVEL=0:ALL:!aNULL:!eNULL")?;
                info!("🔧 [SSL-CONFIG] 레거시 TLS용 SECLEVEL=0 적용");
            }

            // 인증서 검증 비활성화
            if resolved.disable_cert_verify {
                ssl.set_verify(openssl::ssl::SslVerifyMode::NONE);
                info!(
                    "🔧 [SSL-CONFIG] 인증서 검증 비활성화 (패턴: {:?})",
                    resolved.matched_pattern
                );
            }
        } else {
            // === 기존 하드코딩 동작 (TlsConfigManager 미설정 시 backward compatible) ===

            // 레거시 TLS 버전은 OpenSSL 3.0+에서 SECLEVEL=0이 필요
            if matches!(
                tls_info.version,
                TlsVersion::Tls10 | TlsVersion::Ssl30 | TlsVersion::Tls11
            ) {
                ssl.set_cipher_list("@SECLEVEL=0:ALL:!aNULL:!eNULL")?;
                info!("🔧 [SSL-CONFIG] 레거시 TLS용 SECLEVEL=0 적용");
            }

            // 클라이언트가 요청한 TLS 버전에 맞춰 설정
            self.apply_tls_version_for_client(ssl, tls_info)?;

            // Apple 서비스용 특별 설정
            let domain = authority.as_str();
            if domain.contains("apple.com") || domain.contains("icloud.com") {
                info!(
                    "🍎 [SSL-CONFIG] Apple 서비스 감지, 특별 설정 적용: {}",
                    domain
                );

                ssl.set_verify(openssl::ssl::SslVerifyMode::NONE);
                info!("🍎 [SSL-CONFIG] Apple 서비스용 인증서 검증 비활성화");

                let apple_ciphers =
                    "ECDHE+AESGCM:ECDHE+CHACHA20:DHE+AESGCM:DHE+CHACHA20:!aNULL:!MD5:!DSS";
                ssl.set_cipher_list(apple_ciphers)?;
                info!(
                    "🍎 [SSL-CONFIG] Apple 서비스용 암호화 스위트 설정: {}",
                    apple_ciphers
                );
            }
        }

        // Apple 특별 암호화 스위트가 있는 경우
        if tls_info.has_apple_cipher {
            info!("🔧 [SSL-CONFIG] Apple 특별 암호화 스위트 감지, Apple 호환성 모드 활성화");
        }

        // SNI가 없는 경우
        if !tls_info.has_sni {
            info!("🔧 [SSL-CONFIG] SNI 없음 감지, SNI 비활성화 모드 활성화");
        }

        info!(
            "✅ [SSL-CONFIG] SSL 객체 설정 완료: {} (버전: {})",
            authority, tls_info.version
        );
        Ok(())
    }

    /// 클라이언트가 요청한 TLS 버전에 맞춰 SSL 객체의 프로토콜 버전을 고정합니다
    #[cfg(feature = "openssl-ca")]
    fn apply_tls_version_for_client(
        &self,
        ssl: &mut openssl::ssl::Ssl,
        tls_info: &TlsConnectionInfo,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match tls_info.version {
            TlsVersion::Tls10 | TlsVersion::Ssl30 => {
                info!("🔧 [SSL-CONFIG] 레거시 TLS 버전 감지, TLS 1.0 고정");
                ssl.set_min_proto_version(Some(openssl::ssl::SslVersion::TLS1))?;
                ssl.set_max_proto_version(Some(openssl::ssl::SslVersion::TLS1))?;
            }
            TlsVersion::Tls11 => {
                info!("🔧 [SSL-CONFIG] TLS 1.1 감지, TLS 1.1 고정");
                ssl.set_min_proto_version(Some(openssl::ssl::SslVersion::TLS1_1))?;
                ssl.set_max_proto_version(Some(openssl::ssl::SslVersion::TLS1_1))?;
            }
            TlsVersion::Tls12 => {
                info!("🔧 [SSL-CONFIG] TLS 1.2 감지, TLS 1.2 고정 (1.3 업그레이드 방지)");
                ssl.set_min_proto_version(Some(openssl::ssl::SslVersion::TLS1_2))?;
                ssl.set_max_proto_version(Some(openssl::ssl::SslVersion::TLS1_2))?;
            }
            TlsVersion::Tls13 => {
                info!("🔧 [SSL-CONFIG] TLS 1.3 감지, TLS 1.3 고정");
                ssl.set_min_proto_version(Some(openssl::ssl::SslVersion::TLS1_3))?;
                ssl.set_max_proto_version(Some(openssl::ssl::SslVersion::TLS1_3))?;
            }
        }
        Ok(())
    }

    /// 핸드셰이크 전 연결 유효성을 검사합니다
    #[cfg(feature = "openssl-ca")]
    fn validate_connection_before_handshake(
        &self,
        stream: &SslStream<Rewind<TokioIo<Upgraded>>>,
        authority: &Authority,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "🔍 [CONNECTION-VALIDATION] 연결 유효성 검사 시작: {}",
            authority
        );

        // SSL 상태 확인
        let state = stream.ssl().state_string();
        info!("  - SSL 상태: {:?}", state);

        // 핸드셰이크 완료 여부 확인
        let is_finished = stream.ssl().is_init_finished();
        info!("  - 핸드셰이크 완료 여부: {}", is_finished);

        if is_finished {
            warn!("⚠️ [CONNECTION-VALIDATION] 핸드셰이크가 이미 완료됨");
        }

        // 인증서 정보 확인
        let has_cert = stream.ssl().certificate().is_some();
        info!("  - 서버 인증서 존재: {}", has_cert);

        info!(
            "✅ [CONNECTION-VALIDATION] 연결 유효성 검사 완료: {}",
            authority
        );
        Ok(())
    }

    /// 핸드셰이크 성공 후 정보를 로깅합니다
    #[cfg(feature = "openssl-ca")]
    fn log_handshake_success(
        &self,
        stream: &SslStream<Rewind<TokioIo<Upgraded>>>,
        authority: &Authority,
    ) {
        info!("📊 [HANDSHAKE-SUCCESS] 핸드셰이크 성공 정보: {}", authority);
        info!("  - 최종 SSL 상태: {:?}", stream.ssl().state_string());
        info!("  - 협상된 TLS 버전: {:?}", stream.ssl().version_str());
        info!(
            "  - 선택된 암호화 스위트: {:?}",
            stream.ssl().current_cipher()
        );

        // 피어 인증서 정보
        if let Some(peer_cert) = stream.ssl().peer_certificate() {
            if let Some(subject) = peer_cert.subject_name().entries().next() {
                info!("  - 피어 인증서 주체: {:?}", subject.data());
            }
        }
    }

    /// 핸드셰이크 실패를 상세 분석합니다
    #[cfg(feature = "openssl-ca")]
    fn analyze_handshake_failure(
        &self,
        stream: &SslStream<Rewind<TokioIo<Upgraded>>>,
        error: &openssl::ssl::Error,
        authority: &Authority,
    ) {
        error!("🔍 [HANDSHAKE-FAILURE] 핸드셰이크 실패 분석: {}", authority);
        error!("  - 실패 시 SSL 상태: {:?}", stream.ssl().state_string());
        error!("  - 에러 코드: {:?}", error.code());
        error!("  - 에러 상세: {}", error);

        // 추가 진단 정보
        error!("🔍 [HANDSHAKE-FAILURE] 진단 정보:");
        error!(
            "  - 핸드셰이크 완료 여부: {}",
            stream.ssl().is_init_finished()
        );
        error!("  - 현재 TLS 버전: {:?}", stream.ssl().version_str());
        error!("  - 암호화 스위트: {:?}", stream.ssl().current_cipher());

        // OpenSSL 에러 큐 확인
        let error_stack = openssl::error::ErrorStack::get();
        if error_stack.errors().len() > 0 {
            error!("  - OpenSSL 에러 큐: {:?}", error_stack);
        }

        // 특정 오류 타입별 분석
        let error_code = error.code();
        error!("🔍 [HANDSHAKE-FAILURE] 오류 코드: {:?}", error_code);

        // 오류 메시지 기반 분석
        let error_msg = error.to_string().to_lowercase();
        if error_msg.contains("eof") || error_msg.contains("unexpected eof") {
            error!("🔍 [HANDSHAKE-FAILURE] EOF 오류 - 클라이언트 연결 종료 또는 네트워크 문제");
        } else if error_msg.contains("ssl") || error_msg.contains("protocol") {
            error!(
                "🔍 [HANDSHAKE-FAILURE] SSL 프로토콜 오류 - 버전 불일치 또는 암호화 스위트 문제"
            );
        } else if error_msg.contains("syscall") || error_msg.contains("network") {
            error!("🔍 [HANDSHAKE-FAILURE] 시스템 호출 오류 - 네트워크 또는 I/O 문제");
        } else {
            error!("🔍 [HANDSHAKE-FAILURE] 기타 SSL 오류: {}", error_msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 최소 유효한 TLS 1.2 ClientHello를 생성하는 헬퍼
    /// 구조: record_hdr(5) + handshake_hdr(4) + client_version(2) + random(32) +
    ///       session_id_len(1) + cipher_suites_len(2) + cipher_suites(N*2) +
    ///       compression_len(1) + compression(1) + extensions_len(2) + extensions(...)
    fn build_client_hello(
        client_version: [u8; 2],
        cipher_suites: &[u16],
        extensions: &[(u16, &[u8])], // (type, data)
    ) -> Vec<u8> {
        let mut body = Vec::new();

        // client_version
        body.extend_from_slice(&client_version);
        // random (32 bytes)
        body.extend_from_slice(&[0u8; 32]);
        // session_id_length = 0
        body.push(0);
        // cipher suites
        let cs_len = (cipher_suites.len() * 2) as u16;
        body.extend_from_slice(&cs_len.to_be_bytes());
        for &cs in cipher_suites {
            body.extend_from_slice(&cs.to_be_bytes());
        }
        // compression methods: 1 method (null)
        body.push(1);
        body.push(0);

        // extensions
        let mut ext_buf = Vec::new();
        for &(ext_type, ext_data) in extensions {
            ext_buf.extend_from_slice(&ext_type.to_be_bytes());
            ext_buf.extend_from_slice(&(ext_data.len() as u16).to_be_bytes());
            ext_buf.extend_from_slice(ext_data);
        }
        let ext_len = ext_buf.len() as u16;
        body.extend_from_slice(&ext_len.to_be_bytes());
        body.extend_from_slice(&ext_buf);

        // Handshake header: type=ClientHello(0x01), length=body.len() (3 bytes)
        let hs_len = body.len() as u32;
        let mut handshake = vec![0x01];
        handshake.push((hs_len >> 16) as u8);
        handshake.push((hs_len >> 8) as u8);
        handshake.push(hs_len as u8);
        handshake.extend_from_slice(&body);

        // TLS record header: type=Handshake(0x16), version=TLS 1.0(0x03,0x01), length
        let record_len = handshake.len() as u16;
        let mut record = vec![0x16, 0x03, 0x01];
        record.extend_from_slice(&record_len.to_be_bytes());
        record.extend_from_slice(&handshake);

        record
    }

    // ─── TLS 전략 단위 테스트 ───

    #[test]
    fn test_tls10_routes_to_openssl_only() {
        let buf = build_client_hello([0x03, 0x01], &[0x002f], &[]);
        let info = analyze_tls_connection(&buf).unwrap();
        assert_eq!(info.version, TlsVersion::Tls10);

        let authority: Authority = "example.com:443".parse().unwrap();
        let strategy = determine_tls_strategy(&authority, &info, None);
        assert_eq!(strategy, TlsStrategy::OpenSslOnly);
    }

    #[test]
    fn test_tls11_routes_to_openssl_only() {
        let buf = build_client_hello([0x03, 0x02], &[0x002f], &[]);
        let info = analyze_tls_connection(&buf).unwrap();
        assert_eq!(info.version, TlsVersion::Tls11);

        let authority: Authority = "example.com:443".parse().unwrap();
        let strategy = determine_tls_strategy(&authority, &info, None);
        assert_eq!(strategy, TlsStrategy::OpenSslOnly);
    }

    #[test]
    fn test_ssl30_routes_to_openssl_only() {
        let buf = build_client_hello([0x03, 0x00], &[0x002f], &[]);
        let info = analyze_tls_connection(&buf).unwrap();
        assert_eq!(info.version, TlsVersion::Ssl30);

        let authority: Authority = "example.com:443".parse().unwrap();
        let strategy = determine_tls_strategy(&authority, &info, None);
        assert_eq!(strategy, TlsStrategy::OpenSslOnly);
    }

    #[test]
    fn test_tls12_normal_domain_routes_to_rustls() {
        // SNI extension present (type 0x0000)
        let sni_data = b"\x00\x00\x0eexample.com";
        let buf = build_client_hello(
            [0x03, 0x03],
            &[0x1301, 0x1302, 0xc02c],
            &[(0x0000, sni_data)],
        );
        let info = analyze_tls_connection(&buf).unwrap();
        assert_eq!(info.version, TlsVersion::Tls12);
        assert!(info.has_sni);

        let authority: Authority = "example.com:443".parse().unwrap();
        let strategy = determine_tls_strategy(&authority, &info, None);
        assert_eq!(strategy, TlsStrategy::RustlsOnly);
    }

    #[test]
    fn test_apple_domain_routes_normally() {
        // 터널 모드 제거 후 Apple 도메인도 일반 전략으로 처리
        let sni_data = b"\x00\x00\x15gateway.icloud.com";
        let buf = build_client_hello([0x03, 0x03], &[0x1301], &[(0x0000, sni_data)]);
        let info = analyze_tls_connection(&buf).unwrap();

        let authority: Authority = "gateway.icloud.com:443".parse().unwrap();
        let strategy = determine_tls_strategy(&authority, &info, None);
        // TLS 자동 바이패스가 터널 역할을 대체
        assert!(matches!(
            strategy,
            TlsStrategy::OpenSslOnly | TlsStrategy::RustlsOnly
        ));
    }

    #[test]
    fn test_apple_cipher_routes_to_openssl_only() {
        // 0xcaca is Apple's GREASE cipher suite marker
        let sni_data = b"\x00\x00\x0eexample.com";
        let buf = build_client_hello(
            [0x03, 0x03],
            &[0xcaca, 0x1301, 0xc02c],
            &[(0x0000, sni_data)],
        );
        let info = analyze_tls_connection(&buf).unwrap();
        assert!(info.has_apple_cipher);

        let authority: Authority = "example.com:443".parse().unwrap();
        let strategy = determine_tls_strategy(&authority, &info, None);
        assert_eq!(strategy, TlsStrategy::OpenSslOnly);
    }

    #[test]
    fn test_no_sni_high_complexity_routes_to_openssl() {
        // Build a buffer with many cipher suites, many extensions, large size, no SNI
        // This should give a high complexity score
        let many_ciphers: Vec<u16> = (0x0001..=0x0019).collect(); // 25 ciphers → +3
        let mut dummy_extensions = Vec::new();
        for i in 1u16..=12 {
            // 12 extensions → +3, no SNI → +2
            dummy_extensions.push((i, &[][..]));
        }
        let buf = build_client_hello([0x03, 0x03], &many_ciphers, &dummy_extensions);
        let info = analyze_tls_connection(&buf).unwrap();
        assert!(!info.has_sni);
        assert!(info.complexity_score >= 6);

        let authority: Authority = "some-server.com:443".parse().unwrap();
        let strategy = determine_tls_strategy(&authority, &info, None);
        assert_eq!(strategy, TlsStrategy::OpenSslOnly);
    }

    // ─── ClientHello 파싱 테스트 ───

    #[test]
    fn test_valid_tls12_client_hello_parses() {
        let sni_data = b"\x00\x00\x0eexample.com";
        let buf = build_client_hello(
            [0x03, 0x03],
            &[0x1301, 0xc02c],
            &[(0x0000, sni_data), (0x000d, &[0x00, 0x02, 0x04, 0x03])],
        );
        let info = analyze_tls_connection(&buf).unwrap();
        assert_eq!(info.version, TlsVersion::Tls12);
        assert!(info.has_sni);
        assert_eq!(info.cipher_suites.len(), 2);
        assert_eq!(info.extensions.len(), 2);
    }

    #[test]
    fn test_short_buffer_returns_error() {
        let buf = vec![0x16, 0x03, 0x01, 0x00];
        let result = analyze_tls_connection(&buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_http_data_returns_error() {
        let buf = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = analyze_tls_connection(buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("TLS not detected"));
    }

    #[test]
    fn test_unknown_tls_version_returns_error() {
        // Use version bytes [0x03, 0x05] which is not a known TLS version
        let buf = build_client_hello([0x03, 0x05], &[0x002f], &[]);
        let result = analyze_tls_connection(&buf);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown TLS version")
        );
    }

    #[test]
    fn test_openssl_required_domain() {
        let auth: Authority = "api2.cursor.sh:443".parse().unwrap();
        assert!(is_openssl_required_domain(&auth));

        let auth: Authority = "google.com:443".parse().unwrap();
        assert!(!is_openssl_required_domain(&auth));
    }
}
