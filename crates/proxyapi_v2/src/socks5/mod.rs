mod protocol;

pub use protocol::*;

use crate::throttle::{self, ThrottleConfig};
use crate::upstream_proxy::{UpstreamProxyConfig, connect_to_target};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio_graceful::Shutdown;
use tracing::{debug, error, info, warn};

/// SOCKS5 인증 설정
#[derive(Debug, Clone)]
pub enum Socks5Auth {
    /// 인증 없음
    NoAuth,
    /// 사용자명/비밀번호 인증 (RFC 1929)
    UsernamePassword { username: String, password: String },
}

/// SOCKS5 프록시 서버 설정
#[derive(Debug, Clone)]
pub struct Socks5Config {
    pub auth: Socks5Auth,
    pub upstream_proxy: Option<UpstreamProxyConfig>,
    pub throttle_rx: Option<Arc<watch::Receiver<Option<ThrottleConfig>>>>,
}

impl Default for Socks5Config {
    fn default() -> Self {
        Self {
            auth: Socks5Auth::NoAuth,
            upstream_proxy: None,
            throttle_rx: None,
        }
    }
}

/// SOCKS5 프록시 서버
pub struct Socks5Server {
    listener: TcpListener,
    config: Arc<Socks5Config>,
}

impl Socks5Server {
    /// 지정된 주소에 바인드하여 SOCKS5 서버를 생성합니다.
    pub async fn bind(addr: SocketAddr, config: Socks5Config) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener,
            config: Arc::new(config),
        })
    }

    /// 이미 바인드된 리스너를 사용하여 SOCKS5 서버를 생성합니다.
    pub fn from_listener(listener: TcpListener, config: Socks5Config) -> Self {
        Self {
            listener,
            config: Arc::new(config),
        }
    }

    /// 서버가 바인드된 로컬 주소를 반환합니다.
    pub fn local_addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.listener.local_addr()
    }

    /// SOCKS5 서버를 시작합니다.
    pub async fn start<F>(self, graceful_shutdown: F) -> Result<(), std::io::Error>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let shutdown = Shutdown::new(graceful_shutdown);
        let guard = shutdown.guard_weak();

        info!(
            addr = %self.listener.local_addr()?,
            "SOCKS5 프록시 서버 시작"
        );

        loop {
            tokio::select! {
                res = self.listener.accept() => {
                    let (stream, client_addr) = match res {
                        Ok(v) => v,
                        Err(e) => {
                            error!("SOCKS5 연결 수락 실패: {}", e);
                            continue;
                        }
                    };

                    let config = Arc::clone(&self.config);
                    shutdown.spawn_task_fn(move |_guard| async move {
                        if let Err(e) = handle_socks5_client(stream, client_addr, &config).await {
                            debug!(
                                client = %client_addr,
                                error = %e,
                                "SOCKS5 클라이언트 처리 실패"
                            );
                        }
                    });
                }
                _ = guard.cancelled() => {
                    break;
                }
            }
        }

        shutdown.shutdown().await;
        info!("SOCKS5 프록시 서버 종료");
        Ok(())
    }
}

/// 개별 SOCKS5 클라이언트 연결을 처리합니다.
pub async fn handle_socks5_client(
    mut stream: TcpStream,
    client_addr: SocketAddr,
    config: &Socks5Config,
) -> Result<(), Socks5Error> {
    debug!(client = %client_addr, "SOCKS5 클라이언트 연결");

    // 1. 핸드셰이크: 클라이언트 인사 (greeting)
    let greeting = read_greeting(&mut stream).await?;

    // 2. 인증 방법 선택
    let selected_method = select_auth_method(&greeting.methods, config);

    // 선택된 인증 방법 응답
    stream
        .write_all(&[SOCKS_VERSION, selected_method])
        .await
        .map_err(Socks5Error::Io)?;

    if selected_method == AUTH_NO_ACCEPTABLE {
        return Err(Socks5Error::NoAcceptableAuthMethod);
    }

    // 3. 인증 수행 (필요한 경우)
    if selected_method == AUTH_USERNAME_PASSWORD {
        authenticate_username_password(&mut stream, config).await?;
    }

    // 4. 요청 처리
    let request = read_request(&mut stream).await?;

    match request.command {
        CMD_CONNECT => {
            handle_connect(&mut stream, &request, config).await?;
        }
        cmd => {
            send_reply(
                &mut stream,
                REPLY_COMMAND_NOT_SUPPORTED,
                &TargetAddr::Ipv4([0, 0, 0, 0], 0),
            )
            .await?;
            return Err(Socks5Error::UnsupportedCommand(cmd));
        }
    }

    Ok(())
}

/// 인증 방법을 선택합니다.
fn select_auth_method(client_methods: &[u8], config: &Socks5Config) -> u8 {
    match &config.auth {
        Socks5Auth::NoAuth => {
            if client_methods.contains(&AUTH_NO_AUTH) {
                AUTH_NO_AUTH
            } else {
                AUTH_NO_ACCEPTABLE
            }
        }
        Socks5Auth::UsernamePassword { .. } => {
            if client_methods.contains(&AUTH_USERNAME_PASSWORD) {
                AUTH_USERNAME_PASSWORD
            } else {
                AUTH_NO_ACCEPTABLE
            }
        }
    }
}

