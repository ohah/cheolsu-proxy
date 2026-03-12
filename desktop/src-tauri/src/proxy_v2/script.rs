use super::state::get_command_sender;
use super::ProxyV2State;
use proxy_daemon::ClientCommand;
use tauri::State;

#[tauri::command]
pub(crate) async fn load_script(
    proxy: State<'_, ProxyV2State>,
    path: Option<String>,
    code: Option<String>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::LoadScript { path, code };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 스크립트 로드 요청 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn unload_script(proxy: State<'_, ProxyV2State>) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UnloadScript;
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 스크립트 언로드 요청 완료");
    Ok(())
}
