use super::config::{Socks5Auth, Socks5Config};
use super::io::{read_greeting, read_request, send_reply};
use super::protocol::*;
use crate::throttle;
use crate::upstream_proxy::connect_to_target;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, warn};

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
pub(super) fn select_auth_method(client_methods: &[u8], config: &Socks5Config) -> u8 {
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
