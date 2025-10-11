use crate::{
    HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler, body::Body,
    certificate_authority::CertificateAuthority, hybrid_tls_handler::HybridTlsHandler,
    rewind::Rewind, tls_version_detector::TlsVersionDetector,
};
use futures::{Sink, Stream, StreamExt};
use http::uri::{Authority, Scheme};
use hyper::{
    Method, Request, Response, StatusCode, Uri,
    body::{Bytes, Incoming},
    header::Entry,
    service::service_fn,
    upgrade::Upgraded,
};
use hyper_util::{
    client::legacy::{Client, connect::Connect},
    rt::{TokioExecutor, TokioIo},
    server,
};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::{io::AsyncReadExt, net::TcpStream, task::JoinHandle};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::{
    Connector, WebSocketStream,
    tungstenite::{self, Message, protocol::WebSocketConfig},
};
use tracing::{Instrument, Span, error, info, info_span, instrument, warn};

fn bad_request() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::empty())
        .expect("Failed to build response")
}

fn spawn_with_trace<T: Send + Sync + 'static>(
    fut: impl Future<Output = T> + Send + 'static,
    span: Span,
) -> JoinHandle<T> {
    tokio::spawn(fut.instrument(span))
}

pub(crate) struct InternalProxy<C, CA, H, W> {
    pub ca: Arc<CA>,
    pub client: Client<C, Body>,
    pub server: server::conn::auto::Builder<TokioExecutor>,
    pub http_handler: H,
    pub websocket_handler: W,
    pub websocket_connector: Option<Connector>,
    pub client_addr: SocketAddr,
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
    fn context(&self) -> HttpContext {
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

            // 특별한 요청 감지 및 로깅
            if let Some(_host) = req_uri.host() {
                if false {
                    // SSE 스트리밍 요청 감지 (모든 도메인)
                    let accept_header = normalized_req
                        .headers()
                        .get("accept")
                        .and_then(|a| a.to_str().ok())
                        .unwrap_or("");

                    let content_type = normalized_req
                        .headers()
                        .get("content-type")
                        .and_then(|ct| ct.to_str().ok())
                        .unwrap_or("");

                    let _is_sse_request = accept_header.contains("text/event-stream")
                        || accept_header.contains("application/x-ndjson")
                        || content_type.contains("text/event-stream")
                        || content_type.contains("application/x-ndjson");
                }
            }

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
                    println!("❌ 프록시 요청 실패");
                    println!("   - URL: {}", req_uri);
                    println!("   - 메서드: {}", req_method);
                    println!("   - 호스트: {:?}", req_host);
                    println!("   - User-Agent: {:?}", req_user_agent);
                    println!("   - 오류: {}", err);
                    println!("   - 오류 타입: {:?}", err);

                    Ok(self
                        .http_handler
                        .handle_error(&ctx, err)
                        .instrument(info_span!("handle_error"))
                        .await)
                }
            }
        }
    }

    fn process_connect(mut self, mut req: Request<Body>) -> Response<Body> {
        match req.uri().authority().cloned() {
            Some(authority) => {
                let span = info_span!("process_connect");
                let fut = async move {
                    match hyper::upgrade::on(&mut req).await {
                        Ok(upgraded) => {
                            let mut upgraded = TokioIo::new(upgraded);
                            let mut buffer = [0; 11]; // ClientHello 헤더를 위해 11 bytes 필요
                            let bytes_read = match upgraded.read(&mut buffer).await {
                                Ok(bytes_read) => bytes_read,
                                Err(e) => {
                                    error!("Failed to read from upgraded connection: {}", e);
                                    return;
                                }
                            };

                            let mut upgraded = Rewind::new(
                                upgraded,
                                Bytes::copy_from_slice(buffer[..bytes_read].as_ref()),
                            );

                            if self
                                .http_handler
                                .should_intercept(&self.context(), &req)
                                .await
                            {
                                if buffer.len() >= 4 && buffer[..4] == *b"GET " {
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
                                } else if buffer[..2] == *b"\x16\x03" {
                                    // TLS 버전 감지
                                    let tls_version =
                                        TlsVersionDetector::detect_tls_version(&buffer);

                                    match tls_version {
                                        Some(version) => {
                                            info!(
                                                "🔍 TLS 버전 감지: {} - 하이브리드 핸들러 사용",
                                                version
                                            );

                                            // HybridTlsHandler 생성
                                            let hybrid_handler =
                                                match HybridTlsHandler::new(Arc::clone(&self.ca))
                                                    .await
                                                {
                                                    Ok(handler) => handler,
                                                    Err(e) => {
                                                        error!(
                                                            "❌ HybridTlsHandler 생성 실패: {}",
                                                            e
                                                        );
                                                        return;
                                                    }
                                                };

                                            // 하이브리드 TLS 연결 처리
                                            match hybrid_handler
                                                .handle_tls_connection_upgraded(
                                                    &authority, upgraded, &buffer,
                                                )
                                                .await
                                            {
                                                Ok(hybrid_stream) => {
                                                    info!(
                                                        "✅ 하이브리드 TLS 연결 성공: {}",
                                                        version
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

                                                    println!("❌ 하이브리드 TLS 연결 실패");
                                                    println!("   - 대상 서버: {}", authority);
                                                    println!("   - TLS 버전: {}", version);
                                                    println!("   - TLS 백엔드: {}", tls_backend);
                                                    println!("   - 오류: {}", e);
                                                    println!("   - 오류 타입: {:?}", e);

                                                    // TLS 관련 상세 정보
                                                    if e.to_string().contains(
                                                        "SignatureAlgorithmsExtensionRequired",
                                                    ) {
                                                        println!(
                                                            "   - TLS 문제: 서버가 SignatureAlgorithmsExtension을 요구함"
                                                        );
                                                        println!(
                                                            "   - 해결방법: TLS 1.2+ 클라이언트 사용 또는 서버 설정 확인"
                                                        );
                                                    } else if e
                                                        .to_string()
                                                        .contains("peer is incompatible")
                                                    {
                                                        println!(
                                                            "   - TLS 문제: 클라이언트-서버 호환성 문제"
                                                        );
                                                        println!(
                                                            "   - 가능한 원인: 지원하지 않는 TLS 버전, 암호화 스위트, 또는 확장"
                                                        );
                                                    } else if e.to_string().contains("certificate")
                                                    {
                                                        println!("   - TLS 문제: 인증서 관련 오류");
                                                        println!(
                                                            "   - 가능한 원인: 인증서 검증 실패, 만료된 인증서, 또는 CA 신뢰 문제"
                                                        );
                                                    }

                                                    return;
                                                }
                                            }
                                        }
                                        None => {
                                            warn!(
                                                "⚠️ TLS 버전을 감지할 수 없음, 기존 rustls로 시도"
                                            );

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
                                                    println!("❌ TLS 핸드셰이크 실패");
                                                    println!("   - 대상 서버: {}", authority);
                                                    println!("   - 오류: {}", e);
                                                    println!("   - 오류 타입: {:?}", e);

                                                    // TLS 관련 상세 정보
                                                    let error_str = e.to_string();
                                                    if error_str.contains(
                                                        "SignatureAlgorithmsExtensionRequired",
                                                    ) {
                                                        println!(
                                                            "   - TLS 문제: 서버가 SignatureAlgorithmsExtension을 요구함"
                                                        );
                                                        println!(
                                                            "   - 해결방법: TLS 1.2+ 클라이언트 사용 또는 서버 설정 확인"
                                                        );
                                                    } else if error_str
                                                        .contains("peer is incompatible")
                                                    {
                                                        println!(
                                                            "   - TLS 문제: 클라이언트-서버 호환성 문제"
                                                        );
                                                        println!(
                                                            "   - 가능한 원인: 지원하지 않는 TLS 버전, 암호화 스위트, 또는 확장"
                                                        );
                                                    } else if error_str.contains("certificate") {
                                                        println!("   - TLS 문제: 인증서 관련 오류");
                                                        println!(
                                                            "   - 가능한 원인: 인증서 검증 실패, 만료된 인증서, 또는 CA 신뢰 문제"
                                                        );
                                                    } else if error_str.contains("handshake") {
                                                        println!(
                                                            "   - TLS 문제: 핸드셰이크 프로토콜 오류"
                                                        );
                                                        println!(
                                                            "   - 가능한 원인: 프로토콜 버전 불일치, 암호화 스위트 협상 실패"
                                                        );
                                                    } else if error_str.contains("timeout") {
                                                        println!(
                                                            "   - TLS 문제: 핸드셰이크 타임아웃"
                                                        );
                                                        println!(
                                                            "   - 가능한 원인: 네트워크 지연, 서버 과부하, 또는 방화벽 차단"
                                                        );
                                                    }

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
                                        &buffer[..bytes_read]
                                    );
                                }
                            }

                            let mut server = match TcpStream::connect(authority.as_ref()).await {
                                Ok(server) => server,
                                Err(e) => {
                                    println!("❌ 업스트림 서버 연결 실패");
                                    println!("   - 대상 서버: {}", authority);
                                    println!("   - 오류: {}", e);
                                    return;
                                }
                            };

                            if let Err(e) =
                                tokio::io::copy_bidirectional(&mut upgraded, &mut server).await
                            {
                                println!("❌ 터널링 실패");
                                println!("   - 대상 서버: {}", authority);
                                println!("   - 오류: {}", e);
                            }
                        }
                        Err(e) => {
                            println!("❌ 연결 업그레이드 실패");
                            println!("   - 오류: {}", e);
                        }
                    };
                };

                spawn_with_trace(fut, span);
                Response::new(Body::empty())
            }
            None => bad_request(),
        }
    }

    #[instrument(skip_all)]
    fn upgrade_websocket(self, req: Request<Body>) -> Response<Body> {
        let original_uri = req.uri().clone();
        let _headers = req.headers().clone();

        // WebSocket 업그레이드 요청을 원본 핸들러로 전달
        let mut req = {
            let (mut parts, _) = req.into_parts();

            parts.uri = {
                let mut parts = parts.uri.into_parts();

                parts.scheme = if parts.scheme.unwrap_or(Scheme::HTTP) == Scheme::HTTP {
                    Some("ws".try_into().expect("Failed to convert scheme"))
                } else {
                    Some("wss".try_into().expect("Failed to convert scheme"))
                };

                match Uri::from_parts(parts) {
                    Ok(uri) => {
                        println!("🔄 URI 스키마 변환: {} -> {}", original_uri, uri);
                        uri
                    }
                    Err(e) => {
                        println!("❌ URI 변환 실패: {:?}", e);
                        return bad_request();
                    }
                }
            };

            Request::from_parts(parts, ())
        };

        // WebSocket 핸들러를 사용하여 터널링 구현
        // Sec-WebSocket-Protocol 헤더를 수동으로 처리하여 프로토콜 협상 지원
        let requested_protocol = req
            .headers()
            .get("sec-websocket-protocol")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let mut config = WebSocketConfig::default();
        // WebSocket 설정
        config.accept_unmasked_frames = true;
        config.max_frame_size = Some(16777216); // 16MB
        config.max_message_size = Some(67108864); // 64MB

        match hyper_tungstenite::upgrade(&mut req, Some(config)) {
            Ok((mut res, websocket)) => {
                // 클라이언트가 요청한 프로토콜이 있으면 응답에 포함
                if let Some(protocol) = requested_protocol {
                    if let Ok(header_value) = protocol.parse() {
                        res.headers_mut()
                            .insert("sec-websocket-protocol", header_value);
                    }
                }

                let span = info_span!("websocket_tunnel");
                let fut = async move {
                    match websocket.await {
                        Ok(ws) => {
                            if let Err(e) = self.handle_websocket_tunnel(ws, req).await {
                                println!("❌ WebSocket 터널 처리 실패: {}", e);
                            }
                        }
                        Err(e) => {
                            println!("❌ WebSocket 업그레이드 대기 실패: {}", e);
                        }
                    }
                };

                spawn_with_trace(fut, span);
                res.map(Body::from)
            }
            Err(e) => {
                println!("❌ WebSocket 업그레이드 실패: {:?}", e);
                println!("📍 실패한 요청 URI: {}", req.uri());
                println!("🔧 실패한 요청 메서드: {}", req.method());
                bad_request()
            }
        }
    }

    #[instrument(skip_all)]
    async fn handle_websocket_tunnel(
        self,
        client_socket: WebSocketStream<TokioIo<Upgraded>>,
        req: Request<()>,
    ) -> Result<(), tungstenite::Error> {
        // WebSocket 터널링 구현
        let uri = req.uri().clone();

        println!("🌐 WebSocket 터널 시작: {}", uri);
        println!("🔗 대상 서버: {}", uri.host().unwrap_or("unknown"));
        println!(
            "🔌 포트: {}",
            uri.port_u16()
                .unwrap_or(if uri.scheme_str() == Some("wss") {
                    443
                } else {
                    80
                })
        );

        // 서버에 WebSocket 연결
        println!("🔌 서버에 WebSocket 연결 시도 중...");

        #[cfg(any(feature = "rustls-client", feature = "native-tls-client"))]
        let (server_socket, response) = {
            println!("🔐 TLS 클라이언트 기능 활성화됨");
            let mut ws_config = WebSocketConfig::default();
            ws_config.accept_unmasked_frames = true;
            ws_config.max_frame_size = Some(16777216); // 16MB
            ws_config.max_message_size = Some(67108864); // 64MB
            ws_config.read_buffer_size = 262144; // 256KB
            ws_config.write_buffer_size = 262144; // 256KB

            println!("⚙️ 서버 연결용 WebSocket 설정: {:?}", ws_config);

            match tokio_tungstenite::connect_async_tls_with_config(
                req,
                Some(ws_config),
                false,
                self.websocket_connector,
            )
            .await
            {
                Ok(result) => {
                    println!("✅ TLS WebSocket 연결 성공");
                    result
                }
                Err(e) => {
                    println!("❌ TLS WebSocket 연결 실패: {}", e);
                    println!("📍 연결 시도한 URI: {}", uri);
                    println!("🔧 연결 시도한 호스트: {}", uri.host().unwrap_or("unknown"));
                    println!(
                        "🔌 연결 시도한 포트: {}",
                        uri.port_u16()
                            .unwrap_or(if uri.scheme_str() == Some("wss") {
                                443
                            } else {
                                80
                            })
                    );
                    return Err(e);
                }
            }
        };

        #[cfg(not(any(feature = "rustls-client", feature = "native-tls-client")))]
        let (server_socket, response) = {
            println!("🔓 일반 WebSocket 연결 (TLS 기능 비활성화)");
            let mut ws_config = WebSocketConfig::default();
            ws_config.accept_unmasked_frames = true;
            ws_config.max_frame_size = Some(16777216); // 16MB
            ws_config.max_message_size = Some(67108864); // 64MB
            ws_config.read_buffer_size = 262144; // 256KB
            ws_config.write_buffer_size = 262144; // 256KB

            println!("⚙️ 일반 연결용 WebSocket 설정: {:?}", ws_config);

            match tokio_tungstenite::connect_async_with_config(req, Some(ws_config)).await {
                Ok(result) => {
                    println!("✅ 일반 WebSocket 연결 성공");
                    result
                }
                Err(e) => {
                    println!("❌ 일반 WebSocket 연결 실패: {}", e);
                    println!("📍 연결 시도한 URI: {}", uri);
                    println!("🔧 연결 시도한 호스트: {}", uri.host().unwrap_or("unknown"));
                    println!(
                        "🔌 연결 시도한 포트: {}",
                        uri.port_u16()
                            .unwrap_or(if uri.scheme_str() == Some("wss") {
                                443
                            } else {
                                80
                            })
                    );
                    return Err(e);
                }
            }
        };

        println!("✅ 서버 WebSocket 연결 성공");
        println!("📤 서버 응답 상태: {:?}", response.status());

        // 서버 응답 헤더 로그
        for (name, value) in response.headers() {
            if name.as_str().starts_with("sec-websocket") {
                println!("📋 서버 응답 헤더 {}: {:?}", name, value);
            }
        }

        // WebSocket 핸들러를 사용하여 터널링 구현
        let (server_sink, server_stream) = server_socket.split();
        let (client_sink, client_stream) = client_socket.split();

        let InternalProxy {
            websocket_handler, ..
        } = self;

        // WebSocket 핸들러를 사용하여 메시지 전달
        println!("🔄 서버→클라이언트 메시지 전달기 시작");
        spawn_message_forwarder(
            server_stream,
            client_sink,
            websocket_handler.clone(),
            WebSocketContext::ServerToClient {
                src: uri.clone(),
                dst: self.client_addr,
            },
        );

        println!("🔄 클라이언트→서버 메시지 전달기 시작");
        spawn_message_forwarder(
            client_stream,
            server_sink,
            websocket_handler,
            WebSocketContext::ClientToServer {
                src: self.client_addr,
                dst: uri,
            },
        );

        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_websocket(
        self,
        client_socket: WebSocketStream<TokioIo<Upgraded>>,
        req: Request<()>,
    ) -> Result<(), tungstenite::Error> {
        let uri = req.uri().clone();

        #[cfg(any(feature = "rustls-client", feature = "native-tls-client"))]
        let (server_socket, _) = tokio_tungstenite::connect_async_tls_with_config(
            req,
            None,
            false,
            self.websocket_connector,
        )
        .await?;

        #[cfg(not(any(feature = "rustls-client", feature = "native-tls-client")))]
        let (server_socket, _) = tokio_tungstenite::connect_async(req).await?;

        let (server_sink, server_stream) = server_socket.split();
        let (client_sink, client_stream) = client_socket.split();

        let InternalProxy {
            websocket_handler, ..
        } = self;

        spawn_message_forwarder(
            server_stream,
            client_sink,
            websocket_handler.clone(),
            WebSocketContext::ServerToClient {
                src: uri.clone(),
                dst: self.client_addr,
            },
        );

        spawn_message_forwarder(
            client_stream,
            server_sink,
            websocket_handler,
            WebSocketContext::ClientToServer {
                src: self.client_addr,
                dst: uri,
            },
        );

        Ok(())
    }

    #[instrument(skip_all)]
    async fn serve_stream<I>(
        self,
        stream: I,
        scheme: Scheme,
        authority: Authority,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        I: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
    {
        let service = service_fn(|mut req| {
            if req.version() == hyper::Version::HTTP_10 || req.version() == hyper::Version::HTTP_11
            {
                let (mut parts, body) = req.into_parts();

                parts.uri = {
                    let mut parts = parts.uri.into_parts();
                    parts.scheme = Some(scheme.clone());
                    parts.authority = Some(authority.clone());
                    Uri::from_parts(parts).expect("Failed to build URI")
                };

                req = Request::from_parts(parts, body);
            };

            self.clone().proxy(req)
        });

        self.server
            .serve_connection_with_upgrades(stream, service)
            .await
    }
}

fn spawn_message_forwarder(
    stream: impl Stream<Item = Result<Message, tungstenite::Error>> + Unpin + Send + 'static,
    sink: impl Sink<Message, Error = tungstenite::Error> + Unpin + Send + 'static,
    handler: impl WebSocketHandler,
    ctx: WebSocketContext,
) {
    let span = info_span!("message_forwarder", context = ?ctx);
    let fut = handler.handle_websocket(ctx, stream, sink);
    spawn_with_trace(fut, span);
}

#[instrument(skip_all)]
fn normalize_request<T>(mut req: Request<T>) -> Request<T> {
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
        }
    }

    mod bad_request {
        use super::*;

        #[test]
        fn correct_status() {
            let res = bad_request();
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

            let req = normalize_request(req);

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

            let req = normalize_request(req);

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
