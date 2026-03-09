use proxyapi_v2::socks5::*;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

// ---------------------------------------------------------------------------
// 테스트 헬퍼
// ---------------------------------------------------------------------------

/// 간단한 TCP 에코 서버
async fn start_echo_server() -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        let mut line = String::new();
        if buf_reader.read_line(&mut line).await.unwrap() > 0 {
            writer.write_all(line.as_bytes()).await.unwrap();
        }
    });

    (port, handle)
}

/// SOCKS5 서버 시작 헬퍼
async fn start_socks5_server(config: Socks5Config) -> (u16, tokio::sync::oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = tokio::sync::oneshot::channel();

    let server = Socks5Server::from_listener(listener, config);

    tokio::spawn(async move {
        server
            .start(async { rx.await.unwrap_or_default() })
            .await
            .unwrap();
    });

    (port, tx)
}

/// SOCKS5 핸드셰이크 수행 (인증 없음)
async fn socks5_handshake_no_auth(stream: &mut TcpStream) {
    // 인사 메시지: version(5) + nmethods(1) + methods(NO_AUTH)
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();

    let mut response = [0u8; 2];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(response[0], 0x05, "SOCKS 버전");
    assert_eq!(response[1], 0x00, "인증 방법: NO_AUTH");
}

/// SOCKS5 핸드셰이크 수행 (사용자명/비밀번호)
async fn socks5_handshake_with_auth(
    stream: &mut TcpStream,
    username: &str,
    password: &str,
) -> bool {
    // 인사 메시지: version(5) + nmethods(1) + methods(USERNAME_PASSWORD)
    stream.write_all(&[0x05, 0x01, 0x02]).await.unwrap();

    let mut response = [0u8; 2];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(response[0], 0x05);
    assert_eq!(response[1], 0x02); // USERNAME_PASSWORD 선택됨

    // 사용자명/비밀번호 서브네고시에이션
    let mut auth_req = Vec::new();
    auth_req.push(0x01); // subneg version
    auth_req.push(username.len() as u8);
    auth_req.extend_from_slice(username.as_bytes());
    auth_req.push(password.len() as u8);
    auth_req.extend_from_slice(password.as_bytes());
    stream.write_all(&auth_req).await.unwrap();

    let mut auth_response = [0u8; 2];
    stream.read_exact(&mut auth_response).await.unwrap();
    auth_response[1] == 0x00 // 0x00 = 성공
}

/// SOCKS5 CONNECT 요청 (IPv4)
async fn socks5_connect_ipv4(stream: &mut TcpStream, ip: [u8; 4], port: u16) -> u8 {
    let mut req = vec![0x05, 0x01, 0x00, 0x01]; // VER, CMD_CONNECT, RSV, ATYP_IPV4
    req.extend_from_slice(&ip);
    req.extend_from_slice(&port.to_be_bytes());
    stream.write_all(&req).await.unwrap();

    let mut response = [0u8; 10]; // VER + REP + RSV + ATYP(IPv4) + ADDR(4) + PORT(2)
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(response[0], 0x05, "응답 SOCKS 버전");
    response[1] // reply code
}

/// SOCKS5 CONNECT 요청 (도메인)
async fn socks5_connect_domain(stream: &mut TcpStream, domain: &str, port: u16) -> u8 {
    let mut req = vec![0x05, 0x01, 0x00, 0x03]; // VER, CMD_CONNECT, RSV, ATYP_DOMAIN
    req.push(domain.len() as u8);
    req.extend_from_slice(domain.as_bytes());
    req.extend_from_slice(&port.to_be_bytes());
    stream.write_all(&req).await.unwrap();

    // 응답: VER(1) + REP(1) + RSV(1) + ATYP(1) + ...
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await.unwrap();
    assert_eq!(header[0], 0x05);
    let reply = header[1];

    // 나머지 주소 부분 소비
    match header[3] {
        0x01 => {
            let mut rest = [0u8; 6]; // IPv4(4) + port(2)
            stream.read_exact(&mut rest).await.unwrap();
        }
        0x03 => {
            let mut len_buf = [0u8; 1];
            stream.read_exact(&mut len_buf).await.unwrap();
            let mut rest = vec![0u8; len_buf[0] as usize + 2]; // domain + port(2)
            stream.read_exact(&mut rest).await.unwrap();
        }
        0x04 => {
            let mut rest = [0u8; 18]; // IPv6(16) + port(2)
            stream.read_exact(&mut rest).await.unwrap();
        }
        _ => panic!("알 수 없는 ATYP: {}", header[3]),
    }

    reply
}

