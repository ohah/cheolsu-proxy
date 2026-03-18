use http::Uri;
use hyper::rt::{Read, ReadBufCursor, Write};
use hyper_util::client::legacy::connect::{Connected, Connection};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tower::Service;
use tracing::{debug, error};

/// TCP 연결 타임아웃
const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Upstream proxy 에러 타입
#[derive(Debug, thiserror::Error)]
pub enum UpstreamProxyError {
    #[error("연결 실패: {0}")]
    Connect(#[from] std::io::Error),

    #[error("Upstream proxy CONNECT 실패: {0}")]
    ConnectTunnel(String),

    #[error("스트림 재조립 실패: {0}")]
    Reunite(#[from] tokio::net::tcp::ReuniteError),

    #[error("연결 타임아웃 ({0:?} 초과)")]
    Timeout(Duration),
}

/// Upstream proxy 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamProxyConfig {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub auth: Option<UpstreamProxyAuth>,
    #[serde(default)]
    pub bypass: Vec<String>,
}

/// Upstream proxy 인증 정보
#[derive(Clone, Serialize, Deserialize)]
pub struct UpstreamProxyAuth {
    pub username: String,
    pub password: String,
}

impl fmt::Debug for UpstreamProxyAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UpstreamProxyAuth")
            .field("username", &self.username)
            .field("password", &"********")
            .finish()
    }
}

impl UpstreamProxyConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn should_bypass(&self, host: &str) -> bool {
        self.bypass.iter().any(|pattern| {
            if pattern == "localhost" {
                host == "localhost" || host == "127.0.0.1" || host == "::1"
            } else if pattern.starts_with("*.") {
                host.ends_with(&pattern[1..]) || host == &pattern[2..]
            } else if pattern.starts_with('*') {
                host.ends_with(&pattern[1..])
            } else {
                host == pattern
            }
        })
    }
}

// ---------------------------------------------------------------------------
// TCP 연결 헬퍼
// ---------------------------------------------------------------------------

/// 타임아웃이 적용된 TCP 연결
async fn tcp_connect_with_timeout(
    addr: impl tokio::net::ToSocketAddrs,
) -> Result<TcpStream, UpstreamProxyError> {
    tokio::time::timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect(addr))
        .await
        .map_err(|_| UpstreamProxyError::Timeout(TCP_CONNECT_TIMEOUT))?
        .map_err(UpstreamProxyError::Connect)
}

// ---------------------------------------------------------------------------
// TCP 연결 헬퍼 (CONNECT 터널용)
// ---------------------------------------------------------------------------

/// 대상 서버에 TCP 연결. upstream proxy가 있으면 CONNECT 터널을 통해 연결.
pub async fn connect_to_target(
    target: &str,
    upstream: Option<&UpstreamProxyConfig>,
) -> Result<TcpStream, UpstreamProxyError> {
    let host = target.split(':').next().unwrap_or(target);

    match upstream {
        Some(config) if !config.should_bypass(host) => {
            debug!(
                target,
                upstream = %config.address(),
                "Upstream proxy를 통해 연결"
            );
            connect_via_upstream(config, target).await
        }
        _ => tcp_connect_with_timeout(target).await,
    }
}

/// Upstream proxy에 CONNECT 터널을 생성하여 대상에 연결
async fn connect_via_upstream(
    config: &UpstreamProxyConfig,
    target: &str,
) -> Result<TcpStream, UpstreamProxyError> {
    let stream = tcp_connect_with_timeout(config.address())
        .await
        .inspect_err(|e| {
            error!(
                upstream = %config.address(),
                error = %e,
                "Upstream proxy 연결 실패"
            );
        })?;

    let mut connect_req = format!("CONNECT {} HTTP/1.1\r\nHost: {}\r\n", target, target);

    if let Some(auth) = &config.auth {
        use base64::Engine;
        let credentials = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", auth.username, auth.password));
        connect_req.push_str(&format!("Proxy-Authorization: Basic {}\r\n", credentials));
    }

    connect_req.push_str("\r\n");

    let (reader, mut writer) = stream.into_split();
    writer.write_all(connect_req.as_bytes()).await?;

    // BufReader로 응답 헤더를 줄 단위로 읽기
    let mut buf_reader = BufReader::new(reader);
    let mut status_line = String::new();
    buf_reader.read_line(&mut status_line).await?;

    // 나머지 헤더 소비 (\r\n\r\n 까지)
    let mut header_line = String::new();
    loop {
        header_line.clear();
        let n = buf_reader.read_line(&mut header_line).await?;
        if n == 0 || header_line == "\r\n" {
            break;
        }
    }

    if !status_line.contains(" 200 ") {
        return Err(UpstreamProxyError::ConnectTunnel(
            status_line.trim().to_string(),
        ));
    }

    // BufReader의 내부 버퍼에 남은 데이터가 없으므로 stream을 재조립
    let reader = buf_reader.into_inner();
    let stream = reader.reunite(writer)?;

    debug!(target, "Upstream proxy CONNECT 터널 수립 완료");
    Ok(stream)
}

