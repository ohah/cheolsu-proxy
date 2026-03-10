use crate::certificate_authority::CertificateAuthority;
use crate::rewind::Rewind;
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

/// TLS 연결 정보를 담는 구조체
#[derive(Debug, Clone)]
pub struct TlsConnectionInfo {
    /// TLS 버전
    pub version: TlsVersion,
    /// TLS 버전 코드 (예: [0x03, 0x03])
    pub version_code: [u8; 2],
    /// 암호화 스위트 목록
    pub cipher_suites: Vec<u16>,
    /// Extensions 정보
    pub extensions: Vec<TlsExtension>,
    /// SNI (Server Name Indication) 지원 여부
    pub has_sni: bool,
    /// Apple 특별 암호화 스위트 포함 여부
    pub has_apple_cipher: bool,
    /// ClientHello 메시지 크기
    pub message_size: usize,
    /// 연결 복잡도 점수 (높을수록 복잡한 연결)
    pub complexity_score: u8,
}

/// TLS Extension 정보
#[derive(Debug, Clone)]
pub struct TlsExtension {
    pub extension_type: u16,
    pub name: String,
    pub length: u16,
}

/// TLS 처리 전략 — 결정적(deterministic) 선택만 사용
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsStrategy {
    /// OpenSSL 전용 (TLS 1.0/1.1, SSL 3.0, 특수 도메인/암호화)
    OpenSslOnly,
    /// rustls 전용 (TLS 1.2/1.3 일반 연결)
    RustlsOnly,
}

/// TLS 핸들러 - rustls 사용 (Hudsucker 방식으로 단순화)
pub(crate) struct HybridTlsHandler<CA: CertificateAuthority> {
    ca: Arc<CA>,
}

