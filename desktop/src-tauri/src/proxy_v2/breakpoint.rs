use super::state::get_command_sender;
use super::ProxyV2State;
use proxy_daemon::{BreakpointAction, ClientCommand};

#[tauri::command]
pub(crate) async fn update_breakpoint_rules(
    proxy: tauri::State<'_, ProxyV2State>,
    rules: Vec<proxy_daemon::BreakpointRule>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateBreakpointRules { rules };
    sender.send_command(&cmd).await?;
    println!("Daemon에 breakpoint 규칙 업데이트 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn resolve_breakpoint(
    proxy: tauri::State<'_, ProxyV2State>,
    id: String,
    action: BreakpointAction,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::ResolveBreakpoint { id, action };
    sender.send_command(&cmd).await?;
    println!("Daemon에 breakpoint 해제 완료");
    Ok(())
}
