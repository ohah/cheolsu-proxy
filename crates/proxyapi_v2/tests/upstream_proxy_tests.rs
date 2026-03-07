use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use proxyapi_v2::upstream_proxy::{
    ProxyHttpConnector, UpstreamProxyAuth, UpstreamProxyConfig, connect_to_target,
};

// ---------------------------------------------------------------------------
// Mock upstream proxy server
// ---------------------------------------------------------------------------

/// Mock upstream proxy: CONNECT 요청을 받아서 200 응답을 보내고,
/// echo=true이면 에코 서버처럼 동작한다.
async fn start_mock_proxy(
    require_auth: Option<(String, String)>,
    echo: bool,
) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let handle = tokio::spawn(async move {
        // 한 번의 연결만 처리
        let (stream, _) = listener.accept().await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // CONNECT 요청의 첫 번째 줄 읽기
        let mut request_line = String::new();
        buf_reader.read_line(&mut request_line).await.unwrap();

        // 나머지 헤더 읽기
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            buf_reader.read_line(&mut line).await.unwrap();
            if line == "\r\n" {
                break;
            }
            headers.push(line);
        }

        // 인증 확인
        if let Some((expected_user, expected_pass)) = &require_auth {
            use base64::Engine;
            let expected_cred = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", expected_user, expected_pass));
            let expected_header = format!("Proxy-Authorization: Basic {}", expected_cred);

            let has_auth = headers.iter().any(|h| h.trim() == expected_header);

            if !has_auth {
                writer
                    .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                    .await
                    .unwrap();
                return;
            }
        }

        // CONNECT 메서드 확인
        if request_line.starts_with("CONNECT") {
            writer
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .await
                .unwrap();

            if echo {
                // 터널이 수립되면 에코: 클라이언트가 보낸 데이터를 그대로 돌려보낸다
                let mut echo_buf = String::new();
                if buf_reader.read_line(&mut echo_buf).await.unwrap() > 0 {
                    writer.write_all(echo_buf.as_bytes()).await.unwrap();
                }
            }
        } else {
            writer
                .write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n")
                .await
                .unwrap();
        }
    });

    (port, handle)
}

/// Mock upstream proxy: CONNECT 요청에 대해 항상 거부 (403)
async fn start_rejecting_proxy() -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // 헤더를 모두 읽기
        loop {
            let mut line = String::new();
            buf_reader.read_line(&mut line).await.unwrap();
            if line == "\r\n" {
                break;
            }
        }

        writer
            .write_all(b"HTTP/1.1 403 Forbidden\r\n\r\n")
            .await
            .unwrap();
    });

    (port, handle)
}

/// 단순 TCP 에코 서버 (대상 서버 역할)
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// CONNECT 터널 수립 → 에코 데이터 교환 테스트
#[tokio::test]
async fn connect_tunnel_through_mock_proxy() {
    let (proxy_port, proxy_handle) = start_mock_proxy(None, true).await;

    let config = UpstreamProxyConfig {
        host: "127.0.0.1".into(),
        port: proxy_port,
        auth: None,
        bypass: vec![],
    };

    // 실제 대상은 없지만, mock proxy가 200을 반환하고 에코를 해 준다.
    let stream = connect_to_target("example.com:443", Some(&config)).await;
    assert!(stream.is_ok(), "CONNECT 터널 수립 실패: {:?}", stream.err());

    let mut stream = stream.unwrap();
    // 에코 테스트
    stream.write_all(b"hello from client\n").await.unwrap();
    let mut buf = String::new();
    let mut buf_reader = BufReader::new(&mut stream);
    buf_reader.read_line(&mut buf).await.unwrap();
    assert_eq!(buf, "hello from client\n");

    proxy_handle.await.unwrap();
}

/// Basic 인증 헤더가 올바르게 전송되는지 확인
#[tokio::test]
async fn connect_tunnel_with_auth() {
    let (proxy_port, proxy_handle) =
        start_mock_proxy(Some(("admin".into(), "secret".into())), false).await;

    let config = UpstreamProxyConfig {
        host: "127.0.0.1".into(),
        port: proxy_port,
        auth: Some(UpstreamProxyAuth {
            username: "admin".into(),
            password: "secret".into(),
        }),
        bypass: vec![],
    };

    let stream = connect_to_target("example.com:443", Some(&config)).await;
    assert!(stream.is_ok(), "인증 포함 CONNECT 실패: {:?}", stream.err());

    drop(stream);
    proxy_handle.await.unwrap();
}

