use super::internal::InternalProxy;
use crate::{
    HttpHandler, WebSocketHandler, certificate_authority::CertificateAuthority, rewind::Rewind,
    tunnel_event::TunnelEvent,
};
use http::uri::Authority;
use hyper::{Method, Uri, upgrade::Upgraded};
use hyper_util::{client::legacy::connect::Connect, rt::TokioIo};
use proxy_v2_models::{ProxiedRequest, ProxiedResponse, RequestInfo};
use std::collections::HashMap;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};
use tracing::{debug, error, info, instrument, warn};

impl<C, CA, H, W> InternalProxy<C, CA, H, W>
where
    C: Connect + Clone + Send + Sync + 'static,
    CA: CertificateAuthority,
    H: HttpHandler,
    W: WebSocketHandler,
{
    #[instrument(skip_all)]
    /// 터널 모드가 필요한 도메인인지 확인합니다
    /// 현재 비활성화 — 모든 도메인을 MITM 인터셉트합니다
    pub(crate) fn is_tunnel_mode_domain(&self, _authority: &Authority) -> bool {
        false
    }

    /// 터널 모드에서 HTTP 요청을 파싱하는 헬퍼 함수
    pub(crate) fn parse_http_request_from_buffer(
        buffer: &[u8],
    ) -> Option<(String, String, String, HashMap<String, String>)> {
        if let Ok(data_str) = std::str::from_utf8(buffer) {
            let lines: Vec<&str> = data_str.lines().collect();
            if let Some(first_line) = lines.first() {
                let parts: Vec<&str> = first_line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let method = parts[0].to_string();
                    let path = parts[1].to_string();
                    let version = parts[2].to_string();

                    // HTTP 메서드 검증 (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS, TRACE, CONNECT)
                    let valid_methods = [
                        "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "TRACE",
                        "CONNECT",
                    ];
                    if !valid_methods.contains(&method.as_str()) {
                        return None;
                    }

                    // 헤더 파싱
                    let mut headers = HashMap::new();
                    for line in lines.iter().skip(1) {
                        if line.is_empty() {
                            break; // 헤더와 본문 구분
                        }
                        if let Some((key, value)) = line.split_once(':') {
                            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
                        }
                    }

                    return Some((method, path, version, headers));
                }
            }
        }
        None
    }

    /// 터널 모드에서 HTTP 응답을 파싱하는 헬퍼 함수
    pub(crate) fn parse_http_response_from_buffer(
        buffer: &[u8],
    ) -> Option<(String, u16, String, HashMap<String, String>)> {
        if let Ok(data_str) = std::str::from_utf8(buffer) {
            let lines: Vec<&str> = data_str.lines().collect();
            if let Some(first_line) = lines.first() {
                let parts: Vec<&str> = first_line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let version = parts[0].to_string();
                    let status_code = parts[1].parse::<u16>().ok()?;
                    let reason_phrase = parts[2..].join(" ");

                    // 헤더 파싱
                    let mut headers = HashMap::new();
                    for line in lines.iter().skip(1) {
                        if line.is_empty() {
                            break; // 헤더와 본문 구분
                        }
                        if let Some((key, value)) = line.split_once(':') {
                            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
                        }
                    }

                    return Some((version, status_code, reason_phrase, headers));
                }
            }
        }
        None
    }

    /// 터널 모드에서 데이터를 모니터링하면서 양방향 복사를 수행합니다
    pub(crate) async fn copy_bidirectional_with_monitoring(
        client_stream: &mut TokioIo<Upgraded>,
        server_stream: &mut TcpStream,
        target_addr: &str,
        tunnel_event_sender: Option<mpsc::Sender<RequestInfo>>,
    ) -> Result<(u64, u64), std::io::Error> {
        // 데이터 버퍼
        let mut client_to_server_buffer = [0u8; 8192];
        let mut server_to_client_buffer = [0u8; 8192];

        let mut client_to_server_bytes = 0u64;

        // 터널 내부 HTTP 요청과 응답 매칭을 위한 요청 ID 추적
        let mut pending_requests: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut server_to_client_bytes = 0u64;

        let mut client_read_done = false;
        let mut server_read_done = false;

        loop {
            tokio::select! {
                // 클라이언트 → 서버 데이터 읽기
                result = client_stream.read(&mut client_to_server_buffer), if !client_read_done => {
                    match result {
                        Ok(0) => {
                            client_read_done = true;
                            debug!("[TUNNEL-DATA] 클라이언트 연결 종료: {}", target_addr);
                        }
                        Ok(n) => {
                            client_to_server_bytes += n as u64;

                            // HTTP 요청 감지 및 로깅
                            let data_preview = &client_to_server_buffer[..n];
                            if let Some((method, path, version, headers)) = Self::parse_http_request_from_buffer(data_preview) {
                                debug!("[TUNNEL-HTTP] HTTP 요청 감지: {} {} {} (터널: {})",
                                    method, path, version, target_addr);

                                // Host 헤더에서 실제 호스트 추출
                                let host = headers.get("host")
                                    .map(|h| h.split(':').next().unwrap_or(h))
                                    .unwrap_or_else(|| target_addr.split(':').next().unwrap_or("unknown"));

                                // 터널 이벤트로 HTTP 요청 전송
                                if let Some(sender) = &tunnel_event_sender {
                                    use hyper::Version;
                                    use http::HeaderMap;

                                    // HTTP 헤더를 HeaderMap으로 변환
                                    let mut header_map = HeaderMap::new();
                                    for (key, value) in &headers {
                                        if let (Ok(header_name), Ok(header_value)) = (
                                            hyper::header::HeaderName::from_bytes(key.as_bytes()),
                                            hyper::header::HeaderValue::from_str(value)
                                        ) {
                                            header_map.insert(header_name, header_value);
                                        }
                                    }

                                    // URI 생성
                                    let uri = format!("https://{}{}", host, path)
                                        .parse::<Uri>()
                                        .unwrap_or_else(|_| Uri::from_static("https://unknown/"));

                                    // HTTP 메서드 변환
                                    let http_method = method.parse::<Method>()
                                        .unwrap_or_else(|_| Method::GET);

                                    // HTTP 버전 변환
                                    let http_version = if version.contains("1.1") {
                                        Version::HTTP_11
                                    } else if version.contains("2.0") {
                                        Version::HTTP_2
                                    } else {
                                        Version::HTTP_10
                                    };

                                    let proxied_request = ProxiedRequest::new(
                                        http_method,
                                        uri,
                                        http_version,
                                        header_map,
                                        bytes::Bytes::new(),
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_nanos() as i64,
                                    );

                                    let request_id = proxied_request.id().clone();
                                    let tunnel_request = RequestInfo(
                                        Some(proxied_request.for_client(None)),
                                        None,
                                    );

                                    if let Err(e) = sender.try_send(tunnel_request) {
                                        warn!("터널 이벤트 전송 실패: {}", e);
                                    }

                                    // 요청 ID를 pending_requests에 저장 (응답 매칭용)
                                    pending_requests.insert(request_id.clone(), format!("{} {}", method, path));
                                }

                                debug!("[TUNNEL-HTTP] 터널 내 HTTP 요청: {} https://{}{}", method, host, path);
                            }

                            // 데이터 로깅 (처음 1024바이트만)
                            if client_to_server_bytes <= 1024 {
                                if let Ok(data_str) = std::str::from_utf8(data_preview) {
                                    debug!("[TUNNEL-DATA] 클라이언트→서버 ({}): {}", target_addr,
                                        data_str.chars().take(200).collect::<String>());
                                } else {
                                    debug!("[TUNNEL-DATA] 클라이언트→서버 ({}): [바이너리 데이터 {} bytes]",
                                        target_addr, n);
                                }
                            }

                            // 서버로 데이터 전송
                            if let Err(e) = server_stream.write_all(&client_to_server_buffer[..n]).await {
                                error!("[TUNNEL-DATA] 서버로 데이터 전송 실패: {}", e);
                                return Err(e);
                            }
                        }
                        Err(e) => {
                            error!("[TUNNEL-DATA] 클라이언트에서 데이터 읽기 실패: {}", e);
                            return Err(e);
                        }
                    }
                }

                // 서버 → 클라이언트 데이터 읽기
                result = server_stream.read(&mut server_to_client_buffer), if !server_read_done => {
                    match result {
                        Ok(0) => {
                            server_read_done = true;
                            debug!("[TUNNEL-DATA] 서버 연결 종료: {}", target_addr);
                        }
                        Ok(n) => {
                            server_to_client_bytes += n as u64;

                            // HTTP 응답 감지 및 로깅
                            let data_preview = &server_to_client_buffer[..n];
                            if let Some((version, status_code, reason_phrase, headers)) = Self::parse_http_response_from_buffer(data_preview) {
                                debug!("[TUNNEL-HTTP] HTTP 응답 감지: {} {} {} (터널: {})",
                                    version, status_code, reason_phrase, target_addr);

                                // 터널 이벤트로 HTTP 응답 전송 (마지막 요청에 대한 응답으로 처리)
                                if let Some(sender) = &tunnel_event_sender {
                                    use hyper::{StatusCode, Version};
                                    use http::HeaderMap;

                                    // HTTP 헤더를 HeaderMap으로 변환
                                    let mut header_map = HeaderMap::new();
                                    for (key, value) in &headers {
                                        if let (Ok(header_name), Ok(header_value)) = (
                                            hyper::header::HeaderName::from_bytes(key.as_bytes()),
                                            hyper::header::HeaderValue::from_str(value)
                                        ) {
                                            header_map.insert(header_name, header_value);
                                        }
                                    }

                                    // HTTP 버전 변환
                                    let http_version = if version.contains("1.1") {
                                        Version::HTTP_11
                                    } else if version.contains("2.0") {
                                        Version::HTTP_2
                                    } else {
                                        Version::HTTP_10
                                    };

                                    let proxied_response = ProxiedResponse::new(
                                        StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK),
                                        http_version,
                                        header_map,
                                        bytes::Bytes::new(),
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_nanos() as i64,
                                    );

                                    // 가장 최근 요청 ID를 사용하여 응답 매칭
                                    let response_request_id = if let Some((latest_request_id, _)) = pending_requests.iter().last() {
                                        latest_request_id.clone()
                                    } else {
                                        "tunnel-response".to_string()
                                    };

                                    let tunnel_response = RequestInfo(
                                        None, // 요청은 이미 전송됨
                                        Some(proxied_response.for_client(&response_request_id, None)),
                                    );

                                    if let Err(e) = sender.try_send(tunnel_response) {
                                        warn!("터널 응답 이벤트 전송 실패: {}", e);
                                    }

                                    // 응답을 받았으므로 해당 요청을 pending에서 제거
                                    if response_request_id != "tunnel-response" {
                                        pending_requests.remove(&response_request_id);
                                    }
                                }

                                debug!("[TUNNEL-HTTP] 터널 내 HTTP 응답: {} {} {}", version, status_code, reason_phrase);
                            }

                            // 데이터 로깅 (처음 1024바이트만)
                            if server_to_client_bytes <= 1024 {
                                if let Ok(data_str) = std::str::from_utf8(data_preview) {
                                    debug!("[TUNNEL-DATA] 서버→클라이언트 ({}): {}", target_addr,
                                        data_str.chars().take(200).collect::<String>());
                                } else {
                                    debug!("[TUNNEL-DATA] 서버→클라이언트 ({}): [바이너리 데이터 {} bytes]",
                                        target_addr, n);
                                }
                            }

                            // 클라이언트로 데이터 전송
                            if let Err(e) = client_stream.write_all(&server_to_client_buffer[..n]).await {
                                error!("[TUNNEL-DATA] 클라이언트로 데이터 전송 실패: {}", e);
                                return Err(e);
                            }
                        }
                        Err(e) => {
                            error!("[TUNNEL-DATA] 서버에서 데이터 읽기 실패: {}", e);
                            return Err(e);
                        }
                    }
                }
            }

            // 양쪽 모두 연결이 종료되면 루프 종료
            if client_read_done && server_read_done {
                break;
            }
        }

        debug!(
            target_addr,
            client_to_server_bytes, server_to_client_bytes, "[TUNNEL-DATA] 데이터 전송 완료"
        );

        Ok((client_to_server_bytes, server_to_client_bytes))
    }

    /// 터널 모드를 처리합니다
    pub(crate) async fn handle_tunnel_mode(
        &self,
        authority: &Authority,
        upgraded: Rewind<TokioIo<Upgraded>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("[TUNNEL-MODE] 터널 모드 처리 시작: {}", authority);

        // 대상 서버에 직접 연결
        let port = authority
            .port()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "443".to_string());
        let target_addr = format!("{}:{}", authority.host(), port);
        let client_addr = self.client_addr.to_string();

        debug!("[TUNNEL-MODE] 대상 서버 연결 시도: {}", target_addr);

        // 터널 시작 이벤트 전송
        if let Some(ref sender) = self.tunnel_event_sender {
            let start_event = TunnelEvent::started(target_addr.clone(), client_addr.clone());
            let request_info = start_event.to_request_info();
            debug!("[TUNNEL-EVENT] 터널 시작 이벤트 전송: {}", target_addr);
            if let Err(e) = sender.send(request_info).await {
                error!("[TUNNEL-EVENT] 터널 시작 이벤트 전송 실패: {}", e);
            } else {
                info!("[TUNNEL-EVENT] 터널 시작 이벤트 전송 성공: {}", target_addr);
            }
        } else {
            warn!(
                "[TUNNEL-EVENT] tunnel_event_sender가 None입니다: {}",
                target_addr
            );
        }

        let mut server_stream = match tokio::net::TcpStream::connect(&target_addr).await {
            Ok(stream) => {
                info!("[TUNNEL-MODE] 대상 서버 연결 성공: {}", target_addr);
                stream
            }
            Err(e) => {
                error!("[TUNNEL-MODE] 대상 서버 연결 실패: {} - {}", target_addr, e);
                return Err(format!("Failed to connect to target server: {}", e).into());
            }
        };

        // 클라이언트 스트림 추출
        let mut client_stream = upgraded.into_inner();

        // 실제 터널 생성 - 클라이언트와 서버 간 양방향 데이터 전달
        let start_time = std::time::Instant::now();
        debug!(
            "[TUNNEL-TASK] 터널 작업 시작: {} <-> {}",
            target_addr, "클라이언트"
        );

        // 터널 작업에 타임아웃 추가 (5분) - 데이터 모니터링 포함
        match tokio::time::timeout(
            std::time::Duration::from_secs(300),
            Self::copy_bidirectional_with_monitoring(
                &mut client_stream,
                &mut server_stream,
                &target_addr,
                self.tunnel_event_sender.clone(),
            ),
        )
        .await
        {
            Ok(Ok((client_to_server, server_to_client))) => {
                let duration = start_time.elapsed();
                info!(
                    client_to_server_bytes = client_to_server,
                    server_to_client_bytes = server_to_client,
                    ?duration,
                    "[TUNNEL-TASK] 터널 완료"
                );

                // 터널 완료 이벤트 전송
                if let Some(ref sender) = self.tunnel_event_sender {
                    let completed_event = TunnelEvent::completed(
                        target_addr.clone(),
                        client_addr,
                        client_to_server,
                        server_to_client,
                        duration,
                    );
                    let request_info = completed_event.to_request_info();
                    debug!("[TUNNEL-EVENT] 터널 완료 이벤트 전송: {}", target_addr);
                    if let Err(e) = sender.send(request_info).await {
                        error!("[TUNNEL-EVENT] 터널 완료 이벤트 전송 실패: {}", e);
                    } else {
                        info!("[TUNNEL-EVENT] 터널 완료 이벤트 전송 성공: {}", target_addr);
                    }
                } else {
                    warn!(
                        "[TUNNEL-EVENT] tunnel_event_sender가 None입니다 (완료): {}",
                        target_addr
                    );
                }

                Ok(())
            }
            Ok(Err(e)) => {
                let duration = start_time.elapsed();
                error!(
                    error = %e,
                    ?duration,
                    "[TUNNEL-TASK] 터널 오류"
                );

                // 터널 오류 이벤트 전송
                if let Some(ref sender) = self.tunnel_event_sender {
                    let error_event = TunnelEvent::error(
                        target_addr.clone(),
                        client_addr,
                        e.to_string(),
                        duration,
                    );
                    let request_info = error_event.to_request_info();
                    debug!("[TUNNEL-EVENT] 터널 오류 이벤트 전송: {}", target_addr);
                    if let Err(e) = sender.send(request_info).await {
                        error!("[TUNNEL-EVENT] 터널 오류 이벤트 전송 실패: {}", e);
                    } else {
                        info!("[TUNNEL-EVENT] 터널 오류 이벤트 전송 성공: {}", target_addr);
                    }
                } else {
                    warn!(
                        "[TUNNEL-EVENT] tunnel_event_sender가 None입니다 (오류): {}",
                        target_addr
                    );
                }

                Err(format!("Tunnel failed: {}", e).into())
            }
            Err(_timeout) => {
                let duration = start_time.elapsed();
                error!(?duration, "[TUNNEL-TASK] 터널 타임아웃: 5분 후 종료");

                // 터널 타임아웃 이벤트 전송
                if let Some(ref sender) = self.tunnel_event_sender {
                    let error_event = TunnelEvent::error(
                        target_addr.clone(),
                        client_addr,
                        "Tunnel timeout after 5 minutes".to_string(),
                        duration,
                    );
                    let request_info = error_event.to_request_info();
                    debug!("[TUNNEL-EVENT] 터널 타임아웃 이벤트 전송: {}", target_addr);
                    if let Err(e) = sender.send(request_info).await {
                        error!("[TUNNEL-EVENT] 터널 타임아웃 이벤트 전송 실패: {}", e);
                    } else {
                        info!(
                            "[TUNNEL-EVENT] 터널 타임아웃 이벤트 전송 성공: {}",
                            target_addr
                        );
                    }
                } else {
                    warn!(
                        "[TUNNEL-EVENT] tunnel_event_sender가 None입니다 (타임아웃): {}",
                        target_addr
                    );
                }

                Err("Tunnel timeout after 5 minutes".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::certificate_authority::CertificateAuthority;
    use crate::proxy::internal::InternalProxy;
    use hyper_util::{
        client::legacy::{Client, connect::HttpConnector},
        rt::TokioExecutor,
        server,
    };
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio_rustls::rustls::ServerConfig;

    struct MockCA;

    impl CertificateAuthority for MockCA {
        async fn gen_server_config(&self, _authority: &Authority) -> Arc<ServerConfig> {
            unimplemented!();
        }

        fn get_ca_cert_der(&self) -> Option<Vec<u8>> {
            None
        }

        #[cfg(feature = "native-tls-client")]
        async fn gen_pkcs12_identity(&self, _authority: &Authority) -> Option<Vec<u8>> {
            None
        }
    }

    type TestProxy = InternalProxy<HttpConnector, MockCA, crate::NoopHandler, crate::NoopHandler>;

    fn make_test_proxy() -> TestProxy {
        let connector = HttpConnector::new();
        let client = Client::builder(TokioExecutor::new()).build(connector);
        let server_builder = server::conn::auto::Builder::new(TokioExecutor::new());

        InternalProxy {
            ca: Arc::new(MockCA),
            client,
            server: server_builder,
            http_handler: crate::NoopHandler::new(),
            websocket_handler: crate::NoopHandler::new(),
            websocket_connector: None,
            client_addr: "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
            tunnel_event_sender: None,
        }
    }

    mod is_tunnel_mode_domain {
        use super::*;

        #[test]
        fn apple_domains_are_tunnel_mode() {
            let proxy = make_test_proxy();
            let test_cases = vec![
                "wps.apple.com:443",
                "gdmf.apple.com:443",
                "fbs.smoot.apple.com:443",
                "gateway.icloud.com:443",
                "setup.icloud.com:443",
                "www.apple.com:443",
            ];
            for addr in test_cases {
                let authority: Authority = addr.parse().unwrap();
                assert!(
                    proxy.is_tunnel_mode_domain(&authority),
                    "{} should be tunnel mode",
                    addr
                );
            }
        }

        #[test]
        fn non_apple_domains_are_not_tunnel_mode() {
            let proxy = make_test_proxy();
            let test_cases = vec![
                "www.google.com:443",
                "api.github.com:443",
                "example.org:443",
                "cloudflare.com:443",
            ];
            for addr in test_cases {
                let authority: Authority = addr.parse().unwrap();
                assert!(
                    !proxy.is_tunnel_mode_domain(&authority),
                    "{} should NOT be tunnel mode",
                    addr
                );
            }
        }
    }

    mod parse_http_request {
        use super::*;

        #[test]
        fn parses_valid_get_request() {
            let raw = b"GET /api/v1/data HTTP/1.1\r\nHost: example.com\r\nAccept: */*\r\n\r\n";
            let result = TestProxy::parse_http_request_from_buffer(raw);
            assert!(result.is_some());
            let (method, path, version, headers) = result.unwrap();
            assert_eq!(method, "GET");
            assert_eq!(path, "/api/v1/data");
            assert_eq!(version, "HTTP/1.1");
            assert_eq!(headers.get("host"), Some(&"example.com".to_string()));
            assert_eq!(headers.get("accept"), Some(&"*/*".to_string()));
        }

        #[test]
        fn parses_post_request() {
            let raw = b"POST /submit HTTP/1.1\r\nHost: example.com\r\nContent-Type: application/json\r\n\r\n{\"key\":\"value\"}";
            let result = TestProxy::parse_http_request_from_buffer(raw);
            assert!(result.is_some());
            let (method, path, _, headers) = result.unwrap();
            assert_eq!(method, "POST");
            assert_eq!(path, "/submit");
            assert_eq!(
                headers.get("content-type"),
                Some(&"application/json".to_string())
            );
        }

        #[test]
        fn rejects_invalid_method() {
            let raw = b"INVALID /path HTTP/1.1\r\nHost: example.com\r\n\r\n";
            let result = TestProxy::parse_http_request_from_buffer(raw);
            assert!(result.is_none());
        }

        #[test]
        fn rejects_binary_data() {
            let raw: &[u8] = &[0x16, 0x03, 0x01, 0x00, 0xFF, 0x01];
            let result = TestProxy::parse_http_request_from_buffer(raw);
            assert!(result.is_none());
        }

        #[test]
        fn rejects_empty_buffer() {
            let result = TestProxy::parse_http_request_from_buffer(b"");
            assert!(result.is_none());
        }

        #[test]
        fn rejects_incomplete_request_line() {
            let raw = b"GET\r\n";
            let result = TestProxy::parse_http_request_from_buffer(raw);
            assert!(result.is_none());
        }
    }

    mod parse_http_response {
        use super::*;

        #[test]
        fn parses_200_ok() {
            let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\n\r\nHello, World!";
            let result = TestProxy::parse_http_response_from_buffer(raw);
            assert!(result.is_some());
            let (version, status, reason, headers) = result.unwrap();
            assert_eq!(version, "HTTP/1.1");
            assert_eq!(status, 200);
            assert_eq!(reason, "OK");
            assert_eq!(headers.get("content-type"), Some(&"text/html".to_string()));
        }

        #[test]
        fn parses_404_not_found() {
            let raw = b"HTTP/1.1 404 Not Found\r\n\r\n";
            let result = TestProxy::parse_http_response_from_buffer(raw);
            assert!(result.is_some());
            let (_, status, reason, _) = result.unwrap();
            assert_eq!(status, 404);
            assert_eq!(reason, "Not Found");
        }

        #[test]
        fn parses_301_redirect() {
            let raw =
                b"HTTP/1.1 301 Moved Permanently\r\nLocation: https://example.com/new\r\n\r\n";
            let result = TestProxy::parse_http_response_from_buffer(raw);
            assert!(result.is_some());
            let (_, status, reason, headers) = result.unwrap();
            assert_eq!(status, 301);
            assert_eq!(reason, "Moved Permanently");
            assert_eq!(
                headers.get("location"),
                Some(&"https://example.com/new".to_string())
            );
        }

        #[test]
        fn rejects_invalid_status_code() {
            let raw = b"HTTP/1.1 abc Bad\r\n\r\n";
            let result = TestProxy::parse_http_response_from_buffer(raw);
            assert!(result.is_none());
        }

        #[test]
        fn rejects_binary_data() {
            let raw: &[u8] = &[0x16, 0x03, 0x01, 0x00, 0xFF];
            let result = TestProxy::parse_http_response_from_buffer(raw);
            assert!(result.is_none());
        }

        #[test]
        fn rejects_empty_buffer() {
            let result = TestProxy::parse_http_response_from_buffer(b"");
            assert!(result.is_none());
        }
    }
}
