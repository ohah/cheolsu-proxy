use super::state::get_command_sender;
use super::ProxyV2State;
use proxy_daemon::{ClientCommand, TlsPassthroughEntry};
use tauri::State;

#[tauri::command]
pub(crate) async fn get_tls_passthrough_list(
    proxy: State<'_, ProxyV2State>,
) -> Result<Vec<TlsPassthroughEntry>, String> {
    let sender = get_command_sender(&proxy).await?;
    sender
        .send_command(&ClientCommand::GetTlsPassthroughList)
        .await
        .map_err(|e| format!("TLS passthrough 목록 요청 실패: {}", e))?;
    let path = proxy_daemon::daemon::app_support_dir()
        .map(|d| d.join("tls_passthrough.json"))
        .map_err(|e| format!("데이터 디렉토리를 찾을 수 없습니다: {}", e))?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("TLS passthrough 파일 읽기 실패: {}", e))?;
    let map: std::collections::HashMap<String, u32> =
        serde_json::from_str(&data).map_err(|e| format!("JSON 파싱 실패: {}", e))?;
    let mut entries: Vec<TlsPassthroughEntry> = map
        .into_iter()
        .map(|(host, failure_count)| TlsPassthroughEntry {
            host,
            failure_count,
        })
        .collect();
    entries.sort_by(|a, b| a.host.cmp(&b.host));
    Ok(entries)
}

#[tauri::command]
pub(crate) async fn remove_tls_passthrough(
    proxy: State<'_, ProxyV2State>,
    host: String,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    sender
        .send_command(&ClientCommand::RemoveTlsPassthrough { host })
        .await
        .map_err(|e| format!("TLS passthrough 삭제 실패: {}", e))
}

#[tauri::command]
pub(crate) async fn clear_tls_passthrough(proxy: State<'_, ProxyV2State>) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    sender
        .send_command(&ClientCommand::ClearTlsPassthrough)
        .await
        .map_err(|e| format!("TLS passthrough 초기화 실패: {}", e))
}
