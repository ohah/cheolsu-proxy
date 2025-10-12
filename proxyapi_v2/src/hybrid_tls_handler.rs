use crate::certificate_authority::CertificateAuthority;
use crate::rewind::Rewind;
use crate::tls_version_detector::TlsVersionDetector;
use http::uri::Authority;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_rustls::TlsAcceptor;
use tracing::{error, info, warn};

#[cfg(feature = "native-tls-client")]
use tokio_native_tls::{TlsAcceptor as NativeTlsAcceptor, TlsStream as NativeTlsStream};

/// 하이브리드 TLS 핸들러 - TLS 버전에 따라 rustls 또는 OpenSSL 사용
pub struct HybridTlsHandler<CA: CertificateAuthority> {
    ca: Arc<CA>,
    rustls_acceptor: Option<TlsAcceptor>,
    #[cfg(feature = "native-tls-client")]
    native_tls_acceptor: Option<NativeTlsAcceptor>,
}

impl<CA: CertificateAuthority> HybridTlsHandler<CA> {
    /// 새로운 하이브리드 TLS 핸들러를 생성합니다
    pub async fn new(ca: Arc<CA>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // rustls 서버 설정 생성
        let rustls_server_config = ca.gen_server_config(&"localhost".parse().unwrap()).await;
        let rustls_acceptor = Some(TlsAcceptor::from(rustls_server_config));

        #[cfg(feature = "native-tls-client")]
        let native_tls_acceptor = {
            // OpenSSL 기반 TLS 설정 생성 (일단 None으로 설정)
            // 실제 구현에서는 OpenSSL 서버 설정이 필요
            None
        };

        #[cfg(not(feature = "native-tls-client"))]
        let native_tls_acceptor: Option<()> = None;

        Ok(Self {
            ca,
            rustls_acceptor,
            #[cfg(feature = "native-tls-client")]
            native_tls_acceptor,
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

        // TLS 버전 감지
        let tls_version = TlsVersionDetector::detect_tls_version(initial_buffer);

        match tls_version {
            Some(version) => {
                info!(
                    "✅ TLS 버전 감지 성공: {} ({} bytes)",
                    version,
                    initial_buffer.len()
                );
                info!("🔧 버전별 지원 상태:");
                info!(
                    "  - rustls 지원: {}",
                    TlsVersionDetector::is_rustls_supported(version)
                );
                info!(
                    "  - OpenSSL 지원: {}",
                    TlsVersionDetector::is_openssl_supported(version)
                );

                if TlsVersionDetector::is_rustls_supported(version) {
                    info!("✅ [RUSTLS] TLS 연결 시작: {} - {}", version, authority);
                    match self.handle_with_rustls_upgraded(authority, upgraded).await {
                        Ok(stream) => {
                            info!("✅ [RUSTLS] TLS 연결 성공: {} - {}", version, authority);
                            Ok(stream)
                        }
                        Err(e) => {
                            error!(
                                "❌ [RUSTLS] TLS 연결 실패: {} - {} - 오류: {}",
                                version, authority, e
                            );
                            Err(e)
                        }
                    }
                } else {
                    info!("🔧 [NATIVE-TLS] TLS 연결 시작: {} - {}", version, authority);
                    match self
                        .handle_with_native_tls_upgraded(authority, upgraded)
                        .await
                    {
                        Ok(stream) => {
                            info!("✅ [NATIVE-TLS] TLS 연결 성공: {} - {}", version, authority);
                            Ok(stream)
                        }
                        Err(e) => {
                            error!(
                                "❌ [NATIVE-TLS] TLS 연결 실패: {} - {} - 오류: {}",
                                version, authority, e
                            );
                            Err(e)
                        }
                    }
                }
            }
            None => {
                warn!("⚠️ TLS 버전을 감지할 수 없음: {}", authority);
                warn!("📊 버퍼 분석:");
                warn!("  - 버퍼 크기: {} bytes", initial_buffer.len());
                warn!(
                    "  - 첫 바이트: 0x{:02x}",
                    initial_buffer.get(0).unwrap_or(&0)
                );
                if initial_buffer.len() >= 5 {
                    warn!("  - 5번째 바이트: 0x{:02x}", initial_buffer[4]);
                }
                if initial_buffer.len() >= 9 {
                    warn!(
                        "  - 9-10번째 바이트 (TLS 버전): 0x{:02x}{:02x}",
                        initial_buffer[8], initial_buffer[9]
                    );
                }

                warn!(
                    "⚠️ [RUSTLS] TLS 버전을 감지할 수 없음, rustls로 시도: {}",
                    authority
                );
                match self.handle_with_rustls_upgraded(authority, upgraded).await {
                    Ok(stream) => {
                        info!("✅ [RUSTLS] TLS 연결 성공 (버전 감지 실패): {}", authority);
                        Ok(stream)
                    }
                    Err(e) => {
                        error!(
                            "❌ [RUSTLS] TLS 연결 실패 (버전 감지 실패): {} - 오류: {}",
                            authority, e
                        );
                        Err(e)
                    }
                }
            }
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

        // TLS 버전 감지
        let tls_version = TlsVersionDetector::detect_tls_version(&buffer);

        match tls_version {
            Some(version) => {
                info!("🔍 TLS 버전 감지: {} ({} bytes)", version, bytes_read);

                if TlsVersionDetector::is_rustls_supported(version) {
                    info!("✅ rustls 사용: {}", version);
                    self.handle_with_rustls(authority, (read_stream, write_stream), &buffer)
                        .await
                } else {
                    info!("🔧 OpenSSL 사용: {} (rustls 미지원)", version);
                    self.handle_with_openssl(authority, (read_stream, write_stream), &buffer)
                        .await
                }
            }
            None => {
                warn!("⚠️ TLS 버전을 감지할 수 없음, rustls로 시도");
                self.handle_with_rustls(authority, (read_stream, write_stream), &buffer)
                    .await
            }
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

    /// OpenSSL을 사용하여 TLS 연결을 처리합니다
    #[cfg(feature = "native-tls-client")]
    async fn handle_with_openssl<R, W>(
        &self,
        authority: &Authority,
        stream: (R, W),
        initial_data: &[u8],
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        info!("🔧 native-tls로 TLS 연결 처리 시작: {}", authority);

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

        // PKCS12 인증서 생성
        let pkcs12_data = match self.ca.gen_pkcs12_identity(authority).await {
            Some(data) => data,
            None => {
                error!("❌ PKCS12 인증서 생성 실패");
                return Err("Failed to generate PKCS12 certificate".into());
            }
        };

        // native-tls Identity 생성 - PKCS12 대신 PEM 형식 사용 시도
        let identity = match tokio_native_tls::native_tls::Identity::from_pkcs12(&pkcs12_data, "") {
            Ok(identity) => identity,
            Err(e) => {
                error!("❌ native-tls Identity 생성 실패 (PKCS12): {}", e);

                // PKCS12 데이터 디버깅 정보 출력
                error!("❌ PKCS12 데이터 크기: {} bytes", pkcs12_data.len());
                error!(
                    "❌ PKCS12 데이터 헥스 (처음 32 bytes): {:02X?}",
                    &pkcs12_data[..pkcs12_data.len().min(32)]
                );

                // PKCS12 형식이 올바른지 확인
                if pkcs12_data.len() < 4 {
                    error!("❌ PKCS12 데이터가 너무 짧음");
                    return Err("PKCS12 data too short".into());
                }

                // PKCS12 매직 넘버 확인 (0x30 0x82 또는 0x30 0x81)
                let magic = &pkcs12_data[0..2];
                if magic != [0x30, 0x82] && magic != [0x30, 0x81] {
                    error!("❌ PKCS12 매직 넘버가 올바르지 않음: {:02X?}", magic);
                    return Err("Invalid PKCS12 magic number".into());
                }

                // PKCS12 실패 시 다른 방법으로 대체 시도
                info!("🔧 PKCS12 실패, 다른 방법으로 대체 시도");

                // native-tls에서 PKCS12 대신 다른 형식 사용 시도
                // 먼저 PKCS12 데이터를 다시 생성해보기 (패스워드 없음)
                info!("🔧 PKCS12 재생성 시도 (패스워드 없음)");

                // CA에서 새로운 PKCS12 생성
                let new_pkcs12_data = match self.ca.gen_pkcs12_identity(authority).await {
                    Some(data) => data,
                    None => {
                        error!("❌ PKCS12 재생성 실패");
                        return Err("Failed to regenerate PKCS12 certificate".into());
                    }
                };

                // 새로운 PKCS12로 다시 시도
                match tokio_native_tls::native_tls::Identity::from_pkcs12(&new_pkcs12_data, "") {
                    Ok(identity) => {
                        info!("✅ PKCS12 재생성으로 native-tls Identity 생성 성공");
                        identity
                    }
                    Err(e2) => {
                        error!("❌ PKCS12 재생성으로도 실패: {}", e2);
                        error!("❌ 원본 오류: {}", e);
                        return Err(format!(
                            "Failed to create native-tls identity: original={}, retry={}",
                            e, e2
                        )
                        .into());
                    }
                }
            }
        };

        // native-tls TlsAcceptor 생성
        let acceptor = match tokio_native_tls::native_tls::TlsAcceptor::new(identity) {
            Ok(acceptor) => acceptor,
            Err(e) => {
                error!("❌ native-tls TlsAcceptor 생성 실패: {}", e);
                return Err(format!("Failed to create native-tls acceptor: {}", e).into());
            }
        };

        let tokio_acceptor = tokio_native_tls::TlsAcceptor::from(acceptor);

        // TLS 핸드셰이크 수행
        match tokio_acceptor.accept(rewind_stream).await {
            Ok(tls_stream) => {
                info!("✅ native-tls 핸드셰이크 성공: {}", authority);
                Ok(HybridTlsStream::NativeTlsGeneric(tls_stream))
            }
            Err(e) => {
                error!("❌ native-tls 핸드셰이크 실패: {} - {}", authority, e);
                Err(format!("native-tls handshake failed: {}", e).into())
            }
        }
    }

    /// rustls로 Upgraded 스트림을 처리합니다
    async fn handle_with_rustls_upgraded(
        &self,
        authority: &Authority,
        upgraded: Rewind<TokioIo<Upgraded>>,
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>> {
        let server_config = self.ca.gen_server_config(authority).await;
        let acceptor = TlsAcceptor::from(server_config);

        match acceptor.accept(upgraded).await {
            Ok(tls_stream) => {
                info!("✅ rustls 핸드셰이크 성공: {}", authority);
                Ok(HybridTlsStream::Rustls(tokio_rustls::TlsStream::Server(
                    tls_stream,
                )))
            }
            Err(e) => {
                error!("❌ rustls 핸드셰이크 실패: {}", e);
                Err(format!("rustls handshake failed: {}", e).into())
            }
        }
    }

    /// native-tls로 Upgraded 스트림을 처리합니다
    #[cfg(feature = "native-tls-client")]
    async fn handle_with_native_tls_upgraded(
        &self,
        authority: &Authority,
        upgraded: Rewind<TokioIo<Upgraded>>,
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>> {
        info!("🔧 native-tls로 TLS 연결 처리 시작: {}", authority);

        // PKCS12 인증서 생성
        info!("🔧 PKCS12 인증서 생성 시작: {}", authority);
        let pkcs12_data = match self.ca.gen_pkcs12_identity(authority).await {
            Some(data) => {
                info!("✅ PKCS12 인증서 생성 성공: {} bytes", data.len());
                data
            }
            None => {
                error!("❌ PKCS12 인증서 생성 실패");
                return Err("Failed to generate PKCS12 certificate".into());
            }
        };

        // native-tls Identity 생성 - 패스워드 없음으로 시도
        info!("🔧 native-tls Identity 생성 시작 (패스워드 없음)");
        let identity = match tokio_native_tls::native_tls::Identity::from_pkcs12(&pkcs12_data, "") {
            Ok(identity) => {
                info!("✅ native-tls Identity 생성 성공");
                identity
            }
            Err(e) => {
                error!("❌ native-tls Identity 생성 실패 (빈 패스워드): {}", e);

                // PKCS12 데이터 디버깅 정보 출력
                error!("❌ PKCS12 데이터 크기: {} bytes", pkcs12_data.len());
                error!(
                    "❌ PKCS12 데이터 헥스 (처음 32 bytes): {:02X?}",
                    &pkcs12_data[..pkcs12_data.len().min(32)]
                );

                // PKCS12 형식이 올바른지 확인
                if pkcs12_data.len() < 4 {
                    error!("❌ PKCS12 데이터가 너무 짧음");
                    return Err("PKCS12 data too short".into());
                }

                // PKCS12 매직 넘버 확인 (0x30 0x82 또는 0x30 0x81)
                let magic = &pkcs12_data[0..2];
                if magic != [0x30, 0x82] && magic != [0x30, 0x81] {
                    error!("❌ PKCS12 매직 넘버가 올바르지 않음: {:02X?}", magic);
                    return Err("Invalid PKCS12 magic number".into());
                }

                // PKCS12 실패 시 다른 방법으로 대체 시도
                info!("🔧 PKCS12 실패, 다른 방법으로 대체 시도");

                // native-tls에서 PKCS12 대신 다른 형식 사용 시도
                // 먼저 PKCS12 데이터를 다시 생성해보기 (패스워드 없음)
                info!("🔧 PKCS12 재생성 시도 (패스워드 없음)");

                // CA에서 새로운 PKCS12 생성
                let new_pkcs12_data = match self.ca.gen_pkcs12_identity(authority).await {
                    Some(data) => data,
                    None => {
                        error!("❌ PKCS12 재생성 실패");
                        return Err("Failed to regenerate PKCS12 certificate".into());
                    }
                };

                // 새로운 PKCS12로 다시 시도
                match tokio_native_tls::native_tls::Identity::from_pkcs12(&new_pkcs12_data, "") {
                    Ok(identity) => {
                        info!("✅ PKCS12 재생성으로 native-tls Identity 생성 성공");
                        identity
                    }
                    Err(e2) => {
                        error!("❌ PKCS12 재생성으로도 실패: {}", e2);
                        error!("❌ 원본 오류: {}", e);
                        return Err(format!(
                            "Failed to create native-tls identity: original={}, retry={}",
                            e, e2
                        )
                        .into());
                    }
                }
            }
        };

        // native-tls TlsAcceptor 생성
        let acceptor = match tokio_native_tls::native_tls::TlsAcceptor::new(identity) {
            Ok(acceptor) => acceptor,
            Err(e) => {
                error!("❌ native-tls TlsAcceptor 생성 실패: {}", e);
                return Err(format!("Failed to create native-tls acceptor: {}", e).into());
            }
        };

        let tokio_acceptor = tokio_native_tls::TlsAcceptor::from(acceptor);

        // TLS 핸드셰이크 수행
        match tokio_acceptor.accept(upgraded).await {
            Ok(tls_stream) => {
                info!("✅ native-tls 핸드셰이크 성공: {}", authority);
                Ok(HybridTlsStream::NativeTls(tls_stream))
            }
            Err(e) => {
                error!("❌ native-tls 핸드셰이크 실패: {}", e);
                Err(format!("native-tls handshake failed: {}", e).into())
            }
        }
    }

    #[cfg(not(feature = "native-tls-client"))]
    async fn handle_with_native_tls_upgraded(
        &self,
        _authority: &Authority,
        _upgraded: Rewind<TokioIo<Upgraded>>,
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>> {
        error!("native-tls-client feature가 활성화되지 않음");
        Err("native-tls-client feature not enabled".into())
    }

    #[cfg(not(feature = "native-tls-client"))]
    async fn handle_with_openssl<R, W>(
        &self,
        _authority: &Authority,
        _stream: (R, W),
        _initial_data: &[u8],
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        Err("native-tls-client feature not enabled".into())
    }
}

/// 하이브리드 TLS 스트림 - rustls 또는 native-tls 스트림을 래핑
pub enum HybridTlsStream {
    Rustls(tokio_rustls::TlsStream<Rewind<TokioIo<Upgraded>>>),
    RustlsGeneric(tokio_rustls::TlsStream<Rewind<tokio::io::DuplexStream>>),
    #[cfg(feature = "native-tls-client")]
    NativeTls(NativeTlsStream<Rewind<TokioIo<Upgraded>>>),
    #[cfg(feature = "native-tls-client")]
    NativeTlsGeneric(NativeTlsStream<Rewind<tokio::io::DuplexStream>>),
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
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTls(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTlsGeneric(stream) => {
                std::pin::Pin::new(stream).poll_read(cx, buf)
            }
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
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTls(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTlsGeneric(stream) => {
                std::pin::Pin::new(stream).poll_write(cx, buf)
            }
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            HybridTlsStream::Rustls(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            HybridTlsStream::RustlsGeneric(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTls(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTlsGeneric(stream) => std::pin::Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            HybridTlsStream::Rustls(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            HybridTlsStream::RustlsGeneric(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTls(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTlsGeneric(stream) => {
                std::pin::Pin::new(stream).poll_shutdown(cx)
            }
        }
    }
}
