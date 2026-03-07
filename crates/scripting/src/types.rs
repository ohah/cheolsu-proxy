use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JS/TS 스크립트에서 사용하는 HTTP 요청 표현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

/// JS/TS 스크립트에서 사용하는 HTTP 응답 표현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

/// onRequest 훅의 반환값
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum RequestAction {
    /// 요청을 그대로 전달
    #[serde(rename = "forward")]
    Forward,
    /// 수정된 요청으로 전달
    #[serde(rename = "modify")]
    ModifyRequest { request: ScriptRequest },
    /// 요청을 차단하고 응답 반환
    #[serde(rename = "respond")]
    Respond { response: ScriptResponse },
}

/// onResponse 훅의 반환값
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ResponseAction {
    /// 응답을 그대로 전달
    #[serde(rename = "forward")]
    Forward,
    /// 수정된 응답으로 전달
    #[serde(rename = "modify")]
    ModifyResponse { response: ScriptResponse },
}

/// WebSocket 메시지 방향
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WsDirection {
    ToServer,
    ToClient,
}

/// JS/TS 스크립트에서 사용하는 WebSocket 메시지 표현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptWsMessage {
    pub connection_id: String,
    pub url: String,
    pub direction: WsDirection,
    pub payload: String,
    pub is_binary: bool,
}

/// onWebSocketMessage 훅의 반환값
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum WsAction {
    /// 메시지를 그대로 전달
    #[serde(rename = "forward")]
    Forward,
    /// 수정된 메시지 전달
    #[serde(rename = "modify")]
    Modify { payload: String, is_binary: bool },
    /// 메시지 드롭
    #[serde(rename = "drop")]
    Drop,
}

/// 스크립트 로드 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    /// 스크립트 파일 경로 (TypeScript 또는 JavaScript)
    pub path: String,
    /// 스크립트 활성화 여부
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}
