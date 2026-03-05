use super::helpers::{bad_request, spawn_with_trace};
use crate::{
    HttpContext, HttpHandler, RequestOrResponse, WebSocketHandler, body::Body,
    certificate_authority::CertificateAuthority, hybrid_tls_handler::HybridTlsHandler,
    rewind::Rewind, tls_version_detector::TlsVersionDetector,
};
use http::uri::{Authority, Scheme};
use hyper::{
    Method, Request, Response, Uri,
    body::{Bytes, Incoming},
    header::Entry,
    service::service_fn,
};
use hyper_util::{
    client::legacy::{Client, connect::Connect},
    rt::{TokioExecutor, TokioIo},
    server,
};
use proxy_v2_models::RequestInfo;
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;
use tokio::{io::AsyncReadExt, net::TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::Connector;
use tracing::{Instrument, debug, error, info, info_span, instrument, warn};

pub(crate) struct InternalProxy<C, CA, H, W> {
    pub(crate) ca: Arc<CA>,
    pub(crate) client: Client<C, Body>,
    pub(crate) server: server::conn::auto::Builder<TokioExecutor>,
    pub(crate) http_handler: H,
    pub(crate) websocket_handler: W,
    pub(crate) websocket_connector: Option<Connector>,
    pub(crate) client_addr: SocketAddr,
    pub(crate) tunnel_event_sender: Option<mpsc::Sender<RequestInfo>>,
}

impl<C, CA, H, W> Clone for InternalProxy<C, CA, H, W>
where
    C: Clone,
    H: Clone,
    W: Clone,
{
    fn clone(&self) -> Self {
        InternalProxy {
            ca: Arc::clone(&self.ca),
            client: self.client.clone(),
            server: self.server.clone(),
            http_handler: self.http_handler.clone(),
            websocket_handler: self.websocket_handler.clone(),
            websocket_connector: self.websocket_connector.clone(),
            client_addr: self.client_addr,
            tunnel_event_sender: self.tunnel_event_sender.clone(),
        }
    }
}

impl<C, CA, H, W> InternalProxy<C, CA, H, W>
where
    C: Connect + Clone + Send + Sync + 'static,
    CA: CertificateAuthority,
    H: HttpHandler,
    W: WebSocketHandler,
{
    pub(crate) fn context(&self) -> HttpContext {
        HttpContext {
            client_addr: self.client_addr,
        }
    }

    #[instrument(
        skip_all,
        fields(
            version = ?req.version(),
            method = %req.method(),
            uri=%req.uri(),
            client_addr = %self.client_addr,
        )
    )]
    pub(crate) async fn proxy(
        mut self,
        req: Request<Incoming>,
    ) -> Result<Response<Body>, Infallible> {
        let ctx = self.context();

        let req = match self
            .http_handler
            .handle_request(&ctx, req.map(Body::from))
            .instrument(info_span!("handle_request"))
            .await
        {
            RequestOrResponse::Request(req) => req,
            RequestOrResponse::Response(res) => return Ok(res),
        };

        if req.method() == Method::CONNECT {
            Ok(self.process_connect(req))
        } else if hyper_tungstenite::is_upgrade_request(&req) {
            Ok(self.upgrade_websocket(req))
        } else {
            let normalized_req = normalize_request(req);

            // 요청 정보 미리 추출 (에러 로깅용)
            let req_uri = normalized_req.uri().clone();
            let req_method = normalized_req.method().clone();
            let req_host = normalized_req.headers().get("host").cloned();
            let req_user_agent = normalized_req.headers().get("user-agent").cloned();

            // SSE 요청인 경우 추가 로깅
            let _is_sse_request = normalized_req
                .headers()
                .get("accept")
                .and_then(|a| a.to_str().ok())
                .map(|a| a.contains("text/event-stream") || a.contains("application/x-ndjson"))
                .unwrap_or(false);

            let res = self
                .client
                .request(normalized_req)
                .instrument(info_span!("proxy_request"))
                .await;

            match res {
                Ok(res) => {
                    // 응답 수신 시간 기록
                    let _response_received_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis();

                    // 스트리밍 응답 감지 및 로깅
                    let content_type = res
                        .headers()
                        .get("content-type")
                        .and_then(|ct| ct.to_str().ok())
                        .unwrap_or("");

                    let transfer_encoding = res
                        .headers()
                        .get("transfer-encoding")
                        .and_then(|te| te.to_str().ok())
                        .unwrap_or("");

                    let is_streaming = content_type.contains("text/event-stream")
                        || content_type.contains("application/x-ndjson");

                    let is_chunked = transfer_encoding.contains("chunked");

                    // SSE 스트리밍 요청 감지
                    let is_sse_request = content_type.contains("text/event-stream")
                        || content_type.contains("application/x-ndjson");

                    // ces/v1/t는 강제로 스트리밍으로 처리
                    let is_ces_v1_t = req_uri.path().contains("/ces/v1/t");
                    let force_streaming =
                        is_streaming || is_chunked || is_sse_request || is_ces_v1_t;

                    // 응답 전달 시작 시간 기록
                    let _response_delivery_start_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis();

                    // 스트리밍 응답인 경우 헤더를 더 강력하게 최적화
                    let response = if force_streaming {
                        // 스트리밍 응답 헤더 강화
                        let (mut parts, body) = res.into_parts();

                        // 스트리밍을 위한 핵심 헤더 설정
                        parts.headers.insert(
                            "Cache-Control",
                            "no-cache, no-store, must-revalidate".parse().unwrap(),
                        );
                        parts
                            .headers
                            .insert("Connection", "keep-alive".parse().unwrap());
                        parts
                            .headers
                            .insert("Transfer-Encoding", "chunked".parse().unwrap());
                        parts.headers.remove("content-length");

                        // 추가 스트리밍 최적화 헤더
                        parts
                            .headers
                            .insert("X-Accel-Buffering", "no".parse().unwrap()); // Nginx 버퍼링 방지
                        parts
                            .headers
                            .insert("X-Content-Type-Options", "nosniff".parse().unwrap());

                        Response::from_parts(parts, Body::from(body))
                    } else {
                        res.map(Body::from)
                    };

                    Ok(self
                        .http_handler
                        .handle_response(&ctx, response)
                        .instrument(info_span!("handle_response"))
                        .await)
                }
                Err(err) => {
                    // 실패한 요청 정보 로깅
                    error!(
                        url = %req_uri,
                        method = %req_method,
                        host = ?req_host,
                        user_agent = ?req_user_agent,
                        error = %err,
                        error_type = ?err,
                        "프록시 요청 실패"
                    );

                    Ok(self
                        .http_handler
                        .handle_error(&ctx, err)
                        .instrument(info_span!("handle_error"))
                        .await)
                }
            }
        }
    }

    pub(crate) fn process_connect(mut self, mut req: Request<Body>) -> Response<Body> {
        match req.uri().authority().cloned() {
            Some(authority) => {
                // 터널 모드 도메인 확인 (CONNECT 요청 처리 전에 먼저 확인)
                if self.is_tunnel_mode_domain(&authority) {
                    info!("[TUNNEL-MODE] 터널 모드 도메인 감지: {}", authority);

                    // CONNECT 요청에 대한 200 Connection Established 응답 생성
                    let response = Response::builder()
                        .status(200)
                        .header("Connection", "keep-alive")
                        .body(Body::empty())
                        .unwrap();

                    // 백그라운드에서 터널 모드 처리
                    let self_clone = self.clone();
                    tokio::spawn(async move {
                        match hyper::upgrade::on(&mut req).await {
                            Ok(upgraded) => {
                                let upgraded = TokioIo::new(upgraded);
                                let upgraded = Rewind::new(upgraded, Bytes::new());

                                if let Err(e) =
                                    self_clone.handle_tunnel_mode(&authority, upgraded).await
                                {
                                    error!("[TUNNEL-MODE] 터널 모드 처리 실패: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("[TUNNEL-MODE] CONNECT 업그레이드 실패: {}", e);
                            }
                        }
                    });

                    return response;
                }

                let span = info_span!("process_connect");
                let fut = async move {
                    match hyper::upgrade::on(&mut req).await {
                        Ok(upgraded) => {
                            let mut upgraded = TokioIo::new(upgraded);

                            // 먼저 TLS 레코드 헤더를 읽어서 전체 길이를 파악
                            let mut header_buffer = [0; 5]; // TLS 레코드 헤더 (5 bytes)
                            let header_bytes_read = match upgraded.read(&mut header_buffer).await {
                                Ok(bytes_read) => bytes_read,
                                Err(e) => {
                                    error!(
                                        "Failed to read TLS header from upgraded connection: {}",
                                        e
                                    );
                                    return;
                                }
                            };

                            if header_bytes_read < 5 {
                                error!("TLS header too short: {} bytes", header_bytes_read);
                                return;
                            }

                            // 레코드 길이 계산 (bytes 3-4)
                            let record_length =
                                u16::from_be_bytes([header_buffer[3], header_buffer[4]]) as usize;
                            let total_expected_length = 5 + record_length; // 헤더(5) + 레코드 길이

                            debug!(
                                record_type = format_args!("0x{:02x}", header_buffer[0]),
                                record_version = format_args!(
                                    "0x{:02x}{:02x}",
                                    header_buffer[1], header_buffer[2]
                                ),
                                record_length,
                                total_expected_length,
                                "[BUFFER-READ] TLS 레코드 분석"
                            );

                            // 전체 ClientHello 메시지를 읽기 위한 버퍼 생성
                            let mut full_buffer = vec![0; total_expected_length];
                            full_buffer[..5].copy_from_slice(&header_buffer);

                            // 나머지 데이터 읽기
                            let remaining_bytes = total_expected_length - 5;
                            if remaining_bytes > 0 {
                                let remaining_bytes_read =
                                    match upgraded.read(&mut full_buffer[5..]).await {
                                        Ok(bytes_read) => bytes_read,
                                        Err(e) => {
                                            error!("Failed to read remaining TLS data: {}", e);
                                            return;
                                        }
                                    };

                                debug!(
                                    header_bytes = 5,
                                    remaining_bytes_read,
                                    remaining_bytes_expected = remaining_bytes,
                                    total_bytes_read = 5 + remaining_bytes_read,
                                    "[BUFFER-READ] 데이터 읽기 완료"
                                );

                                if remaining_bytes_read < remaining_bytes {
                                    warn!(
                                        remaining_bytes_read,
                                        remaining_bytes_expected = remaining_bytes,
                                        "전체 ClientHello가 완전히 읽히지 않음"
                                    );
                                }
                            }

                            let mut upgraded =
                                Rewind::new(upgraded, Bytes::copy_from_slice(&full_buffer));

                            if self
                                .http_handler
                                .should_intercept(&self.context(), &req)
                                .await
                            {
                                if full_buffer.len() >= 4 && full_buffer[..4] == *b"GET " {
                                    if let Err(e) = self
                                        .serve_stream(
                                            TokioIo::new(upgraded),
                                            Scheme::HTTP,
                                            authority,
                                        )
                                        .await
                                    {
                                        error!("WebSocket connect error: {}", e);
                                    }

                                    return;
                                } else if full_buffer.len() >= 2 && full_buffer[..2] == *b"\x16\x03"
                                {
                                    // TLS 버전 감지
                                    let tls_version =
                                        TlsVersionDetector::detect_tls_version(&full_buffer);

                                    match tls_version {
                                        Some(version) => {
                                            debug!(
                                                %version,
                                                "TLS 버전 감지 - 하이브리드 핸들러 사용"
                                            );

                                            // HybridTlsHandler 생성
                                            let hybrid_handler =
                                                match HybridTlsHandler::new(Arc::clone(&self.ca))
                                                    .await
                                                {
                                                    Ok(handler) => handler,
                                                    Err(e) => {
                                                        error!(
                                                            error = %e,
                                                            "HybridTlsHandler 생성 실패"
                                                        );
                                                        return;
                                                    }
                                                };

                                            // 하이브리드 TLS 연결 처리
                                            match hybrid_handler
                                                .handle_tls_connection_upgraded(
                                                    &authority,
                                                    upgraded,
                                                    &full_buffer,
                                                )
                                                .await
                                            {
                                                Ok(hybrid_stream) => {
                                                    info!(
                                                        %version,
                                                        "하이브리드 TLS 연결 성공"
                                                    );
                                                    let stream = TokioIo::new(hybrid_stream);

                                                    if let Err(e) = self
                                                        .serve_stream(
                                                            stream,
                                                            Scheme::HTTPS,
                                                            authority.clone(),
                                                        )
                                                        .await
                                                    {
                                                        if !e.to_string().starts_with(
                                                            "error shutting down connection",
                                                        ) {
                                                            error!("HTTPS connect error: {}", e);
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    // 오류 메시지에서 TLS 백엔드 확인
                                                    let error_str = e.to_string();
                                                    let tls_backend =
                                                        if error_str.contains("rustls") {
                                                            "RUSTLS"
                                                        } else if error_str.contains("native-tls")
                                                            || error_str.contains("openssl")
                                                        {
                                                            "NATIVE-TLS"
                                                        } else {
                                                            "UNKNOWN"
                                                        };

                                                    let tls_hint = if error_str.contains(
                                                        "SignatureAlgorithmsExtensionRequired",
                                                    ) {
                                                        "서버가 SignatureAlgorithmsExtension을 요구함 - TLS 1.2+ 클라이언트 사용 또는 서버 설정 확인"
                                                    } else if error_str
                                                        .contains("peer is incompatible")
                                                    {
                                                        "클라이언트-서버 호환성 문제 - 지원하지 않는 TLS 버전, 암호화 스위트, 또는 확장"
                                                    } else if error_str.contains("certificate") {
                                                        "인증서 관련 오류 - 인증서 검증 실패, 만료된 인증서, 또는 CA 신뢰 문제"
                                                    } else {
                                                        ""
                                                    };

                                                    error!(
                                                        authority = %authority,
                                                        tls_version = %version,
                                                        tls_backend,
                                                        error = %e,
                                                        error_type = ?e,
                                                        tls_hint,
                                                        "하이브리드 TLS 연결 실패"
                                                    );

                                                    return;
                                                }
                                            }
                                        }
                                        None => {
                                            warn!("TLS 버전을 감지할 수 없음, 기존 rustls로 시도");

                                            // 기존 rustls 로직 사용
                                            let server_config = self
                                                .ca
                                                .gen_server_config(&authority)
                                                .instrument(info_span!("gen_server_config"))
                                                .await;

                                            let stream = match TlsAcceptor::from(server_config)
                                                .accept(upgraded)
                                                .await
                                            {
                                                Ok(stream) => TokioIo::new(stream),
                                                Err(e) => {
                                                    let error_str = e.to_string();
                                                    let tls_hint = if error_str.contains(
                                                        "SignatureAlgorithmsExtensionRequired",
                                                    ) {
                                                        "서버가 SignatureAlgorithmsExtension을 요구함 - TLS 1.2+ 클라이언트 사용 또는 서버 설정 확인"
                                                    } else if error_str
                                                        .contains("peer is incompatible")
                                                    {
                                                        "클라이언트-서버 호환성 문제 - 지원하지 않는 TLS 버전, 암호화 스위트, 또는 확장"
                                                    } else if error_str.contains("certificate") {
                                                        "인증서 관련 오류 - 인증서 검증 실패, 만료된 인증서, 또는 CA 신뢰 문제"
                                                    } else if error_str.contains("handshake") {
                                                        "핸드셰이크 프로토콜 오류 - 프로토콜 버전 불일치, 암호화 스위트 협상 실패"
                                                    } else if error_str.contains("timeout") {
                                                        "핸드셰이크 타임아웃 - 네트워크 지연, 서버 과부하, 또는 방화벽 차단"
                                                    } else {
                                                        ""
                                                    };

                                                    error!(
                                                        authority = %authority,
                                                        error = %e,
                                                        error_type = ?e,
                                                        tls_hint,
                                                        "TLS 핸드셰이크 실패"
                                                    );

                                                    return;
                                                }
                                            };

                                            if let Err(e) = self
                                                .serve_stream(
                                                    stream,
                                                    Scheme::HTTPS,
                                                    authority.clone(),
                                                )
                                                .await
                                            {
                                                if !e
                                                    .to_string()
                                                    .starts_with("error shutting down connection")
                                                {
                                                    error!("HTTPS connect error: {}", e);
                                                }
                                            }
                                        }
                                    }

                                    return;
                                } else {
                                    warn!(
                                        "Unknown protocol, read '{:02X?}' from upgraded connection",
                                        &full_buffer[..full_buffer.len().min(16)]
                                    );
                                }
                            }

                            let mut server = match TcpStream::connect(authority.as_ref()).await {
                                Ok(server) => server,
                                Err(e) => {
                                    error!(
                                        authority = %authority,
                                        error = %e,
                                        "업스트림 서버 연결 실패"
                                    );
                                    return;
                                }
                            };

                            if let Err(e) =
                                tokio::io::copy_bidirectional(&mut upgraded, &mut server).await
                            {
                                error!(
                                    authority = %authority,
                                    error = %e,
                                    "터널링 실패"
                                );
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "연결 업그레이드 실패");
                        }
                    };
                };

                spawn_with_trace(fut, span);
                Response::new(Body::empty())
            }
            None => bad_request(),
        }
    }

    pub(crate) async fn serve_stream<I>(
        self,
        stream: I,
        scheme: Scheme,
        authority: Authority,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        I: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
    {
        debug!(
            %authority,
            ?scheme,
            "[SERVE-STREAM] 스트림 서빙 시작"
        );

        let proxy_clone = self.clone();
        let service = service_fn({
            let authority = authority.clone();
            let scheme = scheme.clone();
            move |mut req| {
                debug!(
                    method = %req.method(),
                    uri = %req.uri(),
                    version = ?req.version(),
                    "[SERVE-STREAM] HTTP 요청 수신"
                );

                if req.version() == hyper::Version::HTTP_10
                    || req.version() == hyper::Version::HTTP_11
                {
                    let (mut parts, body) = req.into_parts();

                    parts.uri = {
                        let mut parts = parts.uri.into_parts();
                        parts.scheme = Some(scheme.clone());
                        parts.authority = Some(authority.clone());
                        Uri::from_parts(parts).expect("Failed to build URI")
                    };

                    req = Request::from_parts(parts, body);
                    debug!(uri = %req.uri(), "[SERVE-STREAM] URI 재구성 완료");
                };

                debug!("[SERVE-STREAM] 프록시 요청 전달 시작");
                proxy_clone.clone().proxy(req)
            }
        });

        debug!("[SERVE-STREAM] 서버 연결 시작 - serve_connection_with_upgrades 호출");
        let result = self
            .server
            .serve_connection_with_upgrades(stream, service)
            .await;

        match result {
            Ok(_) => {
                info!("[SERVE-STREAM] 스트림 서빙 완료: {}", authority);
                Ok(())
            }
            Err(e) => {
                if e.to_string().starts_with("error shutting down connection") {
                    debug!(
                        %authority,
                        "[SERVE-STREAM] 연결 종료 중 에러 (무시)"
                    );
                    Ok(())
                } else {
                    error!(
                        %authority,
                        error = %e,
                        "[SERVE-STREAM] 스트림 서빙 실패"
                    );
                    Err(e)
                }
            }
        }
    }
}

#[instrument(skip_all)]
pub(crate) fn normalize_request<T>(mut req: Request<T>) -> Request<T> {
    // Hyper will automatically add a Host header if needed.
    req.headers_mut().remove(hyper::header::HOST);

    // HTTP/2 supports multiple cookie headers, but HTTP/1.x only supports one.
    if let Entry::Occupied(mut cookies) = req.headers_mut().entry(hyper::header::COOKIE) {
        let joined_cookies = bstr::join(b"; ", cookies.iter());
        cookies.insert(joined_cookies.try_into().expect("Failed to join cookies"));
    }

    *req.version_mut() = hyper::Version::HTTP_11;
    req
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::StatusCode;
    use hyper_util::client::legacy::connect::HttpConnector;
    use tokio_rustls::rustls::ServerConfig;

    struct CA;

    impl CertificateAuthority for CA {
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

    fn build_proxy() -> InternalProxy<HttpConnector, CA, crate::NoopHandler, crate::NoopHandler> {
        InternalProxy {
            ca: Arc::new(CA),
            client: Client::builder(TokioExecutor::new()).build(HttpConnector::new()),
            server: server::conn::auto::Builder::new(TokioExecutor::new()),
            http_handler: crate::NoopHandler::new(),
            websocket_handler: crate::NoopHandler::new(),
            websocket_connector: None,
            client_addr: "127.0.0.1:8080".parse().unwrap(),
            tunnel_event_sender: None,
        }
    }

    mod bad_request {
        use super::*;

        #[test]
        fn correct_status() {
            let res = crate::proxy::helpers::bad_request();
            assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        }
    }

    mod normalize_request {
        use super::*;

        #[test]
        fn removes_host_header() {
            let req = Request::builder()
                .uri("http://example.com/")
                .header(hyper::header::HOST, "example.com")
                .body(())
                .unwrap();

            let req = super::normalize_request(req);

            assert_eq!(req.headers().get(hyper::header::HOST), None);
        }

        #[test]
        fn joins_cookies() {
            let req = Request::builder()
                .uri("http://example.com/")
                .header(hyper::header::COOKIE, "foo=bar")
                .header(hyper::header::COOKIE, "baz=qux")
                .body(())
                .unwrap();

            let req = super::normalize_request(req);

            assert_eq!(
                req.headers().get_all(hyper::header::COOKIE).iter().count(),
                1
            );

            assert_eq!(
                req.headers().get(hyper::header::COOKIE),
                Some(&"foo=bar; baz=qux".parse().unwrap())
            );
        }
    }

    mod process_connect {
        use super::*;

        #[test]
        fn returns_bad_request_if_missing_authority() {
            let proxy = build_proxy();

            let req = Request::builder()
                .uri("/foo/bar?baz")
                .body(Body::empty())
                .unwrap();

            let res = proxy.process_connect(req);

            assert_eq!(res.status(), StatusCode::BAD_REQUEST)
        }
    }

    mod upgrade_websocket {
        use super::*;

        #[test]
        fn returns_bad_request_if_missing_authority() {
            let proxy = build_proxy();

            let req = Request::builder()
                .uri("/foo/bar?baz")
                .body(Body::empty())
                .unwrap();

            let res = proxy.upgrade_websocket(req);

            assert_eq!(res.status(), StatusCode::BAD_REQUEST)
        }

        #[test]
        fn returns_bad_request_if_missing_headers() {
            let proxy = build_proxy();

            let req = Request::builder()
                .uri("http://example.com/foo/bar?baz")
                .body(Body::empty())
                .unwrap();

            let res = proxy.upgrade_websocket(req);

            assert_eq!(res.status(), StatusCode::BAD_REQUEST)
        }
    }
}