// ---------------------------------------------------------------------------
// hyper 클라이언트용 커스텀 커넥터
// ---------------------------------------------------------------------------

/// TcpStream 래퍼 — hyper Connection trait 구현
pub struct UpstreamStream(TokioIo<TcpStream>);

impl Connection for UpstreamStream {
    fn connected(&self) -> Connected {
        Connected::new()
    }
}

impl Read for UpstreamStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl Write for UpstreamStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

impl Unpin for UpstreamStream {}

/// Upstream proxy를 경유하는 HTTP 커넥터
///
/// `watch::Receiver`를 통해 런타임에 upstream 설정이 변경되면 즉시 반영됩니다.
#[derive(Clone)]
pub struct ProxyHttpConnector {
    upstream: Arc<watch::Receiver<Option<UpstreamProxyConfig>>>,
}

impl ProxyHttpConnector {
    pub fn new(upstream_rx: watch::Receiver<Option<UpstreamProxyConfig>>) -> Self {
        Self {
            upstream: Arc::new(upstream_rx),
        }
    }
}

impl Service<Uri> for ProxyHttpConnector {
    type Response = UpstreamStream;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let upstream = self.upstream.borrow().clone();

        Box::pin(async move {
            let host = uri.host().unwrap_or("");
            let is_https = uri.scheme_str() == Some("https");
            let port = uri.port_u16().unwrap_or(if is_https { 443 } else { 80 });

            let should_proxy = match &upstream {
                Some(config) => !config.should_bypass(host),
                None => false,
            };

            let stream = if let (true, Some(config)) = (should_proxy, upstream.as_ref()) {
                if is_https {
                    // HTTPS: CONNECT 터널을 통해 연결
                    let target = format!("{}:{}", host, port);
                    connect_via_upstream(config, &target).await?
                } else {
                    // HTTP: upstream proxy에 직접 연결 (hyper가 full URL로 요청 전송)
                    tcp_connect_with_timeout(config.address()).await?
                }
            } else {
                // 직접 연결 (바이패스 또는 upstream 미설정)
                let addr = format!("{}:{}", host, port);
                tcp_connect_with_timeout(&addr).await?
            };

            Ok(UpstreamStream(TokioIo::new(stream)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypass_localhost() {
        let config = UpstreamProxyConfig {
            host: "proxy.company.com".into(),
            port: 8080,
            auth: None,
            bypass: vec!["localhost".into()],
        };
        assert!(config.should_bypass("localhost"));
        assert!(config.should_bypass("127.0.0.1"));
        assert!(config.should_bypass("::1"));
        assert!(!config.should_bypass("example.com"));
    }

    #[test]
    fn bypass_wildcard() {
        let config = UpstreamProxyConfig {
            host: "proxy.company.com".into(),
            port: 8080,
            auth: None,
            bypass: vec!["*.internal.com".into(), "10.0.0.1".into()],
        };
        assert!(config.should_bypass("api.internal.com"));
        assert!(config.should_bypass("internal.com"));
        assert!(config.should_bypass("10.0.0.1"));
        assert!(!config.should_bypass("external.com"));
    }

    #[test]
    fn no_bypass_when_empty() {
        let config = UpstreamProxyConfig {
            host: "proxy.company.com".into(),
            port: 8080,
            auth: None,
            bypass: vec![],
        };
        assert!(!config.should_bypass("anything.com"));
    }

    #[test]
    fn address_format() {
        let config = UpstreamProxyConfig {
            host: "proxy.example.com".into(),
            port: 3128,
            auth: None,
            bypass: vec![],
        };
        assert_eq!(config.address(), "proxy.example.com:3128");
    }

    #[tokio::test]
    async fn tcp_connect_timeout_on_unreachable() {
        // RFC 5737 TEST-NET: 라우팅 불가 주소로 타임아웃 검증
        let result = tcp_connect_with_timeout("192.0.2.1:1").await;
        assert!(
            matches!(&result, Err(UpstreamProxyError::Timeout(_)))
                || matches!(&result, Err(UpstreamProxyError::Connect(_))),
            "unreachable 주소에 대해 Timeout 또는 Connect 에러가 발생해야 함: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn tcp_connect_error_on_refused() {
        // localhost의 미사용 포트로 연결 거부 검증
        let result = tcp_connect_with_timeout("127.0.0.1:1").await;
        assert!(
            matches!(result, Err(UpstreamProxyError::Connect(_))),
            "연결 거부 시 Connect 에러가 발생해야 함"
        );
    }
}
