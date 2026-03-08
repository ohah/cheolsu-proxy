#[tauri::command]
pub async fn get_proxy_status_command() -> Result<proxy_daemon::ProxyStatus, String> {
    proxy_daemon::get_proxy_status().map_err(|e| e.to_string())
}
