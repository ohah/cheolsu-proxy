use crate::certificate_authority::CertificateAuthority;
use crate::rewind::Rewind;
use crate::tls_event::{TlsEvent, emit_tls_event};
use crate::tls_version_detector::TlsVersion;
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::pin::Pin;
use tokio_openssl::SslStream;
use tracing::{error, info, warn};

use super::HybridTlsHandler;
use super::stream::HybridTlsStream;
use super::types::TlsConnectionInfo;

impl<CA: CertificateAuthority> HybridTlsHandler<CA> {
    /// OpenSSL로 Upgraded 스트림을 처리합니다 (개선된 버전 협상)
    #[cfg(feature = "openssl-ca")]
    pub(super) async fn handle_with_openssl_upgraded(
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
        self.configure_ssl_for_connection(&mut ssl, &tls_info, authority)
            .await?;

        let mut stream = SslStream::new(ssl, upgraded)?;

        // 연결 유효성 검사
        self.validate_connection_before_handshake(&stream, authority)?;

        info!("🔧 [OPENSSL-IMPROVED] accept() 호출 시작...");

        // 도메인별 핸드셰이크 타임아웃 설정
        let timeout_secs = if let Some(ref tls_config) = self.tls_config {
            let guard = tls_config.read().await;
            guard.handshake_timeout(authority.host()).unwrap_or(10)
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
    pub(super) async fn configure_ssl_for_connection(
        &self,
        ssl: &mut openssl::ssl::Ssl,
        tls_info: &TlsConnectionInfo,
        authority: &Authority,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🔧 [SSL-CONFIG] SSL 객체 설정 시작: {}", authority);

        // TlsConfigManager가 있으면 규칙 기반 설정, 없으면 기존 하드코딩 동작
        if let Some(ref tls_config) = self.tls_config {
            let guard = tls_config.read().await;
            let resolved = guard.resolve(authority.host());
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