/// 잘못된 인증 정보로 연결 시 실패 확인
#[tokio::test]
async fn connect_tunnel_with_wrong_auth() {
    let (proxy_port, proxy_handle) =
        start_mock_proxy(Some(("admin".into(), "secret".into())), false).await;

    let config = UpstreamProxyConfig {
        host: "127.0.0.1".into(),
        port: proxy_port,
        auth: Some(UpstreamProxyAuth {
            username: "admin".into(),
            password: "wrong".into(),
        }),
        bypass: vec![],
    };

    let result = connect_to_target("example.com:443", Some(&config)).await;
    assert!(result.is_err(), "잘못된 인증으로도 성공하면 안 됨");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("407") || err.contains("CONNECT 실패"),
        "에러 메시지에 407 또는 CONNECT 실패가 포함되어야 함: {}",
        err
    );

    proxy_handle.await.unwrap();
}

/// bypass 설정 시 upstream proxy를 거치지 않고 직접 연결
#[tokio::test]
async fn bypass_connects_directly() {
    let (echo_port, echo_handle) = start_echo_server().await;

    let config = UpstreamProxyConfig {
        host: "127.0.0.1".into(),
        port: 9999, // 존재하지 않는 proxy — 바이패스되면 이 주소에 연결하지 않음
        auth: None,
        bypass: vec!["127.0.0.1".into()],
    };

    let stream = connect_to_target(&format!("127.0.0.1:{}", echo_port), Some(&config)).await;
    assert!(
        stream.is_ok(),
        "바이패스 직접 연결 실패: {:?}",
        stream.err()
    );

    let mut stream = stream.unwrap();
    stream.write_all(b"bypass test\n").await.unwrap();
    let mut buf = String::new();
    let mut buf_reader = BufReader::new(&mut stream);
    buf_reader.read_line(&mut buf).await.unwrap();
    assert_eq!(buf, "bypass test\n");

    echo_handle.await.unwrap();
}

/// upstream 미설정 시 직접 연결
#[tokio::test]
async fn no_upstream_connects_directly() {
    let (echo_port, echo_handle) = start_echo_server().await;

    let stream = connect_to_target(&format!("127.0.0.1:{}", echo_port), None).await;
    assert!(stream.is_ok(), "직접 연결 실패: {:?}", stream.err());

    let mut stream = stream.unwrap();
    stream.write_all(b"direct\n").await.unwrap();
    let mut buf = String::new();
    let mut buf_reader = BufReader::new(&mut stream);
    buf_reader.read_line(&mut buf).await.unwrap();
    assert_eq!(buf, "direct\n");

    echo_handle.await.unwrap();
}

/// CONNECT 거부 시 에러 반환 확인
#[tokio::test]
async fn connect_rejected_by_proxy() {
    let (proxy_port, proxy_handle) = start_rejecting_proxy().await;

    let config = UpstreamProxyConfig {
        host: "127.0.0.1".into(),
        port: proxy_port,
        auth: None,
        bypass: vec![],
    };

    let result = connect_to_target("example.com:443", Some(&config)).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("403") || err.contains("CONNECT 실패"),
        "에러에 403이 포함되어야 함: {}",
        err
    );

    proxy_handle.await.unwrap();
}

/// 존재하지 않는 upstream proxy 연결 시 에러
#[tokio::test]
async fn connect_to_unreachable_proxy() {
    let config = UpstreamProxyConfig {
        host: "127.0.0.1".into(),
        port: 1, // 거의 확실히 열려있지 않은 포트
        auth: None,
        bypass: vec![],
    };

    let result = connect_to_target("example.com:443", Some(&config)).await;
    assert!(result.is_err(), "존재하지 않는 proxy에 연결 성공하면 안 됨");
}

/// ProxyHttpConnector — upstream 없이 직접 연결
#[tokio::test]
async fn proxy_connector_direct() {
    use http::Uri;
    use tower::Service;

    // 연결만 수락하고 바로 종료하는 서버
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
    });

    let (_tx, rx) = tokio::sync::watch::channel(None);
    let mut connector = ProxyHttpConnector::new(rx);
    let uri: Uri = format!("http://127.0.0.1:{}", port).parse().unwrap();

    let result = connector.call(uri).await;
    assert!(
        result.is_ok(),
        "ProxyHttpConnector 직접 연결 실패: {:?}",
        result.err()
    );

    handle.await.unwrap();
}

