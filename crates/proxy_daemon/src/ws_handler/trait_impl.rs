use proxy_v2_models::WsConnectionEvent;
use proxyapi_v2::{tokio_tungstenite::tungstenite::Message, WebSocketContext, WebSocketHandler};
use tracing::debug;

use super::WsEvent;
use crate::handler::LoggingHandler;

impl WebSocketHandler for LoggingHandler {
    async fn on_connected(&mut self, ctx: &WebSocketContext) {
        if let Some(ws_sender) = &self.ws.ws_sender {
            let (connection_id, uri) = match ctx {
                WebSocketContext::ClientToServer { dst, .. } => (dst.to_string(), dst.to_string()),
                WebSocketContext::ServerToClient { src, .. } => (src.to_string(), src.to_string()),
            };
            let event = WsConnectionEvent::Connected {
                connection_id,
                uri,
                time: chrono::Local::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_default(),
            };
            if let Err(e) = ws_sender.try_send(WsEvent::Connection(event)) {
                debug!("[WS] 연결 이벤트 전송 실패 (채널 가득 참): {}", e);
            }
        }
    }

    async fn on_disconnected(&mut self, ctx: &WebSocketContext) {
        if let Some(ws_sender) = &self.ws.ws_sender {
            let connection_id = match ctx {
                WebSocketContext::ClientToServer { dst, .. } => dst.to_string(),
                WebSocketContext::ServerToClient { src, .. } => src.to_string(),
            };
            let event = WsConnectionEvent::Disconnected {
                connection_id,
                time: chrono::Local::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_default(),
            };
            if let Err(e) = ws_sender.try_send(WsEvent::Connection(event)) {
                debug!("[WS] 연결 이벤트 전송 실패 (채널 가득 참): {}", e);
            }
        }
    }

    async fn handle_message(&mut self, ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        let (direction, connection_id, url) = Self::extract_ws_context(ctx);

        let (message_type, payload, size, is_binary) = match Self::convert_ws_message_payload(&msg)
        {
            Some(tuple) => tuple,
            None => return Some(msg),
        };

        let (msg, payload, is_binary) = self
            .apply_ws_script_hook(
                ctx,
                msg,
                &connection_id,
                &url,
                message_type,
                payload,
                is_binary,
            )
            .await?;

        self.emit_ws_event(
            connection_id,
            direction,
            message_type,
            payload,
            size,
            is_binary,
        );
        Some(msg)
    }
}
