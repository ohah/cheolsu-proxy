use super::helpers::{bad_request, spawn_with_trace};
use super::internal::InternalProxy;
use crate::{
    Body, HttpHandler, WebSocketContext, WebSocketHandler,
    certificate_authority::CertificateAuthority,
    websocket_registry::{WebSocketInjector, WebSocketRegistry},
};
use futures::{Sink, SinkExt, Stream, StreamExt};
use http::uri::{Scheme, Uri};
use hyper::{Request, Response, upgrade::Upgraded};
use hyper_util::{client::legacy::connect::Connect, rt::TokioIo};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{self, Message, protocol::WebSocketConfig},
};
use tracing::{debug, error, info, info_span, instrument, warn};

impl<C, CA, H, W> InternalProxy<C, CA, H, W>
where
    C: Connect + Clone + Send + Sync + 'static,
    CA: CertificateAuthority,
    H: HttpHandler,
    W: WebSocketHandler,
{
    #[instrument(skip_all)]
    pub(crate) fn upgrade_websocket(self, req: Request<Body>) -> Response<Body> {
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
                        debug!(from = %original_uri, to = %uri, "URI 스키마 변환");
                        uri
                    }
                    Err(e) => {
                        error!(error = ?e, "URI 변환 실패");
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
                                error!(error = %e, "WebSocket 터널 처리 실패");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "WebSocket 업그레이드 대기 실패");
                        }
                    }
                };

                spawn_with_trace(fut, span);
                res.map(Body::from)
            }
            Err(e) => {
                error!(
                    error = ?e,
                    uri = %req.uri(),
                    method = %req.method(),
                    "WebSocket 업그레이드 실패"
                );
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

        info!(
            %uri,
            host = uri.host().unwrap_or("unknown"),
            port = uri.port_u16().unwrap_or(if uri.scheme_str() == Some("wss") { 443 } else { 80 }),
            "WebSocket 터널 시작"
        );

        // 서버에 WebSocket 연결
        debug!("서버에 WebSocket 연결 시도 중");

        #[cfg(any(feature = "rustls-client", feature = "native-tls-client"))]
        let (server_socket, response) = {
            debug!("TLS 클라이언트 기능 활성화됨");
            let mut ws_config = WebSocketConfig::default();
            ws_config.accept_unmasked_frames = true;
            ws_config.max_frame_size = Some(16777216); // 16MB
            ws_config.max_message_size = Some(67108864); // 64MB
            ws_config.read_buffer_size = 262144; // 256KB
            ws_config.write_buffer_size = 262144; // 256KB

            debug!(config = ?ws_config, "서버 연결용 WebSocket 설정");

            match tokio_tungstenite::connect_async_tls_with_config(
                req,
                Some(ws_config),
                false,
                self.websocket_connector,
            )
            .await
            {
                Ok(result) => {
                    info!("TLS WebSocket 연결 성공");
                    result
                }
                Err(e) => {
                    error!(
                        error = %e,
                        %uri,
                        host = uri.host().unwrap_or("unknown"),
                        port = uri.port_u16().unwrap_or(if uri.scheme_str() == Some("wss") { 443 } else { 80 }),
                        "TLS WebSocket 연결 실패"
                    );
                    return Err(e);
                }
            }
        };

        #[cfg(not(any(feature = "rustls-client", feature = "native-tls-client")))]
        let (server_socket, response) = {
            debug!("일반 WebSocket 연결 (TLS 기능 비활성화)");
            let mut ws_config = WebSocketConfig::default();
            ws_config.accept_unmasked_frames = true;
            ws_config.max_frame_size = Some(16777216); // 16MB
            ws_config.max_message_size = Some(67108864); // 64MB
            ws_config.read_buffer_size = 262144; // 256KB
            ws_config.write_buffer_size = 262144; // 256KB

            debug!(config = ?ws_config, "일반 연결용 WebSocket 설정");

            match tokio_tungstenite::connect_async_with_config(req, Some(ws_config)).await {
                Ok(result) => {
                    info!("일반 WebSocket 연결 성공");
                    result
                }
                Err(e) => {
                    error!(
                        error = %e,
                        %uri,
                        host = uri.host().unwrap_or("unknown"),
                        port = uri.port_u16().unwrap_or(if uri.scheme_str() == Some("wss") { 443 } else { 80 }),
                        "일반 WebSocket 연결 실패"
                    );
                    return Err(e);
                }
            }
        };

        info!(status = ?response.status(), "서버 WebSocket 연결 성공");

        // 서버 응답 헤더 로그
        for (name, value) in response.headers() {
            if name.as_str().starts_with("sec-websocket") {
                debug!(header_name = %name, header_value = ?value, "서버 응답 헤더");
            }
        }

        // WebSocket 핸들러를 사용하여 터널링 구현
        let (server_sink, server_stream) = server_socket.split();
        let (client_sink, client_stream) = client_socket.split();

        // 주입 채널 생성
        let (inject_to_client_tx, inject_to_client_rx) = mpsc::channel::<Message>(32);
        let (inject_to_server_tx, inject_to_server_rx) = mpsc::channel::<Message>(32);

        // 레지스트리에 등록
        let conn_id = uri.to_string();
        if let Some(ref registry) = self.websocket_registry {
            let injector =
                WebSocketInjector::new(inject_to_client_tx.clone(), inject_to_server_tx.clone());
            registry.register(conn_id.clone(), injector).await;
        }

        let InternalProxy {
            websocket_handler,
            websocket_registry,
            ..
        } = self;

        // 서버→클라이언트 (+ 주입 메시지)
        debug!("서버→클라이언트 메시지 전달기 시작");
        let registry_clone = websocket_registry.clone();
        let conn_id_clone = conn_id.clone();
        spawn_message_forwarder_with_inject(
            server_stream,
            client_sink,
            inject_to_client_rx,
            websocket_handler.clone(),
            WebSocketContext::ServerToClient {
                src: uri.clone(),
                dst: self.client_addr,
            },
            registry_clone,
            conn_id_clone,
        );

        // 클라이언트→서버 (+ 주입 메시지)
        debug!("클라이언트→서버 메시지 전달기 시작");
        spawn_message_forwarder_with_inject(
            client_stream,
            server_sink,
            inject_to_server_rx,
            websocket_handler,
            WebSocketContext::ClientToServer {
                src: self.client_addr,
                dst: uri,
            },
            websocket_registry,
            conn_id,
        );

        Ok(())
    }
}

