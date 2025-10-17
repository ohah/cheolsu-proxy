use crate::certificate_authority::CertificateAuthority;
use crate::rewind::Rewind;
use http::uri::Authority;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_openssl::SslStream;
use tokio_rustls::TlsAcceptor;
use tracing::{error, info};

/// TLS 핸들러 - rustls 사용 (Hudsucker 방식으로 단순화)
pub struct HybridTlsHandler<CA: CertificateAuthority> {
    ca: Arc<CA>,
    rustls_acceptor: Option<TlsAcceptor>,
}

impl<CA: CertificateAuthority> HybridTlsHandler<CA> {
    /// 새로운 TLS 핸들러를 생성합니다 (Hudsucker 방식으로 단순화)
    pub async fn new(ca: Arc<CA>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // rustls 서버 설정 생성
        let rustls_server_config = ca.gen_server_config(&"localhost".parse().unwrap()).await;
        let rustls_acceptor = Some(TlsAcceptor::from(rustls_server_config));

        Ok(Self {
            ca,
            rustls_acceptor,
        })
    }

    /// TLS 버전을 감지하고 적절한 TLS 핸들러를 선택합니다 (Upgraded 스트림 전용)
    pub async fn handle_tls_connection_upgraded(
        &self,
        authority: &Authority,
        upgraded: Rewind<TokioIo<Upgraded>>,
        initial_buffer: &[u8],
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>> {
        // TLS 버전 감지 상세 로그
        info!("🔍 TLS 버전 감지 시작: {}", authority);
        info!("📊 초기 버퍼 크기: {} bytes", initial_buffer.len());

        // 초기 버퍼의 첫 16바이트를 hex로 로그
        let hex_preview = if initial_buffer.len() >= 16 {
            format!("{:02x?}", &initial_buffer[..16])
        } else {
            format!("{:02x?}", initial_buffer)
        };
        info!("🔢 초기 버퍼 (hex): {}", hex_preview);

        // Hudsucker 방식: 간단한 TLS 감지
        let is_tls = initial_buffer.len() >= 2 && initial_buffer[..2] == *b"\x16\x03";

        // ClientHello 메시지 상세 분석
        if initial_buffer.len() >= 11 {
            info!("🔍 [CLIENT-HELLO] 상세 분석:");
            info!(
                "  - 레코드 타입: 0x{:02x} ({})",
                initial_buffer[0],
                if initial_buffer[0] == 0x16 {
                    "Handshake"
                } else {
                    "Unknown"
                }
            );
            info!(
                "  - 레코드 버전: 0x{:02x}{:02x}",
                initial_buffer[1], initial_buffer[2]
            );
            info!(
                "  - 레코드 길이: {} bytes",
                u16::from_be_bytes([initial_buffer[3], initial_buffer[4]])
            );
            info!(
                "  - 핸드셰이크 타입: 0x{:02x} ({})",
                initial_buffer[5],
                if initial_buffer[5] == 0x01 {
                    "ClientHello"
                } else {
                    "Unknown"
                }
            );
            info!(
                "  - 핸드셰이크 길이: {} bytes",
                u32::from_be_bytes([0, initial_buffer[6], initial_buffer[7], initial_buffer[8]])
            );
            info!(
                "  - 클라이언트 버전: 0x{:02x}{:02x} ({})",
                initial_buffer[9],
                initial_buffer[10],
                match [initial_buffer[9], initial_buffer[10]] {
                    [0x03, 0x00] => "SSL 3.0",
                    [0x03, 0x01] => "TLS 1.0",
                    [0x03, 0x02] => "TLS 1.1",
                    [0x03, 0x03] => "TLS 1.2",
                    [0x03, 0x04] => "TLS 1.3",
                    _ => "Unknown",
                }
            );

            // ClientHello의 추가 정보 분석 (가능한 경우)
            if initial_buffer.len() >= 43 {
                // Random (32 bytes) + Session ID Length (1 byte)
                let session_id_length = initial_buffer[43] as usize;
                info!("  - 세션 ID 길이: {} bytes", session_id_length);

                if initial_buffer.len() >= 44 + session_id_length + 2 {
                    let cipher_suites_start = 44 + session_id_length;
                    let cipher_suites_length = u16::from_be_bytes([
                        initial_buffer[cipher_suites_start],
                        initial_buffer[cipher_suites_start + 1],
                    ]) as usize;
                    info!("  - 암호화 스위트 길이: {} bytes", cipher_suites_length);
                    info!("  - 암호화 스위트 개수: {}", cipher_suites_length / 2);

                    // 암호화 스위트 목록 분석 (처음 10개만)
                    if initial_buffer.len() >= cipher_suites_start + 2 + cipher_suites_length {
                        let cipher_suites_end = cipher_suites_start + 2 + cipher_suites_length;
                        let mut cipher_suites = Vec::new();
                        for i in (cipher_suites_start + 2..cipher_suites_end).step_by(2) {
                            if i + 1 < initial_buffer.len() {
                                let suite =
                                    u16::from_be_bytes([initial_buffer[i], initial_buffer[i + 1]]);
                                cipher_suites.push(format!("0x{:04x}", suite));
                                if cipher_suites.len() >= 10 {
                                    break;
                                }
                            }
                        }
                        info!("  - 암호화 스위트 (처음 10개): {:?}", cipher_suites);
                    }
                }
            }

            // 전체 ClientHello 메시지 분석을 위한 추가 로깅
            if initial_buffer.len() >= 5 {
                let record_length =
                    u16::from_be_bytes([initial_buffer[3], initial_buffer[4]]) as usize;
                let total_expected_length = 5 + record_length; // 헤더(5) + 레코드 길이

                info!("🔍 [CLIENT-HELLO] 전체 메시지 분석:");
                info!(
                    "  - 예상 전체 길이: {} bytes (헤더 5 + 레코드 {})",
                    total_expected_length, record_length
                );
                info!("  - 현재 읽은 길이: {} bytes", initial_buffer.len());

                if initial_buffer.len() < total_expected_length {
                    info!("  - ⚠️  전체 ClientHello가 아직 완전히 읽히지 않음");
                    info!(
                        "  - 추가로 읽어야 할 바이트: {} bytes",
                        total_expected_length - initial_buffer.len()
                    );
                } else {
                    info!("  - ✅ 전체 ClientHello 메시지 완전히 읽힘");

                    // Extensions 분석
                    if let Some(extensions_info) = analyze_extensions(&initial_buffer) {
                        info!("  - Extensions: {}", extensions_info);
                    }
                }
            }
        }

        if is_tls {
            // TLS 버전에 따라 적절한 핸들러 선택
            let tls_version = if initial_buffer.len() >= 11 {
                match [initial_buffer[9], initial_buffer[10]] {
                    [0x03, 0x00] => "SSL 3.0",
                    [0x03, 0x01] => "TLS 1.0",
                    [0x03, 0x02] => "TLS 1.1",
                    [0x03, 0x03] => "TLS 1.2",
                    [0x03, 0x04] => "TLS 1.3",
                    _ => "Unknown",
                }
            } else {
                "Unknown"
            };

            // TLS 1.0/1.1은 openssl로 처리, TLS 1.2+는 rustls로 처리
            match tls_version {
                #[cfg(feature = "openssl-ca")]
                "TLS 1.0" | "TLS 1.1" => {
                    info!(
                        "🔧 TLS {} 감지됨, openssl로 처리: {}",
                        tls_version, authority
                    );
                    match self
                        .handle_with_openssl_upgraded(authority, upgraded, initial_buffer)
                        .await
                    {
                        Ok(stream) => {
                            info!("✅ [OPENSSL] TLS 연결 성공: {}", authority);
                            Ok(stream)
                        }
                        Err(e) => {
                            error!("❌ [OPENSSL] TLS 연결 실패: {} - 오류: {}", authority, e);
                            Err(e)
                        }
                    }
                }
                #[cfg(not(feature = "openssl-ca"))]
                "TLS 1.0" | "TLS 1.1" => {
                    error!(
                        "❌ TLS 1.0/1.1은 openssl-ca feature가 필요합니다: {}",
                        authority
                    );
                    Err("TLS 1.0/1.1 requires openssl-ca feature".into())
                }
                "TLS 1.2" | "TLS 1.3" => {
                    info!(
                        "🔧 TLS {} 감지됨, rustls로 처리: {}",
                        tls_version, authority
                    );
                    match self
                        .handle_with_rustls_upgraded(authority, upgraded, initial_buffer)
                        .await
                    {
                        Ok(stream) => {
                            info!("✅ [RUSTLS] TLS 연결 성공: {}", authority);
                            Ok(stream)
                        }
                        Err(e) => {
                            error!("❌ [RUSTLS] TLS 연결 실패: {} - 오류: {}", authority, e);
                            Err(e)
                        }
                    }
                }
                _ => {
                    info!("🔧 알 수 없는 TLS 버전, rustls로 처리: {}", authority);
                    match self
                        .handle_with_rustls_upgraded(authority, upgraded, initial_buffer)
                        .await
                    {
                        Ok(stream) => {
                            info!("✅ [RUSTLS] TLS 연결 성공: {}", authority);
                            Ok(stream)
                        }
                        Err(e) => {
                            error!("❌ [RUSTLS] TLS 연결 실패: {} - 오류: {}", authority, e);
                            Err(e)
                        }
                    }
                }
            }
        } else {
            error!("❌ TLS가 감지되지 않음: {}", authority);
            Err("TLS not detected".into())
        }
    }

    /// TLS 버전을 감지하고 적절한 TLS 핸들러를 선택합니다
    pub async fn handle_tls_connection<R, W>(
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
        let server_config = self.ca.gen_server_config(authority).await;
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
        initial_buffer: &[u8],
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>> {
        info!("🔧 [RUSTLS] 서버 설정 생성 시작: {}", authority);
        let server_config = self.ca.gen_server_config(authority).await;
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

    /// OpenSSL로 Upgraded 스트림을 처리합니다 (TLS 1.0/1.1 지원)
    #[cfg(feature = "openssl-ca")]
    async fn handle_with_openssl_upgraded(
        &self,
        authority: &Authority,
        upgraded: Rewind<TokioIo<Upgraded>>,
        _initial_buffer: &[u8],
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>> {
        info!("🔧 [OPENSSL] 서버 설정 생성 시작: {}", authority);

        // CA에서 OpenSSL 컨텍스트 생성
        let ctx = self.ca.gen_openssl_context(authority).await?;

        // OpenSSL Ssl 객체 생성
        let ssl = openssl::ssl::Ssl::new(&ctx)?;

        info!("🔧 [OPENSSL] TLS 핸드셰이크 시작: {}", authority);
        let start_time = std::time::Instant::now();

        // SslStream 생성 (tokio_openssl의 SslStream은 자동으로 핸드셰이크 수행)
        let stream = SslStream::new(ssl, upgraded)?;

        let duration = start_time.elapsed();
        info!(
            "✅ [OPENSSL] 핸드셰이크 성공: {} (소요시간: {:?})",
            authority, duration
        );

        Ok(HybridTlsStream::OpenSsl(stream))
    }
}

/// TLS 스트림 - rustls와 openssl 스트림을 래핑
pub enum HybridTlsStream {
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

/// TLS Extensions 정보를 분석합니다
fn analyze_extensions(buffer: &[u8]) -> Option<String> {
    if buffer.len() < 5 {
        return None;
    }

    let record_length = u16::from_be_bytes([buffer[3], buffer[4]]) as usize;
    if buffer.len() < 5 + record_length {
        return None;
    }

    // ClientHello 메시지 시작 (헤더 5 bytes 건너뛰기)
    let handshake_start = 5;
    if buffer.len() < handshake_start + 9 {
        return None;
    }

    // Handshake 메시지 구조: [type(1), length(3), version(2), random(32), session_id_length(1)]
    let session_id_length = buffer[handshake_start + 4 + 32] as usize;
    let cipher_suites_start = handshake_start + 4 + 32 + 1 + session_id_length;

    if buffer.len() < cipher_suites_start + 2 {
        return None;
    }

    let cipher_suites_length =
        u16::from_be_bytes([buffer[cipher_suites_start], buffer[cipher_suites_start + 1]]) as usize;

    let compression_methods_start = cipher_suites_start + 2 + cipher_suites_length;
    if buffer.len() < compression_methods_start + 1 {
        return None;
    }

    let compression_methods_length = buffer[compression_methods_start] as usize;
    let extensions_start = compression_methods_start + 1 + compression_methods_length;

    if buffer.len() < extensions_start + 2 {
        return None;
    }

    let extensions_length =
        u16::from_be_bytes([buffer[extensions_start], buffer[extensions_start + 1]]) as usize;

    // Extensions 파싱
    let mut pos = extensions_start + 2;
    let extensions_end = extensions_start + 2 + extensions_length;
    let mut extensions = Vec::new();

    while pos + 4 <= extensions_end && pos + 4 <= buffer.len() {
        let extension_type = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]);
        let extension_length = u16::from_be_bytes([buffer[pos + 2], buffer[pos + 3]]) as usize;

        let extension_name = match extension_type {
            0x0000 => "SNI",
            0x0001 => "max_fragment_length",
            0x0002 => "client_certificate_url",
            0x0003 => "trusted_ca_keys",
            0x0004 => "truncated_hmac",
            0x0005 => "status_request",
            0x0006 => "user_mapping",
            0x0007 => "client_authz",
            0x0008 => "server_authz",
            0x0009 => "cert_type",
            0x000a => "supported_groups",
            0x000b => "ec_point_formats",
            0x000c => "srp",
            0x000d => "signature_algorithms",
            0x000e => "use_srtp",
            0x000f => "heartbeat",
            0x0010 => "application_layer_protocol_negotiation",
            0x0011 => "status_request_v2",
            0x0012 => "signed_certificate_timestamp",
            0x0013 => "client_certificate_type",
            0x0014 => "server_certificate_type",
            0x0015 => "padding",
            0x0016 => "encrypt_then_mac",
            0x0017 => "extended_master_secret",
            0x0018 => "token_binding",
            0x0019 => "cached_info",
            0x001a => "tls_lts",
            0x001b => "compress_certificate",
            0x001c => "record_size_limit",
            0x001d => "pwd_protect",
            0x001e => "pwd_clear",
            0x001f => "password_salt",
            0x0020 => "ticket_pinning",
            0x0021 => "tls_cert_with_extern_psk",
            0x0022 => "delegated_credentials",
            0x0023 => "session_ticket",
            0x0024 => "TLMSP",
            0x0025 => "TLMSP_proxying",
            0x0026 => "TLMSP_delegate",
            0x0027 => "supported_ekt_ciphers",
            0x0028 => "pre_shared_key",
            0x0029 => "early_data",
            0x002a => "supported_versions",
            0x002b => "cookie",
            0x002c => "psk_key_exchange_modes",
            0x002d => "certificate_authorities",
            0x002e => "oid_filters",
            0x002f => "post_handshake_auth",
            0x0030 => "signature_algorithms_cert",
            0x0031 => "key_share",
            _ => "unknown",
        };

        extensions.push(format!(
            "{} (0x{:04x}, {} bytes)",
            extension_name, extension_type, extension_length
        ));

        pos += 4 + extension_length;

        // 최대 10개까지만 표시
        if extensions.len() >= 10 {
            break;
        }
    }

    if extensions.is_empty() {
        None
    } else {
        Some(format!(
            "{}개 - {}",
            extensions.len(),
            extensions.join(", ")
        ))
    }
}
