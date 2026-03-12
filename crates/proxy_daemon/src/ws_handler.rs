use proxy_v2_models::{WsConnectionEvent, WsDirection, WsMessageInfo, WsMessageType};
use proxyapi_v2::{tokio_tungstenite::tungstenite::Message, WebSocketContext, WebSocketHandler};
use tracing::{debug, error};

use crate::handler::LoggingHandler;

/// WebSocket 이벤트 (메시지 또는 연결 상태)
#[derive(Clone, Debug)]
pub enum WsEvent {
    Message(WsMessageInfo),
    Connection(WsConnectionEvent),
}

/// WebSocket 상태 관리
#[derive(Clone)]
pub(crate) struct WebSocketState {
    pub(crate) ws_sender: Option<tokio::sync::mpsc::Sender<WsEvent>>,
    pub(crate) ws_sequence: std::sync::Arc<std::sync::atomic::AtomicU64>,
    // SAFETY: parking_lot::Mutex - async 컨텍스트에서 사용 중이나,
    // .await를 넘어서 lock을 유지하지 않으므로 안전함.
    // 리팩토링 시 tokio::sync::Mutex으로 교체 검토 필요.
    pub(crate) mqtt_versions:
        std::sync::Arc<parking_lot::Mutex<std::collections::HashMap<String, u8>>>,
}

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

        let info = WsMessageInfo {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::ProxyConfig;
    use crate::handler::{InterceptEngine, LoggingHandler, QuickSettings, RequestState};
    use crate::sse_handler::SseState;
    use bytes::Bytes;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// 테스트용 LoggingHandler를 생성합니다.
    fn make_test_handler() -> LoggingHandler {
        let (sender, _rx) = tokio::sync::mpsc::channel(1);
        LoggingHandler::new(sender, std::path::PathBuf::from("/tmp"))
    }

    // --- convert_ws_message_payload 테스트 ---

    #[test]
    fn convert_text_message() {
        let msg = Message::Text("hello".into());
        let (msg_type, payload, size, is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Text);
        assert_eq!(payload, "hello");
        assert_eq!(size, 5);
        assert!(!is_binary);
    }

    #[test]
    fn convert_binary_message() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let msg = Message::Binary(data.clone().into());
        let (msg_type, payload, size, is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Binary);
        use base64::Engine;
        assert_eq!(
            payload,
            base64::engine::general_purpose::STANDARD.encode(&data)
        );
        assert_eq!(size, 4);
        assert!(is_binary);
    }

    #[test]
    fn convert_ping_message() {
        let msg = Message::Ping(vec![1, 2, 3].into());
        let (msg_type, payload, size, is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Ping);
        assert_eq!(payload, "3 bytes");
        assert_eq!(size, 3);
        assert!(is_binary);
    }

    #[test]
    fn convert_pong_message() {
        let msg = Message::Pong(vec![].into());
        let (msg_type, payload, size, is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Pong);
        assert_eq!(payload, "0 bytes");
        assert_eq!(size, 0);
        assert!(is_binary);
    }

    #[test]
    fn convert_close_message_with_frame() {
        use proxyapi_v2::tokio_tungstenite::tungstenite::protocol::CloseFrame;
        let frame = CloseFrame {
            code: proxyapi_v2::tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
            reason: "bye".into(),
        };
        let msg = Message::Close(Some(frame));
        let (msg_type, payload, _size, is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Close);
        assert!(payload.contains("bye"));
        assert!(!is_binary);
    }

    #[test]
    fn convert_close_message_without_frame() {
        let msg = Message::Close(None);
        let (msg_type, payload, _size, _is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Close);
        assert!(payload.is_empty());
    }

    #[test]
    fn convert_frame_message_returns_none() {
        use proxyapi_v2::tokio_tungstenite::tungstenite::protocol::frame::Frame;
        let msg = Message::Frame(Frame::ping(Bytes::new()));
        assert!(LoggingHandler::convert_ws_message_payload(&msg).is_none());
    }

    // --- emit_ws_event 테스트 ---

    #[test]
    fn emit_ws_event_without_sender_does_nothing() {
        let handler = make_test_handler();
        // ws_sender가 None이면 패닉 없이 조용히 반환
        handler.emit_ws_event(
            "conn1".to_string(),
            WsDirection::ClientToServer,
            WsMessageType::Text,
            "hello".to_string(),
            5,
            false,
        );
    }

    #[test]
    fn emit_ws_event_sends_to_channel() {
        let (sender, _rx) = tokio::sync::mpsc::channel(1);
        let (ws_sender, mut ws_rx) = tokio::sync::mpsc::channel(8);
        let handler = LoggingHandler {
            sender,
            request: RequestState {
                req: None,
                res: None,
            },
            config: ProxyConfig {
                cache_dir: None,
                ca_cert_der: None,
                quick_settings: Arc::new(tokio::sync::RwLock::new(QuickSettings::default())),
                proxy_auth: Arc::new(tokio::sync::RwLock::new(None)),
                max_body_size: None,
            },
            intercept: InterceptEngine {
                intercept_rules: Arc::new(RwLock::new(Vec::new())),
                server_replay_entries: Arc::new(RwLock::new(Vec::new())),
                host_mappings: Arc::new(RwLock::new(Vec::new())),
                script_handle: scripting::ScriptHandle::new(),
                ssl_proxying_mode: Arc::new(RwLock::new(
                    crate::protocol::SslProxyingMode::default(),
                )),
                ssl_proxying_entries: Arc::new(RwLock::new(Vec::new())),
            },
            ws: WebSocketState {
                ws_sender: Some(ws_sender),
                ws_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                mqtt_versions: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
            },
            sse: SseState {
                sse_sender: None,
                sse_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            },
            breakpoint_manager: None,
        };

        handler.emit_ws_event(
            "wss://example.com".to_string(),
            WsDirection::ClientToServer,
            WsMessageType::Text,
            "test payload".to_string(),
            12,
            false,
        );

        let event = ws_rx.try_recv().unwrap();
        match event {
            WsEvent::Message(info) => {
                assert_eq!(info.connection_id, "wss://example.com");
                assert_eq!(info.sequence, 0);
                assert_eq!(info.direction, WsDirection::ClientToServer);
                assert_eq!(info.message_type, WsMessageType::Text);
                assert_eq!(info.payload, "test payload");
                assert_eq!(info.size, 12);
                assert!(!info.is_binary);
            }
            _ => panic!("Expected WsEvent::Message"),
        }
    }

    #[test]
    fn emit_ws_event_increments_sequence() {
        let (sender, _rx) = tokio::sync::mpsc::channel(1);
        let (ws_sender, mut ws_rx) = tokio::sync::mpsc::channel(8);
        let handler = LoggingHandler {
            sender,
            request: RequestState {
                req: None,
                res: None,
            },
            config: ProxyConfig {
                cache_dir: None,
                ca_cert_der: None,
                quick_settings: Arc::new(tokio::sync::RwLock::new(QuickSettings::default())),
                proxy_auth: Arc::new(tokio::sync::RwLock::new(None)),
                max_body_size: None,
            },
            intercept: InterceptEngine {
                intercept_rules: Arc::new(RwLock::new(Vec::new())),
                server_replay_entries: Arc::new(RwLock::new(Vec::new())),
                host_mappings: Arc::new(RwLock::new(Vec::new())),
                script_handle: scripting::ScriptHandle::new(),
                ssl_proxying_mode: Arc::new(RwLock::new(
                    crate::protocol::SslProxyingMode::default(),
                )),
                ssl_proxying_entries: Arc::new(RwLock::new(Vec::new())),
            },
            ws: WebSocketState {
                ws_sender: Some(ws_sender),
                ws_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                mqtt_versions: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
            },
            sse: SseState {
                sse_sender: None,
                sse_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            },
            breakpoint_manager: None,
        };

        for i in 0..3 {
            handler.emit_ws_event(
                "conn".to_string(),
                WsDirection::ClientToServer,
                WsMessageType::Text,
                "msg".to_string(),
                3,
                false,
            );
            match ws_rx.try_recv().unwrap() {
                WsEvent::Message(info) => assert_eq!(info.sequence, i),
                _ => panic!("Expected WsEvent::Message"),
            }
        }
    }
}