// ---------------------------------------------------------------------------
// 테스트
// ---------------------------------------------------------------------------

/// 인증 없이 SOCKS5 CONNECT → 에코 서버 통신
#[tokio::test]
async fn socks5_connect_no_auth() {
    let (echo_port, echo_handle) = start_echo_server().await;
    let (socks_port, stop) = start_socks5_server(Socks5Config::default()).await;

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", socks_port))
        .await
        .unwrap();

    // 핸드셰이크
    socks5_handshake_no_auth(&mut client).await;

    // CONNECT 요청
    let reply = socks5_connect_ipv4(&mut client, [127, 0, 0, 1], echo_port).await;
    assert_eq!(reply, REPLY_SUCCEEDED);

    // 에코 테스트
    client.write_all(b"hello socks5\n").await.unwrap();
    let mut buf = String::new();
    let mut reader = BufReader::new(&mut client);
    reader.read_line(&mut buf).await.unwrap();
    assert_eq!(buf, "hello socks5\n");

    stop.send(()).unwrap();
    echo_handle.await.unwrap();
}

/// 사용자명/비밀번호 인증 성공 → CONNECT
#[tokio::test]
async fn socks5_connect_with_auth_success() {
    let (echo_port, echo_handle) = start_echo_server().await;
    let config = Socks5Config {
        auth: Socks5Auth::UsernamePassword {
            username: "admin".into(),
            password: "secret".into(),
        },
        upstream_proxy: None,
        throttle_rx: None,
    };
    let (socks_port, stop) = start_socks5_server(config).await;

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", socks_port))
        .await
        .unwrap();

    // 핸드셰이크 + 인증
    let auth_ok = socks5_handshake_with_auth(&mut client, "admin", "secret").await;
    assert!(auth_ok, "인증 성공해야 함");

    // CONNECT
    let reply = socks5_connect_ipv4(&mut client, [127, 0, 0, 1], echo_port).await;
    assert_eq!(reply, REPLY_SUCCEEDED);

    // 에코
    client.write_all(b"authenticated\n").await.unwrap();
    let mut buf = String::new();
    let mut reader = BufReader::new(&mut client);
    reader.read_line(&mut buf).await.unwrap();
    assert_eq!(buf, "authenticated\n");

    stop.send(()).unwrap();
    echo_handle.await.unwrap();
}

/// 잘못된 비밀번호로 인증 실패
#[tokio::test]
async fn socks5_auth_failure() {
    let config = Socks5Config {
        auth: Socks5Auth::UsernamePassword {
            username: "admin".into(),
            password: "secret".into(),
        },
        upstream_proxy: None,
        throttle_rx: None,
    };
    let (socks_port, stop) = start_socks5_server(config).await;

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", socks_port))
        .await
        .unwrap();

    let auth_ok = socks5_handshake_with_auth(&mut client, "admin", "wrong").await;
    assert!(!auth_ok, "잘못된 비밀번호이므로 인증 실패해야 함");

    stop.send(()).unwrap();
}

/// 서버가 인증을 요구하지만 클라이언트가 NO_AUTH만 제공하는 경우
#[tokio::test]
async fn socks5_no_acceptable_method() {
    let config = Socks5Config {
        auth: Socks5Auth::UsernamePassword {
            username: "user".into(),
            password: "pass".into(),
        },
        upstream_proxy: None,
        throttle_rx: None,
    };
    let (socks_port, stop) = start_socks5_server(config).await;

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", socks_port))
        .await
        .unwrap();

    // NO_AUTH만 제공
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();

    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response[0], 0x05);
    assert_eq!(response[1], 0xFF, "수용 가능한 방법 없음 (0xFF)");

    stop.send(()).unwrap();
}

/// 도메인 이름으로 CONNECT 요청
#[tokio::test]
async fn socks5_connect_domain_name() {
    let (echo_port, echo_handle) = start_echo_server().await;
    let (socks_port, stop) = start_socks5_server(Socks5Config::default()).await;

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", socks_port))
        .await
        .unwrap();

    socks5_handshake_no_auth(&mut client).await;

    // localhost 도메인으로 CONNECT
    let reply = socks5_connect_domain(&mut client, "127.0.0.1", echo_port).await;
    assert_eq!(reply, REPLY_SUCCEEDED);

    // 에코
    client.write_all(b"domain connect\n").await.unwrap();
    let mut buf = String::new();
    let mut reader = BufReader::new(&mut client);
    reader.read_line(&mut buf).await.unwrap();
    assert_eq!(buf, "domain connect\n");

    stop.send(()).unwrap();
    echo_handle.await.unwrap();
}

