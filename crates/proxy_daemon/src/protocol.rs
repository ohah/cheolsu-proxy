use proxy_v2_models::RequestInfo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum DaemonMessage {
    #[serde(rename = "event")]
    Event { data: RequestInfo },
    #[serde(rename = "status")]
    Status { running: bool, port: u16 },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd")]
pub enum ClientCommand {
    #[serde(rename = "subscribe")]
    Subscribe,
    #[serde(rename = "update_sessions")]
    UpdateSessions { data: serde_json::Value },
    #[serde(rename = "stop")]
    Stop,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProxyLockInfo {
    pub pid: u32,
    pub port: u16,
    pub uds_path: String,
}
