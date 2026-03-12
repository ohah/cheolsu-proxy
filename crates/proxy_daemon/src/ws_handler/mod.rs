mod handler;
mod trait_impl;

use proxy_v2_models::{WsConnectionEvent, WsMessageInfo};

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

#[cfg(test)]
mod tests;
