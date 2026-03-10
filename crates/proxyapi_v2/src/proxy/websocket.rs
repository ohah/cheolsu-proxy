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

                parts.scheme = Some(to_websocket_scheme(parts.scheme.unwrap_or(Scheme::HTTP)));

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
                self.ctx.websocket_connector,
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
        if let Some(ref registry) = self.ctx.websocket_registry {
            let injector =
                WebSocketInjector::new(inject_to_client_tx.clone(), inject_to_server_tx.clone());
            registry.register(conn_id.clone(), injector).await;
        }

        let mut websocket_handler = self.websocket_handler;
        let websocket_registry = self.ctx.websocket_registry;
        let client_addr = self.client_addr;

        // Notify handler of new connection
        let connected_ctx = WebSocketContext::ServerToClient {
            src: uri.clone(),
            dst: client_addr,
        };
        websocket_handler.on_connected(&connected_ctx).await;

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
                dst: client_addr,
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
                src: client_addr,
                dst: uri,
            },
            websocket_registry,
            conn_id,
        );

        Ok(())
    }
}

/// HTTP/HTTPS 스킴을 WS/WSS로 변환합니다.
///
/// "ws", "wss"는 항상 유효한 Scheme이므로 실패할 수 없습니다.
fn to_websocket_scheme(scheme: Scheme) -> Scheme {
    if scheme == Scheme::HTTP {
        "ws".try_into()
            .expect("정적 문자열 'ws'는 항상 유효한 Scheme")
    } else {
        "wss"
            .try_into()
            .expect("정적 문자열 'wss'는 항상 유효한 Scheme")
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
                            let modified = handler.handle_message(&ctx, message).await;
                            if let Some(message) = modified {
                                if let Err(e) = sink.send(message).await {
                                    debug!(error = %e, "주입 메시지 전송 실패 (연결 종료 중)");
                                    break;
                                }
                            }
                        }
                        None => {
                            // 주입 채널 닫힘 - 정상 동작 계속
                        }
                    }
                }
            }
        }

        // Notify handler of disconnection
        handler.on_disconnected(&ctx).await;

        // 레지스트리에서 연결 해제
        if let Some(ref registry) = registry {
            registry.unregister(&conn_id).await;
        }
    };

    spawn_with_trace(fut, span);
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::uri::{Parts, Scheme, Uri};

    // ─── 스킴 변환 로직 ───

    #[test]
    fn http_scheme_converts_to_ws() {
        let ws = to_websocket_scheme(Scheme::HTTP);
        assert_eq!(ws.as_str(), "ws");
    }

    #[test]
    fn https_scheme_converts_to_wss() {
        let ws = to_websocket_scheme(Scheme::HTTPS);
        assert_eq!(ws.as_str(), "wss");
    }

    #[test]
    fn unknown_scheme_converts_to_wss() {
        // Scheme::HTTP 이외의 모든 스킴은 wss로 변환되어야 함
        let custom: Scheme = "ftp".try_into().unwrap();
        let ws = to_websocket_scheme(custom);
        assert_eq!(ws.as_str(), "wss");
    }

    // ─── URI 생성/변환 로직 ───

    #[test]
    fn uri_scheme_replacement_http_to_ws() {
        let uri: Uri = "http://example.com/path?query=1".parse().unwrap();
        let mut parts = uri.into_parts();
        parts.scheme = Some(to_websocket_scheme(parts.scheme.unwrap_or(Scheme::HTTP)));
        let new_uri = Uri::from_parts(parts).unwrap();

        assert_eq!(new_uri.scheme_str(), Some("ws"));
        assert_eq!(new_uri.host(), Some("example.com"));
        assert_eq!(new_uri.path(), "/path");
        assert_eq!(new_uri.query(), Some("query=1"));
    }

    #[test]
    fn uri_scheme_replacement_https_to_wss() {
        let uri: Uri = "https://example.com:8443/ws".parse().unwrap();
        let mut parts = uri.into_parts();
        parts.scheme = Some(to_websocket_scheme(parts.scheme.unwrap_or(Scheme::HTTP)));
        let new_uri = Uri::from_parts(parts).unwrap();

        assert_eq!(new_uri.scheme_str(), Some("wss"));
        assert_eq!(new_uri.host(), Some("example.com"));
        assert_eq!(new_uri.port_u16(), Some(8443));
    }

    #[test]
    fn uri_without_scheme_defaults_to_ws() {
        // 스킴이 없으면 HTTP로 기본 처리 → ws로 변환
        let mut parts = Parts::default();
        parts.authority = Some("example.com".parse().unwrap());
        parts.path_and_query = Some("/chat".parse().unwrap());

        parts.scheme = Some(to_websocket_scheme(parts.scheme.unwrap_or(Scheme::HTTP)));

        let uri = Uri::from_parts(parts).unwrap();
        assert_eq!(uri.scheme_str(), Some("ws"));
        assert_eq!(uri.host(), Some("example.com"));
        assert_eq!(uri.path(), "/chat");
    }

    #[test]
    fn uri_with_port_preserved_after_scheme_change() {
        let uri: Uri = "http://localhost:3000/ws".parse().unwrap();
        let mut parts = uri.into_parts();
        parts.scheme = Some(to_websocket_scheme(parts.scheme.unwrap_or(Scheme::HTTP)));
        let new_uri = Uri::from_parts(parts).unwrap();

        assert_eq!(new_uri.scheme_str(), Some("ws"));
        assert_eq!(new_uri.host(), Some("localhost"));
        assert_eq!(new_uri.port_u16(), Some(3000));
        assert_eq!(new_uri.path(), "/ws");
    }

    #[test]
    fn uri_with_ipv6_host() {
        let uri: Uri = "http://[::1]:8080/ws".parse().unwrap();
        let mut parts = uri.into_parts();
        parts.scheme = Some(to_websocket_scheme(parts.scheme.unwrap_or(Scheme::HTTP)));
        let new_uri = Uri::from_parts(parts).unwrap();

        assert_eq!(new_uri.scheme_str(), Some("ws"));
        assert_eq!(new_uri.host(), Some("[::1]"));
        assert_eq!(new_uri.port_u16(), Some(8080));
    }

    #[test]
    fn uri_with_userinfo_preserved() {
        let uri: Uri = "http://user:pass@example.com/ws".parse().unwrap();
        let mut parts = uri.into_parts();
        parts.scheme = Some(to_websocket_scheme(parts.scheme.unwrap_or(Scheme::HTTP)));
        let new_uri = Uri::from_parts(parts).unwrap();

        assert_eq!(new_uri.scheme_str(), Some("ws"));
        assert_eq!(
            new_uri.authority().unwrap().as_str(),
            "user:pass@example.com"
        );
    }

    // ─── 프로토콜 헤더 처리 ───

    #[test]
    fn extract_websocket_protocol_header() {
        let req = hyper::Request::builder()
            .uri("http://example.com/ws")
            .header(
                "sec-websocket-protocol",
                "graphql-ws, subscriptions-transport-ws",
            )
            .body(())
            .unwrap();

        let protocol = req
            .headers()
            .get("sec-websocket-protocol")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        assert_eq!(
            protocol.as_deref(),
            Some("graphql-ws, subscriptions-transport-ws")
        );
    }

    #[test]
    fn missing_websocket_protocol_header_returns_none() {
        let req = hyper::Request::builder()
            .uri("http://example.com/ws")
            .body(())
            .unwrap();

        let protocol = req
            .headers()
            .get("sec-websocket-protocol")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        assert!(protocol.is_none());
    }

    #[test]
    fn protocol_header_can_be_parsed_as_header_value() {
        // 프로토콜 문자열이 유효한 HeaderValue로 파싱 가능해야 함
        let protocol = "graphql-ws";
        let header_value: Result<hyper::header::HeaderValue, _> = protocol.parse();
        assert!(header_value.is_ok());
    }

    #[test]
    fn single_protocol_header() {
        let req = hyper::Request::builder()
            .uri("http://example.com/ws")
            .header("sec-websocket-protocol", "mqtt")
            .body(())
            .unwrap();

        let protocol = req
            .headers()
            .get("sec-websocket-protocol")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        assert_eq!(protocol.as_deref(), Some("mqtt"));
    }

    // ─── 에지 케이스 ───

    #[test]
    fn uri_from_parts_fails_with_path_only() {
        // authority 없이 path_and_query만 있으면 스킴을 설정할 수 없음
        let mut parts = Parts::default();
        parts.scheme = Some("ws".try_into().unwrap());
        parts.path_and_query = Some("/path".parse().unwrap());
        // authority가 없으면 scheme과 함께 사용 불가 → 에러
        let result = Uri::from_parts(parts);
        assert!(result.is_err());
    }

    #[test]
    fn websocket_config_values() {
        // WebSocket 설정 값이 코드에서 사용하는 값과 일치하는지 확인
        let mut config = WebSocketConfig::default();
        config.accept_unmasked_frames = true;
        config.max_frame_size = Some(16777216);
        config.max_message_size = Some(67108864);

        assert!(config.accept_unmasked_frames);
        assert_eq!(config.max_frame_size, Some(16 * 1024 * 1024));
        assert_eq!(config.max_message_size, Some(64 * 1024 * 1024));
    }

    #[test]
    fn default_port_logic_for_ws() {
        let uri: Uri = "ws://example.com/path".parse().unwrap();
        let port = uri
            .port_u16()
            .unwrap_or(if uri.scheme_str() == Some("wss") {
                443
            } else {
                80
            });
        assert_eq!(port, 80);
    }

    #[test]
    fn default_port_logic_for_wss() {
        let uri: Uri = "wss://example.com/path".parse().unwrap();
        let port = uri
            .port_u16()
            .unwrap_or(if uri.scheme_str() == Some("wss") {
                443
            } else {
                80
            });
        assert_eq!(port, 443);
    }

    #[test]
    fn explicit_port_overrides_default() {
        let uri: Uri = "wss://example.com:9090/path".parse().unwrap();
        let port = uri
            .port_u16()
            .unwrap_or(if uri.scheme_str() == Some("wss") {
                443
            } else {
                80
            });
        assert_eq!(port, 9090);
    }

    #[test]
    fn scheme_try_into_ws_does_not_panic() {
        let _ws: Scheme = "ws".try_into().expect("ws scheme should be valid");
    }

    #[test]
    fn scheme_try_into_wss_does_not_panic() {
        let _wss: Scheme = "wss".try_into().expect("wss scheme should be valid");
    }

    #[test]
    fn uri_with_empty_path() {
        let uri: Uri = "http://example.com".parse().unwrap();
        let mut parts = uri.into_parts();
        parts.scheme = Some(to_websocket_scheme(parts.scheme.unwrap_or(Scheme::HTTP)));
        let new_uri = Uri::from_parts(parts).unwrap();

        assert_eq!(new_uri.scheme_str(), Some("ws"));
        assert_eq!(new_uri.path(), "");
    }

    #[test]
    fn uri_with_fragment_and_query() {
        let uri: Uri = "https://example.com/ws?token=abc".parse().unwrap();
        let mut parts = uri.into_parts();
        parts.scheme = Some(to_websocket_scheme(parts.scheme.unwrap_or(Scheme::HTTP)));
        let new_uri = Uri::from_parts(parts).unwrap();

        assert_eq!(new_uri.scheme_str(), Some("wss"));
        assert_eq!(new_uri.query(), Some("token=abc"));
    }
}
