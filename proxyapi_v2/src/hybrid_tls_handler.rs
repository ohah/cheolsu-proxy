use crate::certificate_authority::CertificateAuthority;
use crate::rewind::Rewind;
use crate::tls_version_detector::TlsVersionDetector;
use http::uri::Authority;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
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
        // TLS 버전 감지
        let tls_version = TlsVersionDetector::detect_tls_version(initial_buffer);

        match tls_version {
            Some(version) => {
                info!("🔍 TLS 버전 감지: {}", version);

                if TlsVersionDetector::is_rustls_supported(version) {
                    info!("✅ [RUSTLS] TLS 연결 시작: {} - {}", version, authority);
                    match self.handle_with_rustls_upgraded(authority, upgraded).await {
                        Ok(stream) => {
                            info!("✅ [RUSTLS] TLS 연결 성공: {} - {}", version, authority);
                            Ok(stream)
                        }
                        Err(e) => {
                            error!("❌ [RUSTLS] TLS 연결 실패: {} - {} - 오류: {}", version, authority, e);
                            Err(e)
                        }
                    }
                } else {
                    info!("🔧 [NATIVE-TLS] TLS 연결 시작: {} - {}", version, authority);
                    match self.handle_with_native_tls_upgraded(authority, upgraded).await {
                        Ok(stream) => {
                            info!("✅ [NATIVE-TLS] TLS 연결 성공: {} - {}", version, authority);
                            Ok(stream)
                        }
                        Err(e) => {
                            error!("❌ [NATIVE-TLS] TLS 연결 실패: {} - {} - 오류: {}", version, authority, e);
                            Err(e)
                        }
                    }
                }
            }
            None => {
                warn!("⚠️ [RUSTLS] TLS 버전을 감지할 수 없음, rustls로 시도: {}", authority);
                match self.handle_with_rustls_upgraded(authority, upgraded).await {
                    Ok(stream) => {
                        info!("✅ [RUSTLS] TLS 연결 성공 (버전 감지 실패): {}", authority);
                        Ok(stream)
                    }
                    Err(e) => {
                        error!("❌ [RUSTLS] TLS 연결 실패 (버전 감지 실패): {} - 오류: {}", authority, e);
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
        _initial_data: &[u8],
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (_read_stream, _write_stream) = stream;

        // TODO: 초기 데이터를 다시 스트림에 써넣는 로직 구현 필요
        // 현재는 단순히 rustls로 처리

        let server_config = self.ca.gen_server_config(authority).await;
        let _acceptor = TlsAcceptor::from(server_config);

        // TODO: 실제 구현에서는 스트림을 TcpStream으로 변환하는 로직이 필요
        // 현재는 에러를 반환하여 기존 로직을 사용하도록 함
        error!("하이브리드 TLS 핸들러는 아직 완전히 구현되지 않음");
        Err("Hybrid TLS handler not fully implemented yet".into())
    }

    /// OpenSSL을 사용하여 TLS 연결을 처리합니다
    #[cfg(feature = "native-tls-client")]
    async fn handle_with_openssl<R, W>(
        &self,
        authority: &Authority,
        _stream: (R, W),
        _initial_data: &[u8],
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        info!("🔧 native-tls로 TLS 연결 처리 시작: {}", authority);

        // PKCS12 인증서 생성
        let pkcs12_data = match self.ca.gen_pkcs12_identity(authority).await {
            Some(data) => data,
            None => {
                error!("❌ PKCS12 인증서 생성 실패");
                return Err("Failed to generate PKCS12 certificate".into());
            }
        };

        // native-tls Identity 생성
        let identity = match tokio_native_tls::native_tls::Identity::from_pkcs12(&pkcs12_data, "") {
            Ok(identity) => identity,
            Err(e) => {
                error!("❌ native-tls Identity 생성 실패: {}", e);
                return Err(format!("Failed to create native-tls identity: {}", e).into());
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

        let _tokio_acceptor = tokio_native_tls::TlsAcceptor::from(acceptor);

        // 직접 TLS 핸드셰이크 수행 (generic stream은 native-tls에서 지원하지 않음)
        // TODO: generic 스트림을 TcpStream으로 변환하는 로직 필요
        error!("generic 스트림은 native-tls에서 지원하지 않습니다");
        Err("generic stream not supported by native-tls".into())
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
        let pkcs12_data = match self.ca.gen_pkcs12_identity(authority).await {
            Some(data) => data,
            None => {
                error!("❌ PKCS12 인증서 생성 실패");
                return Err("Failed to generate PKCS12 certificate".into());
            }
        };

        // native-tls Identity 생성
        let identity = match tokio_native_tls::native_tls::Identity::from_pkcs12(&pkcs12_data, "") {
            Ok(identity) => identity,
            Err(e) => {
                error!("❌ native-tls Identity 생성 실패: {}", e);
                return Err(format!("Failed to create native-tls identity: {}", e).into());
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
    #[cfg(feature = "native-tls-client")]
    NativeTls(NativeTlsStream<Rewind<TokioIo<Upgraded>>>),
}

impl AsyncRead for HybridTlsStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            HybridTlsStream::Rustls(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTls(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
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
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTls(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            HybridTlsStream::Rustls(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTls(stream) => std::pin::Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            HybridTlsStream::Rustls(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            #[cfg(feature = "native-tls-client")]
            HybridTlsStream::NativeTls(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
        }
    }
}
