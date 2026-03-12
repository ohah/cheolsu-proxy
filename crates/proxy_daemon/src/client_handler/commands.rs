use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info, warn};

use crate::breakpoint::BreakpointManager;
use crate::daemon::{DaemonChannels, DaemonMetrics};
use crate::protocol::{ClientCommand, DaemonMessage, ProxyAuthConfig, TlsPassthroughEntry};
use proxyapi_v2::websocket_registry::WebSocketRegistry;

use super::watcher::start_file_watcher;

#[allow(clippy::too_many_arguments)]
/// 커맨드를 처리하고, Stop 커맨드인 경우 true를 반환합니다.
pub(super) async fn handle_command(
    cmd: ClientCommand,
    writer: &Arc<Mutex<tokio::net::unix::OwnedWriteHalf>>,
    channels: &DaemonChannels,
    breakpoint_manager: &BreakpointManager,
    event_tx: &broadcast::Sender<String>,
    ws_registry: &WebSocketRegistry,
    script_handle: &scripting::ScriptHandle,
    quick_settings: &Arc<tokio::sync::RwLock<crate::handler::QuickSettings>>,
    proxy_auth: &Arc<tokio::sync::RwLock<Option<ProxyAuthConfig>>>,
    subscribed: &Arc<std::sync::atomic::AtomicBool>,
    metrics: &DaemonMetrics,
    client_count: &Arc<std::sync::atomic::AtomicUsize>,
    tls_passthrough: &proxyapi_v2::tls_passthrough::TlsPassthrough,
    connection_strategy: &Arc<std::sync::atomic::AtomicU8>,
    watched_path: &Arc<Mutex<Option<String>>>,
) -> bool {
    match cmd {
        ClientCommand::Subscribe => {
            subscribed.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        ClientCommand::UpdateInterceptRules { rules } => {
            info!("Intercept rules updated from client: {} rules", rules.len());
            if let Err(e) = channels.intercept_tx.send(rules.clone()) {
                warn!("인터셉트 규칙 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::InterceptRulesUpdated { rules };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if event_tx.receiver_count() > 0 {
                    if let Err(e) = event_tx.send(json) {
                        warn!("인터셉트 규칙 broadcast 전송 실패: {}", e);
                    }
                }
            }
        }
        ClientCommand::WsInject {
            connection_id,
            direction,
            payload,
            is_binary,
        } => {
            use tokio_tungstenite::tungstenite::Message;

            let msg = if is_binary {
                use base64::Engine;
                match base64::engine::general_purpose::STANDARD.decode(&payload) {
                    Ok(bytes) => Message::Binary(bytes.into()),
                    Err(e) => {
                        let result = DaemonMessage::WsInjectResult {
                            success: false,
                            error: Some(format!("Base64 decode failed: {}", e)),
                        };
                        let mut line = serde_json::to_string(&result).unwrap_or_default();
                        line.push('\n');
                        let mut w = writer.lock().await;
                        let _ = w.write_all(line.as_bytes()).await;
                        let _ = w.flush().await;
                        return false;
                    }
                }
            } else {
                Message::Text(payload.into())
            };

            let result = match direction.as_str() {
                "to_client" => ws_registry.inject_to_client(&connection_id, msg).await,
                "to_server" => ws_registry.inject_to_server(&connection_id, msg).await,
                _ => Err(format!("Invalid direction: {}", direction)),
            };

            let response = match result {
                Ok(()) => DaemonMessage::WsInjectResult {
                    success: true,
                    error: None,
                },
                Err(e) => DaemonMessage::WsInjectResult {
                    success: false,
                    error: Some(e),
                },
            };
            let mut line = serde_json::to_string(&response).unwrap_or_default();
            line.push('\n');
            let mut w = writer.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
            let _ = w.flush().await;
        }
        ClientCommand::UpdateUpstreamProxy { config } => {
            info!(
                "Upstream proxy config updated: {:?}",
                config.as_ref().map(|c| c.address())
            );
            if let Err(e) = channels.upstream_tx.send(config) {
                warn!("업스트림 프록시 watch 채널 전송 실패: {}", e);
            }
        }
        ClientCommand::UpdateThrottle { config } => {
            info!(
                "Throttle config updated: enabled={:?}",
                config.as_ref().map(|c| c.enabled)
            );
            if let Err(e) = channels.throttle_tx.send(config) {
                warn!("스로틀 설정 watch 채널 전송 실패: {}", e);
            }
        }
        ClientCommand::UpdateHostMappings { mappings } => {
            info!(
                "Host mappings updated from client: {} mappings",
                mappings.len()
            );
            if let Err(e) = channels.host_mapping_tx.send(mappings.clone()) {
                warn!("호스트 매핑 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::HostMappingsUpdated { mappings };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if event_tx.receiver_count() > 0 {
                    if let Err(e) = event_tx.send(json) {
                        warn!("호스트 매핑 broadcast 전송 실패: {}", e);
                    }
                }
            }
        }
        ClientCommand::UpdateSslProxyingList { mode, entries } => {
            info!(
                "SSL Proxying list updated from client: mode={:?}, {} entries",
                mode,
                entries.len()
            );
            if let Err(e) = channels
                .ssl_proxying_tx
                .send((mode.clone(), entries.clone()))
            {
                warn!("SSL 프록싱 목록 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::SslProxyingListUpdated { mode, entries };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if event_tx.receiver_count() > 0 {
                    if let Err(e) = event_tx.send(json) {
                        warn!("SSL 프록싱 목록 broadcast 전송 실패: {}", e);
                    }
                }
            }
        }
        ClientCommand::UpdateClientCertificate { config } => {
            info!(
                "Client certificate config updated: enabled={:?}",
                config.as_ref().map(|c| c.enabled)
            );
            if let Err(e) = channels.client_cert_tx.send(config.clone()) {
                warn!("클라이언트 인증서 설정 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::ClientCertificateUpdated { config };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if event_tx.receiver_count() > 0 {
                    if let Err(e) = event_tx.send(json) {
                        warn!("클라이언트 인증서 설정 broadcast 전송 실패: {}", e);
                    }
                }
            }
        }
        ClientCommand::UpdateQuickSettings {
            no_caching,
            block_cookies,
            no_gzip,
        } => {
            info!(
                "Quick settings updated: no_caching={}, block_cookies={}, no_gzip={}",
                no_caching, block_cookies, no_gzip
            );
            {
                let mut settings = quick_settings.write().await;
                settings.no_caching = no_caching;
                settings.block_cookies = block_cookies;
                settings.no_gzip = no_gzip;
            }
        }
        ClientCommand::UpdateProxyAuth { config } => {
            info!(
                "Proxy auth config updated: enabled={}, username={}",
                config.enabled, config.username
            );
            {
                let mut auth = proxy_auth.write().await;
                *auth = Some(config);
            }
        }
        ClientCommand::UpdateServerReplay { entries } => {
            info!("Server replay entries updated: {} entries", entries.len());
            if let Err(e) = channels.server_replay_tx.send(entries) {
                warn!("서버 리플레이 watch 채널 전송 실패: {}", e);
            }
        }
        ClientCommand::LoadScript { path, code } => {
            let result: Result<(), String> = if let Some(file_path) = &path {
                script_handle
                    .load_file(file_path)
                    .await
                    .map_err(|e| e.to_string())
            } else if let Some(script_code) = &code {
                // JS로 먼저 시도, 실패 시 TS로 트랜스파일
                match script_handle.load_code(script_code).await {
                    Ok(()) => Ok(()),
                    Err(_) => script_handle
                        .load_ts_code(script_code)
                        .await
                        .map_err(|e| e.to_string()),
                }
            } else {
                Err("path 또는 code 중 하나가 필요합니다".to_string())
            };

            let response = match &result {
                Ok(()) => {
                    info!("Script loaded successfully");
                    // 파일 감시 시작
                    if let Some(file_path) = &path {
                        let mut wp = watched_path.lock().await;
                        *wp = Some(file_path.clone());
                        start_file_watcher(
                            file_path.clone(),
                            script_handle.clone(),
                            writer.clone(),
                            watched_path.clone(),
                            event_tx.clone(),
                        );
                    }
                    // 스크립트 상태 브로드캐스트
                    let status_msg = DaemonMessage::ScriptStatus {
                        active: true,
                        path: path.clone(),
                        message: "스크립트 로드 완료".to_string(),
                    };
                    if let Ok(json) = serde_json::to_string(&status_msg) {
                        let _ = event_tx.send(json);
                    }
                    DaemonMessage::ScriptResult {
                        success: true,
                        error: None,
                    }
                }
                Err(e) => {
                    error!("Script load failed: {}", e);
                    DaemonMessage::ScriptResult {
                        success: false,
                        error: Some(e.clone()),
                    }
                }
            };
            let mut line = serde_json::to_string(&response).unwrap_or_default();
            line.push('\n');
            let mut w = writer.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
            let _ = w.flush().await;
        }
        ClientCommand::UnloadScript => {
            script_handle.unload().await;
            {
                let mut wp = watched_path.lock().await;
                *wp = None;
            }
            info!("Script unloaded");
            // 스크립트 상태 브로드캐스트
            let status_msg = DaemonMessage::ScriptStatus {
                active: false,
                path: None,
                message: "스크립트 언로드됨".to_string(),
            };
            if let Ok(json) = serde_json::to_string(&status_msg) {
                let _ = event_tx.send(json);
            }
            let response = DaemonMessage::ScriptResult {
                success: true,
                error: None,
            };
            let mut line = serde_json::to_string(&response).unwrap_or_default();
            line.push('\n');
            let mut w = writer.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
            let _ = w.flush().await;
        }
        ClientCommand::UpdateBreakpointRules { rules } => {
            info!(
                "Breakpoint rules updated from client: {} rules",
                rules.len()
            );
            if let Err(e) = channels.breakpoint_tx.send(rules.clone()) {
                warn!("브레이크포인트 규칙 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::BreakpointRulesUpdated { rules };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if event_tx.receiver_count() > 0 {
                    if let Err(e) = event_tx.send(json) {
                        warn!("브레이크포인트 규칙 broadcast 전송 실패: {}", e);
                    }
                }
            }
        }
        ClientCommand::ResolveBreakpoint { id, action } => {
            info!("Resolving breakpoint: {} -> {:?}", id, action);
            if let Err(e) = breakpoint_manager.resolve(&id, action).await {
                warn!("Failed to resolve breakpoint: {}", e);
            }
        }
        ClientCommand::SaveSession { path, filter } => {
            info!(
                "SaveSession command received: path={}, filter={:?}",
                path, filter
            );
            // NOTE: SaveSession/LoadSession are primarily handled by MCP server directly.
            // This handler sends an acknowledgment message but does not perform the actual save/load.
            let msg = DaemonMessage::SessionSaved {
                path: path.clone(),
                transaction_count: 0,
            };
            let mut line = serde_json::to_string(&msg).unwrap_or_default();
            line.push('\n');
            let mut w = writer.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
            let _ = w.flush().await;
        }
        ClientCommand::LoadSession { path } => {
            info!("LoadSession command received: path={}", path);
            // NOTE: SaveSession/LoadSession are primarily handled by MCP server directly.
            // This handler sends an acknowledgment message but does not perform the actual save/load.
            let msg = DaemonMessage::SessionLoaded {
                path: path.clone(),
                transaction_count: 0,
            };
            let mut line = serde_json::to_string(&msg).unwrap_or_default();
            line.push('\n');
            let mut w = writer.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
            let _ = w.flush().await;
        }
        ClientCommand::HealthCheck => {
            let uptime_secs = metrics.started_at.elapsed().as_secs();
            let active_conns = client_count.load(std::sync::atomic::Ordering::Relaxed) as u32;
            let total_txns = metrics
                .total_transactions
                .load(std::sync::atomic::Ordering::Relaxed);
            let response = DaemonMessage::HealthCheckResult {
                uptime_secs,
                active_connections: active_conns,
                total_transactions: total_txns,
            };
            let mut line = serde_json::to_string(&response).unwrap_or_default();
            line.push('\n');
            let mut w = writer.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
            let _ = w.flush().await;
        }
        ClientCommand::GetTlsPassthroughList => {
            let list = tls_passthrough.list_bypassed().await;
            let entries: Vec<TlsPassthroughEntry> = list
                .into_iter()
                .map(|(host, failure_count)| TlsPassthroughEntry {
                    host,
                    failure_count,
                })
                .collect();
            let response = DaemonMessage::TlsPassthroughUpdated { entries };
            let mut line = serde_json::to_string(&response).unwrap_or_default();
            line.push('\n');
            let mut w = writer.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
            let _ = w.flush().await;
        }
        ClientCommand::RemoveTlsPassthrough { host } => {
            info!("TLS Passthrough 바이패스 해제: {}", host);
            tls_passthrough.clear_domain(&host).await;
        }
        ClientCommand::ClearTlsPassthrough => {
            info!("TLS Passthrough 전체 초기화");
            tls_passthrough.clear_all().await;
        }
        ClientCommand::UpdateRequestClientCert { config } => {
            info!(
                "Request client cert config updated: enabled={:?}",
                config.as_ref().map(|c| c.enabled)
            );
            if let Err(e) = channels.request_client_cert_tx.send(config.clone()) {
                warn!("클라이언트 인증서 요청 설정 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::RequestClientCertUpdated { config };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if event_tx.receiver_count() > 0 {
                    if let Err(e) = event_tx.send(json) {
                        warn!("클라이언트 인증서 요청 설정 broadcast 전송 실패: {}", e);
                    }
                }
            }
        }
        ClientCommand::UpdateConnectionStrategy { strategy } => {
            let strategy_value = match strategy.as_str() {
                "eager" => 1u8,
                "eager_with_fallback" => 2u8,
                _ => 0u8, // lazy
            };
            connection_strategy.store(strategy_value, std::sync::atomic::Ordering::Relaxed);
            info!("Connection strategy updated: {}", strategy);
        }
        ClientCommand::GetMetrics => {
            let snapshot = metrics.metrics_aggregator.get_metrics_snapshot();
            let uptime = metrics.metrics_aggregator.uptime_secs();
            let response = DaemonMessage::MetricsResult {
                active_requests: snapshot.active_requests,
                total_requests: snapshot.total_requests,
                total_bytes_sent: snapshot.total_bytes_sent,
                total_bytes_received: snapshot.total_bytes_received,
                total_tls_handshakes: snapshot.total_tls_handshakes,
                total_tls_failures: snapshot.total_tls_failures,
                total_connection_failures: snapshot.total_connection_failures,
                total_timeouts: snapshot.total_timeouts,
                uptime_secs: uptime,
            };
            let mut line = serde_json::to_string(&response).unwrap_or_default();
            line.push('\n');
            let mut w = writer.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
            let _ = w.flush().await;
        }
        ClientCommand::GetDomainStats { domain } => {
            let stats = metrics
                .metrics_aggregator
                .get_domain_stats(domain.as_deref())
                .await;
            let response = DaemonMessage::DomainStatsResult { stats };
            let mut line = serde_json::to_string(&response).unwrap_or_default();
            line.push('\n');
            let mut w = writer.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
            let _ = w.flush().await;
        }
        ClientCommand::GetRecentErrors { limit } => {
            let errors = metrics.metrics_aggregator.get_recent_errors(limit).await;
            let response = DaemonMessage::RecentErrorsResult { errors };
            let mut line = serde_json::to_string(&response).unwrap_or_default();
            line.push('\n');
            let mut w = writer.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
            let _ = w.flush().await;
        }
        ClientCommand::Stop => {
            return true;
        }
    }
    false
}
