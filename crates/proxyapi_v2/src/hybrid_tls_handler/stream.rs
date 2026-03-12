use crate::rewind::Rewind;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_openssl::SslStream;

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
