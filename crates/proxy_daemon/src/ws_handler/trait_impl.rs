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
        let connection_id = match ctx {
            WebSocketContext::ClientToServer { dst, .. } => dst.to_string(),
            WebSocketContext::ServerToClient { src, .. } => src.to_string(),
        };

        // 연결 종료 시 mqtt_versions 항목을 정리해 메모리 누수를 방지한다.
        self.ws.mqtt_versions.lock().remove(&connection_id);

        if let Some(ws_sender) = &self.ws.ws_sender {
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

        let (message_type, payload, _size, is_binary) = match Self::convert_ws_message_payload(&msg)
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

        // 스크립트가 메시지를 수정했을 수 있으므로 size를 (수정 후) 실제 메시지에서 다시 계산한다.
        let size = msg.len();

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
