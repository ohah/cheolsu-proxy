use proxy_v2_models::RequestInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum DaemonMessage {
    #[serde(rename = "event")]
    Event { data: RequestInfo },
    #[serde(rename = "status")]
    Status { running: bool, port: u16 },
    #[serde(rename = "intercept_rules_updated")]
    InterceptRulesUpdated { rules: Vec<InterceptRule> },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd")]
pub enum ClientCommand {
    #[serde(rename = "subscribe")]
    Subscribe,
    #[serde(rename = "update_sessions")]
    UpdateSessions { data: serde_json::Value },
    #[serde(rename = "update_intercept_rules")]
    UpdateInterceptRules { rules: Vec<InterceptRule> },
    #[serde(rename = "stop")]
    Stop,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProxyLockInfo {
    pub pid: u32,
    pub port: u16,
    pub uds_path: String,
}

/// 인터셉트 규칙 정의
/// `filter`는 mitmproxy 스타일 flow filter 표현식 (예: `~u api & ~m POST & ~bq "action=delete"`)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterceptRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub filter: String,
    pub action: InterceptAction,
}

/// 인터셉트 동작
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum InterceptAction {
    /// 요청을 차단하고 지정된 응답을 반환
    #[serde(rename = "block")]
    Block {
        #[serde(default = "default_block_status")]
        status_code: u16,
        #[serde(default)]
        body: String,
    },
    /// 요청 헤더/바디를 수정하여 전달
    #[serde(rename = "modify_request")]
    ModifyRequest {
        #[serde(default)]
        add_headers: HashMap<String, String>,
        #[serde(default)]
        remove_headers: Vec<String>,
        #[serde(default)]
        set_body: Option<String>,
    },
    /// 응답 헤더/바디/상태코드를 수정
    #[serde(rename = "modify_response")]
    ModifyResponse {
        #[serde(default)]
        set_status: Option<u16>,
        #[serde(default)]
        add_headers: HashMap<String, String>,
        #[serde(default)]
        remove_headers: Vec<String>,
        #[serde(default)]
        set_body: Option<String>,
    },
}

fn default_block_status() -> u16 {
    403
}

impl std::fmt::Display for InterceptRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.enabled { "활성" } else { "비활성" };
        let action_desc = match &self.action {
            InterceptAction::Block { status_code, .. } => {
                format!("Block({})", status_code)
            }
            InterceptAction::ModifyRequest { .. } => "ModifyRequest".to_string(),
            InterceptAction::ModifyResponse { set_status, .. } => {
                if let Some(status) = set_status {
                    format!("ModifyResponse(status={})", status)
                } else {
                    "ModifyResponse".to_string()
                }
            }
        };
        write!(
            f,
            "[{}] {} (filter: {}) -> {} [{}]",
            self.id, self.name, self.filter, action_desc, status
        )
    }
}
