use serde::{Deserialize, Serialize};

/// WebSocket 메시지 방향
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsDirection {
    ClientToServer,
    ServerToClient,
}

/// WebSocket 메시지 타입
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsMessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

/// UI로 전달되는 WebSocket 메시지 정보
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WsMessageInfo {
    /// WebSocket 연결 ID (URI)
    pub connection_id: String,
    /// 메시지 순번
    pub sequence: u64,
    /// 메시지 방향
    pub direction: WsDirection,
    /// 메시지 타입
    pub message_type: WsMessageType,
    /// 메시지 페이로드 (텍스트 또는 base64 인코딩된 바이너리)
    pub payload: String,
    /// 페이로드 크기 (바이트)
    pub size: usize,
    /// 타임스탬프 (나노초)
    pub time: i64,
    /// 바이너리 여부
    pub is_binary: bool,
}

/// WebSocket 연결 상태 이벤트
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "status")]
pub enum WsConnectionEvent {
    #[serde(rename = "connected")]
    Connected {
        connection_id: String,
        uri: String,
        time: i64,
    },
    #[serde(rename = "disconnected")]
    Disconnected { connection_id: String, time: i64 },
}