/// 연결할 수 없는 대상으로 CONNECT → HOST_UNREACHABLE 응답
#[tokio::test]
async fn socks5_connect_unreachable_target() {
    let (socks_port, stop) = start_socks5_server(Socks5Config::default()).await;

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", socks_port))
        .await
        .unwrap();

    socks5_handshake_no_auth(&mut client).await;

    // 존재하지 않는 포트로 CONNECT
    let reply = socks5_connect_ipv4(&mut client, [127, 0, 0, 1], 1).await;
    assert_eq!(reply, REPLY_HOST_UNREACHABLE);

    stop.send(()).unwrap();
}

/// 지원하지 않는 명령 (BIND) → COMMAND_NOT_SUPPORTED 응답
#[tokio::test]
async fn socks5_unsupported_command() {
    let (socks_port, stop) = start_socks5_server(Socks5Config::default()).await;

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", socks_port))
        .await
        .unwrap();

    socks5_handshake_no_auth(&mut client).await;

    // BIND 명령 (0x02)
    let req = [
        0x05, 0x02, 0x00, 0x01, // VER, CMD_BIND, RSV, ATYP_IPV4
        127, 0, 0, 1, // addr
        0x00, 0x50, // port 80
    ];
    client.write_all(&req).await.unwrap();

    let mut response = [0u8; 10];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response[0], 0x05);
    assert_eq!(response[1], REPLY_COMMAND_NOT_SUPPORTED);

    stop.send(()).unwrap();
}

/// 잘못된 SOCKS 버전 → 연결 끊김
#[tokio::test]
async fn socks5_invalid_version() {
    let (socks_port, stop) = start_socks5_server(Socks5Config::default()).await;

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", socks_port))
        .await
        .unwrap();

    // SOCKS4 버전으로 보내기
    client.write_all(&[0x04, 0x01, 0x00]).await.unwrap();

    // 서버가 연결을 끊어야 함
    let mut buf = [0u8; 1];
    let result = client.read(&mut buf).await;
    match result {
        Ok(0) => {}  // 연결 끊김 (정상)
        Err(_) => {} // IO 에러 (정상)
        Ok(n) => panic!("잘못된 버전에 대해 연결이 끊겨야 하는데 {} 바이트 수신", n),
    }

    stop.send(()).unwrap();
}

/// 여러 클라이언트 동시 연결
#[tokio::test]
async fn socks5_concurrent_connections() {
    let (_echo_port, _echo_handle) = start_echo_server().await;

    // 에코 서버를 여러 연결 수용하도록 재시작
    let echo_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_port = echo_listener.local_addr().unwrap().port();
    let echo_handle = tokio::spawn(async move {
        for _ in 0..3 {
            let (stream, _) = echo_listener.accept().await.unwrap();
            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut buf_reader = BufReader::new(reader);
                let mut line = String::new();
                if buf_reader.read_line(&mut line).await.unwrap() > 0 {
                    writer.write_all(line.as_bytes()).await.unwrap();
                }
            });
        }
    });

    let (socks_port, stop) = start_socks5_server(Socks5Config::default()).await;

    let mut handles = vec![];
    for i in 0..3 {
        let handle = tokio::spawn(async move {
            let mut client = TcpStream::connect(format!("127.0.0.1:{}", socks_port))
                .await
                .unwrap();
            socks5_handshake_no_auth(&mut client).await;
            let reply = socks5_connect_ipv4(&mut client, [127, 0, 0, 1], echo_port).await;
            assert_eq!(reply, REPLY_SUCCEEDED);

            let msg = format!("client {}\n", i);
            client.write_all(msg.as_bytes()).await.unwrap();
            let mut buf = String::new();
            let mut reader = BufReader::new(&mut client);
            reader.read_line(&mut buf).await.unwrap();
            assert_eq!(buf, msg);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    stop.send(()).unwrap();
    echo_handle.await.unwrap();
}

/// Socks5Server::bind로 서버 생성
#[tokio::test]
async fn socks5_server_bind() {
    let addr = "127.0.0.1:0".parse().unwrap();
    let server = Socks5Server::bind(addr, Socks5Config::default())
        .await
        .unwrap();
    let local_addr = server.local_addr().unwrap();
    assert_ne!(local_addr.port(), 0);

    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        server
            .start(async { rx.await.unwrap_or_default() })
            .await
            .unwrap();
    });

    // 연결 테스트
    let mut client = TcpStream::connect(local_addr).await.unwrap();
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();

    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response, [0x05, 0x00]);

    tx.send(()).unwrap();
}