/// ProxyHttpConnector — upstream proxy 경유 HTTP 연결 (proxy에 직접 연결)
#[tokio::test]
async fn proxy_connector_via_upstream_http() {
    use http::Uri;
    use tower::Service;

    // HTTP의 경우 ProxyHttpConnector는 upstream proxy에 직접 TCP 연결만 한다
    // (CONNECT를 쓰지 않고 hyper가 full URL 요청을 전송)
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_port = listener.local_addr().unwrap().port();

    let handle = tokio::spawn(async move {
        let (_stream, _addr) = listener.accept().await.unwrap();
        // 연결 수립만 확인 — 바로 종료
    });

    let config = UpstreamProxyConfig {
        host: "127.0.0.1".into(),
        port: proxy_port,
        auth: None,
        bypass: vec![],
    };

    let (_tx, rx) = tokio::sync::watch::channel(Some(config));
    let mut connector = ProxyHttpConnector::new(rx);
    let uri: Uri = "http://example.com".parse().unwrap();

    let result = connector.call(uri).await;
    assert!(
        result.is_ok(),
        "HTTP upstream 연결 실패: {:?}",
        result.err()
    );

    handle.await.unwrap();
}

/// Protocol serialization: UpdateUpstreamProxy 직렬화/역직렬화
#[test]
fn upstream_proxy_config_serde() {
    let config = UpstreamProxyConfig {
        host: "proxy.corp.com".into(),
        port: 3128,
        auth: Some(UpstreamProxyAuth {
            username: "user".into(),
            password: "pass".into(),
        }),
        bypass: vec!["localhost".into(), "*.internal.com".into()],
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: UpstreamProxyConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.host, "proxy.corp.com");
    assert_eq!(deserialized.port, 3128);
    assert!(deserialized.auth.is_some());
    let auth = deserialized.auth.unwrap();
    assert_eq!(auth.username, "user");
    assert_eq!(auth.password, "pass");
    assert_eq!(deserialized.bypass, vec!["localhost", "*.internal.com"]);
}

/// auth가 null인 config도 정상적으로 역직렬화되는지
#[test]
fn upstream_proxy_config_serde_no_auth() {
    let json = r#"{"host":"proxy.example.com","port":8080}"#;
    let config: UpstreamProxyConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.host, "proxy.example.com");
    assert_eq!(config.port, 8080);
    assert!(config.auth.is_none());
    assert!(config.bypass.is_empty());
}

/// bypass 패턴 - 다양한 와일드카드 케이스
#[test]
fn bypass_pattern_edge_cases() {
    let config = UpstreamProxyConfig {
        host: "proxy".into(),
        port: 8080,
        auth: None,
        bypass: vec![
            "localhost".into(),
            "*.corp.com".into(),
            "*suffix.io".into(),
            "exact.host.net".into(),
        ],
    };

    // localhost 확장 매칭
    assert!(config.should_bypass("localhost"));
    assert!(config.should_bypass("127.0.0.1"));
    assert!(config.should_bypass("::1"));

    // *.corp.com — 서브도메인 및 정확히 corp.com
    assert!(config.should_bypass("api.corp.com"));
    assert!(config.should_bypass("deep.sub.corp.com"));
    assert!(config.should_bypass("corp.com"));
    assert!(!config.should_bypass("notcorp.com"));

    // *suffix.io — 접미사 매칭
    assert!(config.should_bypass("anysuffix.io"));
    assert!(config.should_bypass("suffix.io"));
    assert!(!config.should_bypass("suffix.io.other"));

    // exact match
    assert!(config.should_bypass("exact.host.net"));
    assert!(!config.should_bypass("other.host.net"));
    assert!(!config.should_bypass("sub.exact.host.net"));
}

/// Base64 인증 인코딩 형식 확인
#[test]
fn auth_base64_encoding() {
    use base64::Engine;

    let auth = UpstreamProxyAuth {
        username: "admin".into(),
        password: "p@ssw0rd!".into(),
    };

    let encoded = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", auth.username, auth.password));

    // 디코딩해서 올바른지 확인
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .unwrap();
    let decoded_str = String::from_utf8(decoded).unwrap();
    assert_eq!(decoded_str, "admin:p@ssw0rd!");
}
