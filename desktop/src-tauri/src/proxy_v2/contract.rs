use super::state::get_command_sender;
use super::ProxyV2State;
use proxy_daemon::ClientCommand;
use tauri::State;

#[tauri::command]
pub(crate) async fn load_contract_spec(
    proxy: State<'_, ProxyV2State>,
    path: String,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::LoadContractSpec { path };
    sender.send_command(&cmd).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn unload_contract_spec(
    proxy: State<'_, ProxyV2State>,
    id: String,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UnloadContractSpec { id };
    sender.send_command(&cmd).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn toggle_contract_spec(
    proxy: State<'_, ProxyV2State>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::ToggleContractSpec { id, enabled };
    sender.send_command(&cmd).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_contract_specs(proxy: State<'_, ProxyV2State>) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::GetContractSpecs;
    sender.send_command(&cmd).await?;
    Ok(())
}