/// 주입 채널을 포함한 메시지 전달기
fn spawn_message_forwarder_with_inject(
    stream: impl Stream<Item = Result<Message, tungstenite::Error>> + Unpin + Send + 'static,
    sink: impl Sink<Message, Error = tungstenite::Error> + Unpin + Send + 'static,
    mut inject_rx: mpsc::Receiver<Message>,
    mut handler: impl WebSocketHandler,
    ctx: WebSocketContext,
    registry: Option<WebSocketRegistry>,
    conn_id: String,
) {
    let span = info_span!("message_forwarder_inject", context = ?ctx);

    let fut = async move {
        let mut stream = std::pin::pin!(stream);
        let mut sink = sink;

        loop {
            tokio::select! {
                // 원본 스트림 메시지
                msg = stream.next() => {
                    match msg {
                        Some(Ok(message)) => {
                            let is_close = matches!(message, Message::Close(_));

                            let modified = handler.handle_message(&ctx, message).await;
                            if let Some(message) = modified {
                                if let Err(e) = sink.send(message).await {
                                    match e {
                                        tungstenite::Error::ConnectionClosed => break,
                                        _ => {
                                            debug!(error = %e, "WebSocket 전송 에러 (연결 종료 중)");
                                            break;
                                        }
                                    }
                                }
                            }

                            if is_close {
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            if e.to_string().contains("Reserved bits are non-zero") {
                                warn!("Reserved bits 에러 - 건너뜀");
                                continue;
                            }
                            debug!(error = %e, "WebSocket 수신 에러");
                            let _ = sink.send(Message::Close(None)).await;
                            break;
                        }
                        None => break,
                    }
                }
                // 주입 메시지
                injected = inject_rx.recv() => {
                    match injected {
                        Some(message) => {
                            debug!("주입 메시지 전달");
                            if let Err(e) = sink.send(message).await {
                                debug!(error = %e, "주입 메시지 전송 실패 (연결 종료 중)");
                                break;
                            }
                        }
                        None => {
                            // 주입 채널 닫힘 - 정상 동작 계속
                        }
                    }
                }
            }
        }

        // 레지스트리에서 연결 해제
        if let Some(ref registry) = registry {
            registry.unregister(&conn_id).await;
        }
    };

    spawn_with_trace(fut, span);
}
