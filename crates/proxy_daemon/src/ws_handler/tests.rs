use super::*;
use crate::handler::ProxyConfig;
use crate::handler::{InterceptEngine, LoggingHandler, QuickSettings, RequestState};
use crate::sse_handler::SseState;
use bytes::Bytes;
use proxy_v2_models::{WsDirection, WsMessageType};
use proxyapi_v2::tokio_tungstenite::tungstenite::Message;
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
        code:
            proxyapi_v2::tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
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
            request_start: None,
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
            ssl_proxying: Arc::new(RwLock::new(crate::handler::SslProxyingConfig {
                mode: crate::protocol::SslProxyingMode::default(),
                entries: Vec::new(),
                default_passthrough: crate::ssl_proxying::default_passthrough_entries(),
            })),
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
            request_start: None,
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
            ssl_proxying: Arc::new(RwLock::new(crate::handler::SslProxyingConfig {
                mode: crate::protocol::SslProxyingMode::default(),
                entries: Vec::new(),
                default_passthrough: crate::ssl_proxying::default_passthrough_entries(),
            })),
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
