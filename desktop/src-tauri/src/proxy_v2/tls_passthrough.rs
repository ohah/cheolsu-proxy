use super::state::get_command_sender;
use super::ProxyV2State;
use proxy_daemon::{ClientCommand, LearnedTlsStrategy, TlsConfigRule, TlsPassthroughEntry};
use serde::Deserialize;
use tauri::State;

const MIN_FAILURES_FOR_BYPASS: u32 = 2;
const FAILURE_TTL_SECS: u64 = 60 * 60;

#[derive(Deserialize)]
struct StoredTlsFailureRecord {
    failure_count: u32,
    last_failure_unix_secs: u64,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StoredTlsFailure {
    Count(u32),
    Record(StoredTlsFailureRecord),
}

fn wildcard_matches(pattern: &str, text: &str) -> bool {
    fn inner(pattern: &[u8], text: &[u8]) -> bool {
        match (pattern.first(), text.first()) {
            (None, None) => true,
            (Some(b'*'), _) => {
                inner(&pattern[1..], text) || (!text.is_empty() && inner(pattern, &text[1..]))
            }
            (Some(b'?'), Some(_)) => inner(&pattern[1..], &text[1..]),
            (Some(a), Some(b)) if a == b => inner(&pattern[1..], &text[1..]),
            _ => false,
        }
    }
    inner(
        pattern.to_ascii_lowercase().as_bytes(),
        text.to_ascii_lowercase().as_bytes(),
    )
}

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
    let map: std::collections::HashMap<String, StoredTlsFailure> =
        serde_json::from_str(&data).map_err(|e| format!("JSON 파싱 실패: {}", e))?;
    let never_passthrough_path = proxy_daemon::daemon::app_support_dir()
        .map(|d| d.join("never_passthrough.json"))
        .map_err(|e| format!("데이터 디렉토리를 찾을 수 없습니다: {}", e))?;
    let never_passthrough: Vec<String> = if never_passthrough_path.exists() {
        std::fs::read_to_string(&never_passthrough_path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut entries: Vec<TlsPassthroughEntry> = map
        .into_iter()
        .map(|(host, stored)| {
            let (failure_count, last_failure_unix_secs) = match stored {
                StoredTlsFailure::Count(count) => (count, now),
                StoredTlsFailure::Record(record) => {
                    (record.failure_count, record.last_failure_unix_secs)
                }
            };
            let expires_at = last_failure_unix_secs + FAILURE_TTL_SECS;
            let expired = now >= expires_at;
            let blocked_by_never_passthrough = never_passthrough
                .iter()
                .any(|pattern| wildcard_matches(pattern, &host));
            TlsPassthroughEntry {
                host,
                failure_count,
                active: failure_count >= MIN_FAILURES_FOR_BYPASS
                    && !expired
                    && !blocked_by_never_passthrough,
                blocked_by_never_passthrough,
                last_failure_unix_secs,
                expires_at_unix_secs: if expired { None } else { Some(expires_at) },
            }
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
pub(crate) async fn get_learned_tls_strategies() -> Result<Vec<LearnedTlsStrategy>, String> {
    // 데몬에 일회성 연결로 학습된 TLS 전략을 동기 조회한다(request-response).
    // 데몬 미실행 시 연결 에러를 반환하며, 프론트는 이를 빈 배열로 graceful 처리한다.
    proxy_daemon::query_learned_tls_strategies()
        .await
        .map_err(|e| format!("학습된 TLS 전략 조회 실패: {}", e))
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
