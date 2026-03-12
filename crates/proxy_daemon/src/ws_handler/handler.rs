use proxy_v2_models::{WsDirection, WsMessageType};
use proxyapi_v2::{tokio_tungstenite::tungstenite::Message, WebSocketContext};
use tracing::{debug, error};

use super::WsEvent;
use crate::handler::LoggingHandler;

impl LoggingHandler {
    /// WebSocketContext에서 방향, connection_id, URL을 추출합니다.
    pub(crate) fn extract_ws_context(ctx: &WebSocketContext) -> (WsDirection, String, String) {
        match ctx {
            WebSocketContext::ClientToServer { dst, .. } => {
                let url = dst.to_string();
                (WsDirection::ClientToServer, url.clone(), url)
            }
            WebSocketContext::ServerToClient { src, .. } => {
                let url = src.to_string();
                (WsDirection::ServerToClient, url.clone(), url)
            }
        }
    }

    /// WebSocket 메시지를 (message_type, payload, size, is_binary) 튜플로 변환합니다.
    /// Message::Frame은 None을 반환합니다.
    pub(crate) fn convert_ws_message_payload(
        msg: &Message,
    ) -> Option<(WsMessageType, String, usize, bool)> {
        match msg {
            Message::Text(text) => Some((WsMessageType::Text, text.to_string(), text.len(), false)),
            Message::Binary(data) => {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(data);
                Some((WsMessageType::Binary, encoded, data.len(), true))
            }
            Message::Ping(data) => Some((
                WsMessageType::Ping,
                format!("{} bytes", data.len()),
                data.len(),
                true,
            )),
            Message::Pong(data) => Some((
                WsMessageType::Pong,
                format!("{} bytes", data.len()),
                data.len(),
                true,
            )),
            Message::Close(frame) => {
                let payload = frame
                    .as_ref()
                    .map(|f| format!("{}: {}", f.code, f.reason))
                    .unwrap_or_default();
                let size = payload.len();
                Some((WsMessageType::Close, payload, size, false))
            }
            Message::Frame(_) => None,
        }
    }

    /// Text/Binary 메시지에 대해 스크립트 onWebSocketMessage 훅을 적용합니다.
    /// Drop이면 None 반환, Forward/Modify면 (변경된 msg, payload, is_binary) 반환.
    pub(crate) async fn apply_ws_script_hook(
        &self,
        ctx: &WebSocketContext,
        msg: Message,
        connection_id: &str,
        url: &str,
        message_type: WsMessageType,
        payload: String,
        is_binary: bool,
    ) -> Option<(Message, String, bool)> {
        if !matches!(message_type, WsMessageType::Text | WsMessageType::Binary) {
            return Some((msg, payload, is_binary));
        }

        let script_direction = match ctx {
            WebSocketContext::ClientToServer { .. } => scripting::WsDirection::ToServer,
            WebSocketContext::ServerToClient { .. } => scripting::WsDirection::ToClient,
        };
        let script_msg = scripting::ScriptWsMessage {
            connection_id: connection_id.to_string(),
            url: url.to_string(),
            direction: script_direction,
            payload: payload.clone(),
            is_binary,
        };
        match self
            .intercept
            .script_handle
            .invoke_on_ws_message(&script_msg)
            .await
        {
            Ok(scripting::WsAction::Forward) => Some((msg, payload, is_binary)),
            Ok(scripting::WsAction::Modify {
                payload: new_payload,
                is_binary: new_is_binary,
            }) => {
                let new_msg = if new_is_binary {
                    use base64::Engine;
                    match base64::engine::general_purpose::STANDARD.decode(&new_payload) {
                        Ok(data) => Message::Binary(data.into()),
                        Err(_) => Message::Text(new_payload.clone().into()),
                    }
                } else {
                    Message::Text(new_payload.clone().into())
                };
                Some((new_msg, new_payload, new_is_binary))
            }
            Ok(scripting::WsAction::Drop) => None,
            Err(e) => {
                error!("[Script] onWebSocketMessage 오류: {}", e);
                Some((msg, payload, is_binary))
            }
        }
    }

    /// WebSocket 이벤트를 생성하여 ws_sender로 전송합니다.
    pub(crate) fn emit_ws_event(
        &self,
        connection_id: String,
        direction: WsDirection,
        message_type: WsMessageType,
        payload: String,
        size: usize,
        is_binary: bool,
    ) {
        let Some(ws_sender) = &self.ws.ws_sender else {
            return;
        };

        let sequence = self
            .ws
            .ws_sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let content_type = proxy_v2_models::detect_ws_content_type(&payload, is_binary);

        let mqtt_version = if content_type == proxy_v2_models::WsContentType::Mqtt {
            if let Some(ver) = proxy_v2_models::extract_mqtt_version_from_connect(&payload) {
                // SAFETY: parking_lot lock - .await 없이 즉시 해제되므로 안전함.
                self.ws
                    .mqtt_versions
                    .lock()
                    .insert(connection_id.clone(), ver);
                Some(ver)
            } else {
                // SAFETY: parking_lot lock - .await 없이 즉시 해제되므로 안전함.
                self.ws.mqtt_versions.lock().get(&connection_id).copied()
            }
        } else {
            None
        };

        let info = proxy_v2_models::WsMessageInfo {
            connection_id,
            sequence,
            direction,
            message_type,
            payload,
            size,
            time: chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
            is_binary,
            content_type,
            mqtt_version,
        };

        if let Err(e) = ws_sender.try_send(WsEvent::Message(info)) {
            debug!("[WS] 메시지 이벤트 전송 실패 (채널 가득 참): {}", e);
        }
    }
}