/// 사용자명/비밀번호 인증을 수행합니다 (RFC 1929).
async fn authenticate_username_password(
    stream: &mut TcpStream,
    config: &Socks5Config,
) -> Result<(), Socks5Error> {
    // subnegotiation version (1) + ulen (1)
    let mut header = [0u8; 2];
    stream
        .read_exact(&mut header)
        .await
        .map_err(Socks5Error::Io)?;

    if header[0] != 0x01 {
        return Err(Socks5Error::InvalidSubnegotiationVersion(header[0]));
    }

    let ulen = header[1] as usize;
    let mut username_buf = vec![0u8; ulen];
    stream
        .read_exact(&mut username_buf)
        .await
        .map_err(Socks5Error::Io)?;

    let mut plen_buf = [0u8; 1];
    stream
        .read_exact(&mut plen_buf)
        .await
        .map_err(Socks5Error::Io)?;
    let plen = plen_buf[0] as usize;

    let mut password_buf = vec![0u8; plen];
    stream
        .read_exact(&mut password_buf)
        .await
        .map_err(Socks5Error::Io)?;

    let client_username = String::from_utf8_lossy(&username_buf);
    let client_password = String::from_utf8_lossy(&password_buf);

    let auth_ok = match &config.auth {
        Socks5Auth::UsernamePassword { username, password } => {
            client_username == *username && client_password == *password
        }
        _ => false,
    };

    if auth_ok {
        stream
            .write_all(&[0x01, 0x00])
            .await
            .map_err(Socks5Error::Io)?;
        debug!("SOCKS5 인증 성공");
        Ok(())
    } else {
        stream
            .write_all(&[0x01, 0x01])
            .await
            .map_err(Socks5Error::Io)?;
        Err(Socks5Error::AuthenticationFailed)
    }
}

/// CONNECT 명령을 처리합니다.
async fn handle_connect(
    stream: &mut TcpStream,
    request: &Socks5Request,
    config: &Socks5Config,
) -> Result<(), Socks5Error> {
    let target_str = request.target.to_address_string();
    debug!(target = %target_str, "SOCKS5 CONNECT 요청");

    // 대상 서버에 연결 (upstream proxy가 있으면 경유)
    let target_stream = connect_to_target(&target_str, config.upstream_proxy.as_ref()).await;

    match target_stream {
        Ok(mut target) => {
            // 연결된 대상의 로컬 주소를 바인드 주소로 사용
            let bind_addr = target
                .local_addr()
                .map(|a| TargetAddr::from_socket_addr(a))
                .unwrap_or(TargetAddr::Ipv4([0, 0, 0, 0], 0));

            send_reply(stream, REPLY_SUCCEEDED, &bind_addr).await?;

            // 양방향 데이터 복사 (스로틀링 적용)
            let throttle_config = config
                .throttle_rx
                .as_ref()
                .and_then(|rx| rx.borrow().clone());
            let tunnel_result = if let Some(ref tc) = throttle_config {
                if tc.enabled {
                    throttle::copy_bidirectional_throttled(stream, &mut target, tc).await
                } else {
                    tokio::io::copy_bidirectional(stream, &mut target).await
                }
            } else {
                tokio::io::copy_bidirectional(stream, &mut target).await
            };
            match tunnel_result {
                Ok((client_to_server, server_to_client)) => {
                    debug!(client_to_server, server_to_client, "SOCKS5 터널 종료");
                }
                Err(e) => {
                    debug!("SOCKS5 터널링 에러: {}", e);
                }
            }
        }
        Err(e) => {
            warn!(
                target = %target_str,
                error = %e,
                "SOCKS5 대상 연결 실패"
            );
            send_reply(
                stream,
                REPLY_HOST_UNREACHABLE,
                &TargetAddr::Ipv4([0, 0, 0, 0], 0),
            )
            .await?;
        }
    }

    Ok(())
}

/// 클라이언트 인사 메시지를 읽습니다.
async fn read_greeting(stream: &mut TcpStream) -> Result<Socks5Greeting, Socks5Error> {
    let mut header = [0u8; 2];
    stream
        .read_exact(&mut header)
        .await
        .map_err(Socks5Error::Io)?;

    if header[0] != SOCKS_VERSION {
        return Err(Socks5Error::InvalidVersion(header[0]));
    }

    let nmethods = header[1] as usize;
    if nmethods == 0 {
        return Err(Socks5Error::NoMethodsProvided);
    }

    let mut methods = vec![0u8; nmethods];
    stream
        .read_exact(&mut methods)
        .await
        .map_err(Socks5Error::Io)?;

    Ok(Socks5Greeting { methods })
}