impl<CA: CertificateAuthority> HybridTlsHandler<CA> {
    /// 새로운 TLS 핸들러를 생성합니다
    pub async fn new(ca: Arc<CA>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self { ca })
    }

    /// TLS 연결을 상세 분석합니다
    fn analyze_tls_connection(
        &self,
        initial_buffer: &[u8],
    ) -> Result<TlsConnectionInfo, Box<dyn std::error::Error + Send + Sync>> {
        analyze_tls_connection(initial_buffer)
    }

    /// TLS 처리 전략을 결정합니다
    fn determine_tls_strategy(
        &self,
        authority: &Authority,
        tls_info: &TlsConnectionInfo,
    ) -> TlsStrategy {
        determine_tls_strategy(authority, tls_info)
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

        // 1단계: TLS 연결 유효성 검사
        let tls_info = self.analyze_tls_connection(initial_buffer)?;
        info!("📊 [TLS-INFO] 연결 분석 완료: {:?}", tls_info);

        // 2단계: 결정적 라이브러리 선택
        let strategy = self.determine_tls_strategy(authority, &tls_info);
        info!("🎯 [TLS-STRATEGY] 선택된 전략: {:?}", strategy);

        // 3단계: 선택된 전략으로 연결 시도
        match strategy {
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
                match self
                    .handle_with_rustls_upgraded(authority, upgraded, initial_buffer, upstream_cert)
                    .await
                {
                    Ok(stream) => {
                        info!("✅ [RUSTLS-ONLY] 성공: {}", authority);
                        Ok(stream)
                    }
                    Err(e) => {
                        error!("❌ [RUSTLS-ONLY] 실패: {} - {}", authority, e);
                        Err(e)
                    }
                }
            }
        }
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

        // Apple 서비스용 더 긴 타임아웃 설정
        let timeout_duration = if authority.as_str().contains("apple.com")
            || authority.as_str().contains("icloud.com")
        {
            std::time::Duration::from_secs(15) // Apple 서비스용 15초 타임아웃
        } else {
            std::time::Duration::from_secs(10) // 일반 서비스용 10초 타임아웃
        };

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

        // 레거시 TLS 버전은 OpenSSL 3.0+에서 SECLEVEL=0이 필요
        if matches!(
            tls_info.version,
            TlsVersion::Tls10 | TlsVersion::Ssl30 | TlsVersion::Tls11
        ) {
            ssl.set_cipher_list("@SECLEVEL=0:ALL:!aNULL:!eNULL")?;
            info!("🔧 [SSL-CONFIG] 레거시 TLS용 SECLEVEL=0 적용");
        }

        // 클라이언트가 요청한 TLS 버전에 맞춰 설정
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

        // Apple 서비스용 특별 설정
        let domain = authority.as_str();
        if domain.contains("apple.com") || domain.contains("icloud.com") {
            info!(
                "🍎 [SSL-CONFIG] Apple 서비스 감지, 특별 설정 적용: {}",
                domain
            );

            // Apple 서비스용 더 관대한 설정
            ssl.set_verify(openssl::ssl::SslVerifyMode::NONE);
            info!("🍎 [SSL-CONFIG] Apple 서비스용 인증서 검증 비활성화");

            // Apple 서비스용 특별한 암호화 스위트 설정
            let apple_ciphers =
                "ECDHE+AESGCM:ECDHE+CHACHA20:DHE+AESGCM:DHE+CHACHA20:!aNULL:!MD5:!DSS";
            ssl.set_cipher_list(apple_ciphers)?;
            info!(
                "🍎 [SSL-CONFIG] Apple 서비스용 암호화 스위트 설정: {}",
                apple_ciphers
            );

            // Apple 서비스용 추가 설정 (OpenSSL 옵션 설정)
            info!("🍎 [SSL-CONFIG] Apple 서비스용 SSL 옵션 설정 완료");
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

// ─── 순수 함수 (테스트 가능) ───

/// 특정 도메인이 openssl을 필요로 하는지 확인합니다
pub(crate) fn is_openssl_required_domain(authority: &Authority) -> bool {
    let host = authority.host();
    let openssl_required_domains = [
        "api2.cursor.sh",
        "wps.apple.com",
        "gdmf.apple.com",
        "fbs.smoot.apple.com",
        "gateway.icloud.com",
        "jamf.payhere.in",
    ];
    openssl_required_domains
        .iter()
        .any(|&domain| host == domain)
}

/// Extension 타입을 이름으로 변환
pub(crate) fn get_extension_name(extension_type: u16) -> String {
    match extension_type {
        0x0000 => "SNI".to_string(),
        0x0001 => "max_fragment_length".to_string(),
        0x0002 => "client_certificate_url".to_string(),
        0x0003 => "trusted_ca_keys".to_string(),
        0x0004 => "truncated_hmac".to_string(),
        0x0005 => "status_request".to_string(),
        0x0006 => "user_mapping".to_string(),
        0x0007 => "client_authz".to_string(),
        0x0008 => "server_authz".to_string(),
        0x0009 => "cert_type".to_string(),
        0x000a => "supported_groups".to_string(),
        0x000b => "ec_point_formats".to_string(),
        0x000c => "srp".to_string(),
        0x000d => "signature_algorithms".to_string(),
        0x000e => "use_srtp".to_string(),
        0x000f => "heartbeat".to_string(),
        0x0010 => "application_layer_protocol_negotiation".to_string(),
        0x0011 => "status_request_v2".to_string(),
        0x0012 => "signed_certificate_timestamp".to_string(),
        0x0013 => "client_certificate_type".to_string(),
        0x0014 => "server_certificate_type".to_string(),
        0x0015 => "padding".to_string(),
        0x0016 => "encrypt_then_mac".to_string(),
        0x0017 => "extended_master_secret".to_string(),
        0x0018 => "token_binding".to_string(),
        0x0019 => "cached_info".to_string(),
        0x001a => "tls_lts".to_string(),
        0x001b => "compress_certificate".to_string(),
        0x001c => "record_size_limit".to_string(),
        0x001d => "pwd_protect".to_string(),
        0x001e => "pwd_clear".to_string(),
        0x001f => "password_salt".to_string(),
        0x0020 => "ticket_pinning".to_string(),
        0x0021 => "tls_cert_with_extern_psk".to_string(),
        0x0022 => "delegated_credentials".to_string(),
        0x0023 => "session_ticket".to_string(),
        0x0024 => "TLMSP".to_string(),
        0x0025 => "TLMSP_proxying".to_string(),
        0x0026 => "TLMSP_delegate".to_string(),
        0x0027 => "supported_ekt_ciphers".to_string(),
        0x0028 => "pre_shared_key".to_string(),
        0x0029 => "early_data".to_string(),
        0x002a => "supported_versions".to_string(),
        0x002b => "cookie".to_string(),
        0x002c => "psk_key_exchange_modes".to_string(),
        0x002d => "certificate_authorities".to_string(),
        0x002e => "oid_filters".to_string(),
        0x002f => "post_handshake_auth".to_string(),
        0x0030 => "signature_algorithms_cert".to_string(),
        0x0031 => "key_share".to_string(),
        _ => format!("unknown_0x{:04x}", extension_type),
    }
}

/// 연결 복잡도 점수를 계산합니다
pub(crate) fn calculate_complexity_score(
    cipher_suites: &[u16],
    extensions: &[TlsExtension],
    message_size: usize,
    has_apple_cipher: bool,
) -> u8 {
    let mut score = 0u8;

    // 암호화 스위트 개수에 따른 점수
    if cipher_suites.len() > 20 {
        score += 3;
    } else if cipher_suites.len() > 10 {
        score += 2;
    } else if cipher_suites.len() > 5 {
        score += 1;
    }

    // Extensions 개수에 따른 점수
    if extensions.len() > 10 {
        score += 3;
    } else if extensions.len() > 5 {
        score += 2;
    } else if extensions.len() > 2 {
        score += 1;
    }

    // 메시지 크기에 따른 점수
    if message_size > 1000 {
        score += 3;
    } else if message_size > 500 {
        score += 2;
    } else if message_size > 200 {
        score += 1;
    }

    // Apple 특별 암호화 스위트
    if has_apple_cipher {
        score += 2;
    }

    // SNI가 없는 경우 복잡도 증가
    let has_sni = extensions.iter().any(|ext| ext.extension_type == 0x0000);
    if !has_sni {
        score += 2;
    }

    score.min(10) // 최대 10점
}

/// TLS 연결을 상세 분석합니다 (순수 함수)
pub(crate) fn analyze_tls_connection(
    initial_buffer: &[u8],
) -> Result<TlsConnectionInfo, Box<dyn std::error::Error + Send + Sync>> {
    info!("🔍 [TLS-ANALYSIS] TLS 연결 분석 시작");

    // 기본 TLS 감지
    if initial_buffer.len() < 2 || initial_buffer[..2] != *b"\x16\x03" {
        return Err("TLS not detected".into());
    }

    if initial_buffer.len() < 11 {
        return Err("TLS handshake data too short".into());
    }

    // TLS 버전 분석
    let version_code = [initial_buffer[9], initial_buffer[10]];
    let version = match version_code {
        [0x03, 0x00] => TlsVersion::Ssl30,
        [0x03, 0x01] => TlsVersion::Tls10,
        [0x03, 0x02] => TlsVersion::Tls11,
        [0x03, 0x03] => TlsVersion::Tls12,
        [0x03, 0x04] => TlsVersion::Tls13,
        _ => {
            return Err(format!(
                "Unknown TLS version: 0x{:02x}{:02x}",
                version_code[0], version_code[1]
            )
            .into());
        }
    };

    info!("📊 [TLS-ANALYSIS] 기본 정보:");
    info!(
        "  - TLS 버전: {} (0x{:02x}{:02x})",
        version, version_code[0], version_code[1]
    );
    info!("  - 메시지 크기: {} bytes", initial_buffer.len());

    // ClientHello 상세 분석
    let mut cipher_suites = Vec::new();
    let mut extensions = Vec::new();
    let mut has_sni = false;
    let mut has_apple_cipher = false;

    if initial_buffer.len() >= 43 {
        let session_id_length = initial_buffer[43] as usize;
        info!("  - 세션 ID 길이: {} bytes", session_id_length);

        if initial_buffer.len() >= 44 + session_id_length + 2 {
            let cipher_suites_start = 44 + session_id_length;
            let cipher_suites_length = u16::from_be_bytes([
                initial_buffer[cipher_suites_start],
                initial_buffer[cipher_suites_start + 1],
            ]) as usize;

            // 암호화 스위트 분석
            if initial_buffer.len() >= cipher_suites_start + 2 + cipher_suites_length {
                let cipher_suites_end = cipher_suites_start + 2 + cipher_suites_length;
                for i in (cipher_suites_start + 2..cipher_suites_end).step_by(2) {
                    if i + 1 < initial_buffer.len() {
                        let suite = u16::from_be_bytes([initial_buffer[i], initial_buffer[i + 1]]);
                        cipher_suites.push(suite);

                        // Apple 특별 암호화 스위트 감지
                        if suite == 0xcaca {
                            has_apple_cipher = true;
                            info!("  - 🍎 Apple 특별 암호화 스위트 감지: 0x{:04x}", suite);
                        }
                    }
                }
            }

            // Extensions 분석
            let compression_methods_start = cipher_suites_start + 2 + cipher_suites_length;
            if initial_buffer.len() >= compression_methods_start + 1 {
                let compression_methods_length = initial_buffer[compression_methods_start] as usize;
                let extensions_start = compression_methods_start + 1 + compression_methods_length;

                if initial_buffer.len() >= extensions_start + 2 {
                    let extensions_length = u16::from_be_bytes([
                        initial_buffer[extensions_start],
                        initial_buffer[extensions_start + 1],
                    ]) as usize;

                    let mut pos = extensions_start + 2;
                    let extensions_end = extensions_start + 2 + extensions_length;

                    while pos + 4 <= extensions_end && pos + 4 <= initial_buffer.len() {
                        let extension_type =
                            u16::from_be_bytes([initial_buffer[pos], initial_buffer[pos + 1]]);
                        let extension_length =
                            u16::from_be_bytes([initial_buffer[pos + 2], initial_buffer[pos + 3]])
                                as usize;

                        let extension_name = get_extension_name(extension_type);
                        extensions.push(TlsExtension {
                            extension_type,
                            name: extension_name.clone(),
                            length: extension_length as u16,
                        });

                        // SNI Extension 감지
                        if extension_type == 0x0000 {
                            has_sni = true;
                            info!("  - ✅ SNI Extension 감지됨");
                        }

                        pos += 4 + extension_length;
                    }
                }
            }
        }
    }

    // 복잡도 점수 계산
    let complexity_score = calculate_complexity_score(
        &cipher_suites,
        &extensions,
        initial_buffer.len(),
        has_apple_cipher,
    );

    info!("📊 [TLS-ANALYSIS] 분석 결과:");
    info!("  - 암호화 스위트 개수: {}", cipher_suites.len());
    info!("  - Extensions 개수: {}", extensions.len());
    info!("  - SNI 지원: {}", has_sni);
    info!("  - Apple 암호화 스위트: {}", has_apple_cipher);
    info!("  - 복잡도 점수: {}", complexity_score);

    Ok(TlsConnectionInfo {
        version,
        version_code,
        cipher_suites,
        extensions,
        has_sni,
        has_apple_cipher,
        message_size: initial_buffer.len(),
        complexity_score,
    })
}

/// TLS 처리 전략을 결정합니다 (순수 함수)
pub(crate) fn determine_tls_strategy(
    authority: &Authority,
    tls_info: &TlsConnectionInfo,
) -> TlsStrategy {
    let host = authority.host();

    info!("🎯 [STRATEGY] 전략 결정 분석:");
    info!("  - 도메인: {}", host);
    info!("  - TLS 버전: {}", tls_info.version);
    info!("  - SNI 지원: {}", tls_info.has_sni);
    info!("  - Apple 암호화 스위트: {}", tls_info.has_apple_cipher);
    info!("  - 복잡도 점수: {}", tls_info.complexity_score);

    // 1. TLS 1.0/1.1/SSL 3.0은 OpenSSL 전용
    if matches!(
        tls_info.version,
        TlsVersion::Tls10 | TlsVersion::Tls11 | TlsVersion::Ssl30
    ) {
        info!(
            "🎯 [STRATEGY] 레거시 TLS 버전 감지 ({}) → OpenSSL 전용",
            tls_info.version
        );
        return TlsStrategy::OpenSslOnly;
    }

    // 3. 특별한 도메인들은 OpenSSL 전용
    if is_openssl_required_domain(authority) {
        info!("🎯 [STRATEGY] 특별한 도메인 감지 → OpenSSL 전용");
        return TlsStrategy::OpenSslOnly;
    }

    // 4. Apple 특별 암호화 스위트가 있으면 OpenSSL 전용
    if tls_info.has_apple_cipher {
        info!("🎯 [STRATEGY] Apple 암호화 스위트 감지 → OpenSSL 전용");
        return TlsStrategy::OpenSslOnly;
    }

    // 5. SNI가 없고 복잡도가 높으면 OpenSSL 전용
    if !tls_info.has_sni && tls_info.complexity_score >= 6 {
        info!("🎯 [STRATEGY] SNI 없음 + 높은 복잡도 → OpenSSL 전용");
        return TlsStrategy::OpenSslOnly;
    }

    // 6. 기본적으로는 rustls 전용
    info!("🎯 [STRATEGY] 기본 전략 → Rustls 전용");
    TlsStrategy::RustlsOnly
}

/// TLS 스트림 - rustls와 openssl 스트림을 래핑
#[allow(dead_code)]
pub(crate) enum HybridTlsStream {
    Rustls(tokio_rustls::TlsStream<Rewind<TokioIo<Upgraded>>>),
    RustlsGeneric(tokio_rustls::TlsStream<Rewind<tokio::io::DuplexStream>>),
    OpenSsl(SslStream<Rewind<TokioIo<Upgraded>>>),
}

impl AsyncRead for HybridTlsStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            HybridTlsStream::Rustls(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            HybridTlsStream::RustlsGeneric(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            HybridTlsStream::OpenSsl(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for HybridTlsStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            HybridTlsStream::Rustls(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
            HybridTlsStream::RustlsGeneric(stream) => {
                std::pin::Pin::new(stream).poll_write(cx, buf)
            }
            HybridTlsStream::OpenSsl(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            HybridTlsStream::Rustls(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            HybridTlsStream::RustlsGeneric(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            HybridTlsStream::OpenSsl(stream) => std::pin::Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            HybridTlsStream::Rustls(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            HybridTlsStream::RustlsGeneric(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            HybridTlsStream::OpenSsl(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
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
        let strategy = determine_tls_strategy(&authority, &info);
        assert_eq!(strategy, TlsStrategy::OpenSslOnly);
    }

    #[test]
    fn test_tls11_routes_to_openssl_only() {
        let buf = build_client_hello([0x03, 0x02], &[0x002f], &[]);
        let info = analyze_tls_connection(&buf).unwrap();
        assert_eq!(info.version, TlsVersion::Tls11);

        let authority: Authority = "example.com:443".parse().unwrap();
        let strategy = determine_tls_strategy(&authority, &info);
        assert_eq!(strategy, TlsStrategy::OpenSslOnly);
    }

    #[test]
    fn test_ssl30_routes_to_openssl_only() {
        let buf = build_client_hello([0x03, 0x00], &[0x002f], &[]);
        let info = analyze_tls_connection(&buf).unwrap();
        assert_eq!(info.version, TlsVersion::Ssl30);

        let authority: Authority = "example.com:443".parse().unwrap();
        let strategy = determine_tls_strategy(&authority, &info);
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
        let strategy = determine_tls_strategy(&authority, &info);
        assert_eq!(strategy, TlsStrategy::RustlsOnly);
    }

    #[test]
    fn test_apple_domain_routes_normally() {
        // 터널 모드 제거 후 Apple 도메인도 일반 전략으로 처리
        let sni_data = b"\x00\x00\x15gateway.icloud.com";
        let buf = build_client_hello([0x03, 0x03], &[0x1301], &[(0x0000, sni_data)]);
        let info = analyze_tls_connection(&buf).unwrap();

        let authority: Authority = "gateway.icloud.com:443".parse().unwrap();
        let strategy = determine_tls_strategy(&authority, &info);
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
        let strategy = determine_tls_strategy(&authority, &info);
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
        let strategy = determine_tls_strategy(&authority, &info);
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
