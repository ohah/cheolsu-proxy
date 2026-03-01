use super::helpers::{bad_request, spawn_with_trace};
use super::internal::InternalProxy;
use crate::{
    Body, HttpHandler, WebSocketContext, WebSocketHandler,
    certificate_authority::CertificateAuthority,
};
use futures::{Sink, Stream, StreamExt};
use http::uri::{Scheme, Uri};
use hyper::{Request, Response, upgrade::Upgraded};
use hyper_util::{client::legacy::connect::Connect, rt::TokioIo};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{self, Message, protocol::WebSocketConfig},
};
use tracing::{debug, error, info, info_span, instrument};

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

        let InternalProxy {
            websocket_handler, ..
        } = self;

        // WebSocket 핸들러를 사용하여 메시지 전달
        debug!("서버→클라이언트 메시지 전달기 시작");
        spawn_message_forwarder(
            server_stream,
            client_sink,
            websocket_handler.clone(),
            WebSocketContext::ServerToClient {
                src: uri.clone(),
                dst: self.client_addr,
            },
        );

        debug!("클라이언트→서버 메시지 전달기 시작");
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