/// 클라이언트 요청을 읽습니다.
async fn read_request(stream: &mut TcpStream) -> Result<Socks5Request, Socks5Error> {
    let mut header = [0u8; 4];
    stream
        .read_exact(&mut header)
        .await
        .map_err(Socks5Error::Io)?;

    if header[0] != SOCKS_VERSION {
        return Err(Socks5Error::InvalidVersion(header[0]));
    }

    let command = header[1];
    // header[2] is reserved
    let atyp = header[3];

    let target = match atyp {
        ATYP_IPV4 => {
            let mut addr = [0u8; 4];
            stream
                .read_exact(&mut addr)
                .await
                .map_err(Socks5Error::Io)?;
            let mut port_buf = [0u8; 2];
            stream
                .read_exact(&mut port_buf)
                .await
                .map_err(Socks5Error::Io)?;
            let port = u16::from_be_bytes(port_buf);
            TargetAddr::Ipv4(addr, port)
        }
        ATYP_DOMAIN => {
            let mut len_buf = [0u8; 1];
            stream
                .read_exact(&mut len_buf)
                .await
                .map_err(Socks5Error::Io)?;
            let len = len_buf[0] as usize;
            let mut domain_buf = vec![0u8; len];
            stream
                .read_exact(&mut domain_buf)
                .await
                .map_err(Socks5Error::Io)?;
            let domain =
                String::from_utf8(domain_buf).map_err(|_| Socks5Error::InvalidDomainName)?;
            let mut port_buf = [0u8; 2];
            stream
                .read_exact(&mut port_buf)
                .await
                .map_err(Socks5Error::Io)?;
            let port = u16::from_be_bytes(port_buf);
            TargetAddr::Domain(domain, port)
        }
        ATYP_IPV6 => {
            let mut addr = [0u8; 16];
            stream
                .read_exact(&mut addr)
                .await
                .map_err(Socks5Error::Io)?;
            let mut port_buf = [0u8; 2];
            stream
                .read_exact(&mut port_buf)
                .await
                .map_err(Socks5Error::Io)?;
            let port = u16::from_be_bytes(port_buf);
            TargetAddr::Ipv6(addr, port)
        }
        _ => return Err(Socks5Error::InvalidAddressType(atyp)),
    };

    Ok(Socks5Request { command, target })
}

/// SOCKS5 응답을 보냅니다.
async fn send_reply(
    stream: &mut TcpStream,
    reply: u8,
    bind_addr: &TargetAddr,
) -> Result<(), Socks5Error> {
    let mut buf = Vec::with_capacity(32);
    buf.push(SOCKS_VERSION);
    buf.push(reply);
    buf.push(0x00); // reserved

    match bind_addr {
        TargetAddr::Ipv4(addr, port) => {
            buf.push(ATYP_IPV4);
            buf.extend_from_slice(addr);
            buf.extend_from_slice(&port.to_be_bytes());
        }
        TargetAddr::Domain(domain, port) => {
            buf.push(ATYP_DOMAIN);
            buf.push(domain.len() as u8);
            buf.extend_from_slice(domain.as_bytes());
            buf.extend_from_slice(&port.to_be_bytes());
        }
        TargetAddr::Ipv6(addr, port) => {
            buf.push(ATYP_IPV6);
            buf.extend_from_slice(addr);
            buf.extend_from_slice(&port.to_be_bytes());
        }
    }

    stream.write_all(&buf).await.map_err(Socks5Error::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_no_auth() {
        let config = Socks5Config::default();
        assert_eq!(select_auth_method(&[AUTH_NO_AUTH], &config), AUTH_NO_AUTH);
        assert_eq!(
            select_auth_method(&[AUTH_USERNAME_PASSWORD], &config),
            AUTH_NO_ACCEPTABLE
        );
    }

    #[test]
    fn select_username_password_auth() {
        let config = Socks5Config {
            auth: Socks5Auth::UsernamePassword {
                username: "user".into(),
                password: "pass".into(),
            },
            upstream_proxy: None,
            throttle_rx: None,
        };
        assert_eq!(
            select_auth_method(&[AUTH_NO_AUTH, AUTH_USERNAME_PASSWORD], &config),
            AUTH_USERNAME_PASSWORD
        );
        assert_eq!(
            select_auth_method(&[AUTH_NO_AUTH], &config),
            AUTH_NO_ACCEPTABLE
        );
    }

    #[test]
    fn target_addr_display() {
        assert_eq!(
            TargetAddr::Ipv4([127, 0, 0, 1], 8080).to_address_string(),
            "127.0.0.1:8080"
        );
        assert_eq!(
            TargetAddr::Domain("example.com".into(), 443).to_address_string(),
            "example.com:443"
        );
        assert_eq!(
            TargetAddr::Ipv6([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1], 80)
                .to_address_string(),
            "::1:80"
        );
    }

    #[test]
    fn target_addr_from_socket_addr() {
        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let target = TargetAddr::from_socket_addr(addr);
        assert_eq!(target.to_address_string(), "127.0.0.1:3000");

        let addr: SocketAddr = "[::1]:443".parse().unwrap();
        let target = TargetAddr::from_socket_addr(addr);
        assert_eq!(target.to_address_string(), "::1:443");
    }
}
