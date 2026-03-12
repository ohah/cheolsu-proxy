use proxy_daemon::{CommandSender, DaemonConnection};
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) type ProxyV2State = Arc<Mutex<Option<DaemonConnection>>>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ProxyStartResult {
    pub status: bool,
    pub message: String,
}

pub(crate) async fn get_command_sender(proxy: &ProxyV2State) -> Result<CommandSender, String> {
    let guard = proxy.lock().await;
    guard
        .as_ref()
        .ok_or_else(|| "프록시가 실행 중이 아닙니다".to_string())
        .map(|conn| conn.command_sender())
}

pub(crate) fn is_hop_by_hop_header(name: &str) -> bool {
    const HOP_BY_HOP: &[&str] = &[
        "host",
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "transfer-encoding",
        "upgrade",
    ];
    HOP_BY_HOP.iter().any(|h| h.eq_ignore_ascii_case(name))
}

pub(crate) fn base64_engine() -> base64::engine::GeneralPurpose {
    use base64::engine::general_purpose::STANDARD;
    STANDARD
}

pub(crate) fn base64_encode(engine: &base64::engine::GeneralPurpose, data: &[u8]) -> String {
    use base64::Engine;
    engine.encode(data)
}
