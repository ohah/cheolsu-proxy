use super::state::get_command_sender;
use super::ProxyV2State;
use proxy_daemon::{
    ClientCommand, HostMapping, ProxyAuthConfig, ServerReplayEntry, SslProxyingEntry,
    SslProxyingMode, ThrottleConfig, UpstreamProxyConfig,
};
use tauri::State;

#[tauri::command]
pub(crate) async fn update_upstream_proxy(
    proxy: State<'_, ProxyV2State>,
    config: Option<UpstreamProxyConfig>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateUpstreamProxy { config };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 upstream proxy 설정 업데이트 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_proxy_auth(
    proxy: State<'_, ProxyV2State>,
    config: ProxyAuthConfig,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateProxyAuth { config };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 프록시 인증 설정 업데이트 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_throttle(
    proxy: State<'_, ProxyV2State>,
    config: Option<ThrottleConfig>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateThrottle { config };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 스로틀링 설정 업데이트 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_connection_strategy(
    proxy: State<'_, ProxyV2State>,
    strategy: String,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateConnectionStrategy {
        strategy: strategy.clone(),
    };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 연결 전략 업데이트 완료: {}", strategy);
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_server_replay(
    proxy: State<'_, ProxyV2State>,
    entries: Vec<ServerReplayEntry>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateServerReplay { entries };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 서버 리플레이 엔트리 업데이트 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_host_mappings(
    proxy: State<'_, ProxyV2State>,
    mappings: Vec<HostMapping>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateHostMappings { mappings };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 호스트 매핑 업데이트 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_ssl_proxying_list(
    proxy: State<'_, ProxyV2State>,
    mode: SslProxyingMode,
    entries: Vec<SslProxyingEntry>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateSslProxyingList { mode, entries };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 SSL Proxying 목록 업데이트 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_default_passthrough_domains(
    proxy: State<'_, ProxyV2State>,
    entries: Vec<SslProxyingEntry>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateDefaultPassthroughDomains { entries };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 기본 패스스루 도메인 목록 업데이트 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_client_certificate(
    proxy: State<'_, ProxyV2State>,
    config: Option<proxy_daemon::ClientCertConfig>,
) -> Result<(), String> {
    if let Some(ref cert_config) = config {
        if cert_config.enabled {
            let cert_config_clone = cert_config.clone();
            tokio::task::spawn_blocking(move || {
                proxy_daemon::validate_client_cert_config(&cert_config_clone)
                    .map_err(|e| format!("인증서 검증 실패: {}", e))
            })
            .await
            .map_err(|e| format!("검증 태스크 실패: {}", e))??;
        }
    }

    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateClientCertificate { config };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 클라이언트 인증서 설정 업데이트 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_request_client_cert(
    proxy: State<'_, ProxyV2State>,
    config: Option<proxy_daemon::RequestClientCertConfig>,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateRequestClientCert { config };
    sender.send_command(&cmd).await?;
    tracing::info!("Daemon에 클라이언트 인증서 요청 설정 업데이트 완료");
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_quick_settings(
    proxy: State<'_, ProxyV2State>,
    no_caching: bool,
    block_cookies: bool,
    no_gzip: bool,
) -> Result<(), String> {
    let sender = get_command_sender(&proxy).await?;
    let cmd = ClientCommand::UpdateQuickSettings {
        no_caching,
        block_cookies,
        no_gzip,
    };
    sender.send_command(&cmd).await?;
    tracing::info!(
        "Daemon에 빠른 설정 업데이트 완료: no_caching={}, block_cookies={}, no_gzip={}",
        no_caching,
        block_cookies,
        no_gzip
    );
    Ok(())
}
