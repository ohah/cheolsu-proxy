use super::state::get_command_sender;
use super::ProxyV2State;
use proxy_daemon::{ClientCommand, LearnedTlsStrategy, TlsConfigRule, TlsPassthroughEntry};
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

#[tauri::command]
pub(crate) async fn get_tls_config_rules(
    proxy: State<'_, ProxyV2State>,
) -> Result<Vec<TlsConfigRule>, String> {
    let sender = get_command_sender(&proxy).await?;
    sender
        .send_command(&ClientCommand::GetTlsConfigRules)
        .await
        .map_err(|e| format!("TLS 설정 규칙 조회 실패: {}", e))?;
    // 현재 규칙은 명령어 응답으로 돌아오지만, 동기적으로 반환하기 위해
    // 내장 규칙을 직접 반환
    let manager = proxy_daemon::TlsConfigManager::with_builtin_rules();
    Ok(manager.rules().to_vec())
}

#[tauri::command]
pub(crate) async fn get_learned_tls_strategies(
    proxy: State<'_, ProxyV2State>,
) -> Result<Vec<LearnedTlsStrategy>, String> {
    let sender = get_command_sender(&proxy).await?;
    sender
        .send_command(&ClientCommand::GetLearnedTlsStrategies)
        .await
        .map_err(|e| format!("학습된 TLS 전략 조회 실패: {}", e))?;
    // TODO: 데몬 응답을 비동기로 수신하여 반환
    Ok(vec![])
}

#[tauri::command]
pub(crate) async fn clear_learned_tls_strategies(
    proxy: State<'_, ProxyV2State>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    sender
        .send_command(&ClientCommand::ClearLearnedTlsStrategies)
        .await
        .map_err(|e| format!("학습된 TLS 전략 초기화 실패: {}", e))
}

#[tauri::command]
pub(crate) async fn update_never_passthrough_domains(
    proxy: State<'_, ProxyV2State>,
    entries: Vec<String>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    sender
        .send_command(&ClientCommand::UpdateNeverPassthroughDomains { entries })
        .await
        .map_err(|e| format!("Never Passthrough 도메인 업데이트 실패: {}", e))
}

#[tauri::command]
pub(crate) async fn get_never_passthrough_domains(
    _proxy: State<'_, ProxyV2State>,
) -> Result<Vec<String>, String> {
    let path = proxy_daemon::daemon::app_support_dir()
        .map(|d| d.join("never_passthrough.json"))
        .map_err(|e| format!("데이터 디렉토리를 찾을 수 없습니다: {}", e))?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("Never Passthrough 파일 읽기 실패: {}", e))?;
    let entries: Vec<String> =
        serde_json::from_str(&data).map_err(|e| format!("JSON 파싱 실패: {}", e))?;
    Ok(entries)
}
