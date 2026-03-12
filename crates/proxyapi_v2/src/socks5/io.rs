use super::protocol::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// 클라이언트 인사 메시지를 읽습니다.
pub(super) async fn read_greeting(stream: &mut TcpStream) -> Result<Socks5Greeting, Socks5Error> {
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
pub(super) async fn read_request(stream: &mut TcpStream) -> Result<Socks5Request, Socks5Error> {
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
pub(super) async fn send_reply(
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
