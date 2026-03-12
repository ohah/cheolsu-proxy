use super::state::get_command_sender;
use super::ProxyV2State;
use proxy_daemon::{ClientCommand, InterceptRule};
use tauri::State;

#[tauri::command]
pub(crate) async fn update_intercept_rules_v2(
    proxy: tauri::State<'_, ProxyV2State>,
    rules: Vec<InterceptRule>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateInterceptRules { rules };
    sender.send_command(&cmd).await?;
    println!("Daemon에 인터셉트 규칙 업데이트 완료");
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct WsInjectParams {
    pub connection_id: String,
    pub direction: String,
    pub payload: String,
    pub is_binary: bool,
}

#[tauri::command]
pub(crate) async fn ws_inject_message(
    proxy: State<'_, ProxyV2State>,
    params: WsInjectParams,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::WsInject {
        connection_id: params.connection_id,
        direction: params.direction,
        payload: params.payload,
        is_binary: params.is_binary,
    };
    sender.send_command(&cmd).await?;
    Ok(())
}
