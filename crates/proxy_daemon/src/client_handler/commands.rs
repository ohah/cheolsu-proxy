use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::protocol::{ClientCommand, DaemonMessage, TlsPassthroughEntry};

use super::watcher::start_file_watcher;
use super::SharedDaemonState;

/// `handle_command`에 전달되는 파라미터를 그룹화한 상태 구조체.
/// 기존 15개 이상의 개별 파라미터를 하나로 묶어 가독성과 유지보수성을 높인다.
/// 참고: `ClientHandlerContext`(mod.rs)는 클라이언트 접속 시 일회성으로 전달되는 초기화 컨텍스트이고,
/// `CommandState`는 커맨드 처리 루프 동안 유지되는 공유 상태를 나타낸다.
pub(super) struct CommandState {
    pub(super) writer: Arc<Mutex<tokio::net::unix::OwnedWriteHalf>>,
    pub(super) shared: SharedDaemonState,
    pub(super) subscribed: Arc<std::sync::atomic::AtomicBool>,
    pub(super) watched_path: Arc<Mutex<Option<String>>>,
}

impl CommandState {
    /// writer를 잠그고, 메시지를 JSON 직렬화 후 전송하는 헬퍼.
    /// 에러는 무시한다 (기존 호출부와 동일한 동작).
    pub(super) async fn send_message(&self, msg: &DaemonMessage) {
        let mut line = serde_json::to_string(msg).unwrap_or_default();
        line.push('\n');
        let mut w = self.writer.lock().await;
        let _ = w.write_all(line.as_bytes()).await;
        let _ = w.flush().await;
    }
}

/// 커맨드를 처리하고, Stop 커맨드인 경우 true를 반환합니다.
pub(super) async fn handle_command(cmd: ClientCommand, ctx: &CommandState) -> bool {
    let s = &ctx.shared;
    match cmd {
        ClientCommand::Subscribe => {
            ctx.subscribed
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        ClientCommand::UpdateInterceptRules { rules } => {
            info!("Intercept rules updated from client: {} rules", rules.len());
            if let Err(e) = s.channels.intercept_tx.send(rules.clone()) {
                warn!("인터셉트 규칙 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::InterceptRulesUpdated { rules };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if s.event_tx.receiver_count() > 0 {
                    if let Err(e) = s.event_tx.send(json) {
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
                        ctx.send_message(&result).await;
                        return false;
                    }
                }
            } else {
                Message::Text(payload.into())
            };

            let result = match direction.as_str() {
                "to_client" => s.ws_registry.inject_to_client(&connection_id, msg).await,
                "to_server" => s.ws_registry.inject_to_server(&connection_id, msg).await,
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
            ctx.send_message(&response).await;
        }
        ClientCommand::UpdateUpstreamProxy { config } => {
            info!(
                "Upstream proxy config updated: {:?}",
                config.as_ref().map(|c| c.address())
            );
            if let Err(e) = s.channels.upstream_tx.send(config) {
                warn!("업스트림 프록시 watch 채널 전송 실패: {}", e);
            }
        }
        ClientCommand::UpdateThrottle { config } => {
            info!(
                "Throttle config updated: enabled={:?}",
                config.as_ref().map(|c| c.enabled)
            );
            if let Err(e) = s.channels.throttle_tx.send(config) {
                warn!("스로틀 설정 watch 채널 전송 실패: {}", e);
            }
        }
        ClientCommand::UpdateHostMappings { mappings } => {
            info!(
                "Host mappings updated from client: {} mappings",
                mappings.len()
            );
            if let Err(e) = s.channels.host_mapping_tx.send(mappings.clone()) {
                warn!("호스트 매핑 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::HostMappingsUpdated { mappings };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if s.event_tx.receiver_count() > 0 {
                    if let Err(e) = s.event_tx.send(json) {
                        warn!("호스트 매핑 broadcast 전송 실패: {}", e);
                    }
                }
            }
        }
        ClientCommand::UpdateReverseProxyRules { rules } => {
            info!(
                "Reverse proxy rules updated from client: {} rules",
                rules.len()
            );
            if let Err(e) = s.channels.reverse_proxy_tx.send(rules.clone()) {
                warn!("리버스 프록시 규칙 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::ReverseProxyRulesUpdated { rules };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if s.event_tx.receiver_count() > 0 {
                    if let Err(e) = s.event_tx.send(json) {
                        warn!("리버스 프록시 규칙 broadcast 전송 실패: {}", e);
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
            if let Err(e) = s
                .channels
                .ssl_proxying_tx
                .send((mode.clone(), entries.clone()))
            {
                warn!("SSL 프록싱 목록 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::SslProxyingListUpdated { mode, entries };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if s.event_tx.receiver_count() > 0 {
                    if let Err(e) = s.event_tx.send(json) {
                        warn!("SSL 프록싱 목록 broadcast 전송 실패: {}", e);
                    }
                }
            }
        }
        ClientCommand::UpdateDefaultPassthroughDomains { entries } => {
            info!(
                "Default passthrough domains updated from client: {} entries",
                entries.len()
            );
            if let Err(e) = s.channels.default_passthrough_tx.send(entries.clone()) {
                warn!("기본 패스스루 도메인 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::DefaultPassthroughDomainsUpdated { entries };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if s.event_tx.receiver_count() > 0 {
                    if let Err(e) = s.event_tx.send(json) {
                        warn!("기본 패스스루 도메인 broadcast 전송 실패: {}", e);
                    }
                }
            }
        }
        ClientCommand::UpdateClientCertificate { config } => {
            info!(
                "Client certificate config updated: enabled={:?}",
                config.as_ref().map(|c| c.enabled)
            );
            if let Err(e) = s.channels.client_cert_tx.send(config.clone()) {
                warn!("클라이언트 인증서 설정 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::ClientCertificateUpdated { config };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if s.event_tx.receiver_count() > 0 {
                    if let Err(e) = s.event_tx.send(json) {
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
                let mut settings = s.quick_settings.write().await;
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
                let mut auth = s.proxy_auth.write().await;
                *auth = Some(config);
            }
        }
        ClientCommand::UpdateServerReplay { entries } => {
            info!("Server replay entries updated: {} entries", entries.len());
            if let Err(e) = s.channels.server_replay_tx.send(entries) {
                warn!("서버 리플레이 watch 채널 전송 실패: {}", e);
            }
        }
        ClientCommand::LoadScript { path, code } => {
            let result: Result<(), String> = if let Some(file_path) = &path {
                s.script_handle
                    .load_file(file_path)
                    .await
                    .map_err(|e| e.to_string())
            } else if let Some(script_code) = &code {
                s.script_handle
                    .load_ts_code(script_code)
                    .await
                    .map_err(|e| e.to_string())
            } else {
                Err("path 또는 code 중 하나가 필요합니다".to_string())
            };

            let response = match &result {
                Ok(()) => {
                    info!("Script loaded successfully");
                    // 파일 감시 시작
                    if let Some(file_path) = &path {
                        let mut wp = ctx.watched_path.lock().await;
                        *wp = Some(file_path.clone());
                        start_file_watcher(
                            file_path.clone(),
                            s.script_handle.clone(),
                            ctx.writer.clone(),
                            ctx.watched_path.clone(),
                            s.event_tx.clone(),
                        );
                    }
                    // 스크립트 상태 브로드캐스트
                    let status_msg = DaemonMessage::ScriptStatus {
                        active: true,
                        path: path.clone(),
                        message: "스크립트 로드 완료".to_string(),
                    };
                    if let Ok(json) = serde_json::to_string(&status_msg) {
                        let _ = s.event_tx.send(json);
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
            ctx.send_message(&response).await;
        }
        ClientCommand::UnloadScript => {
            s.script_handle.unload().await;
            {
                let mut wp = ctx.watched_path.lock().await;
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
                let _ = s.event_tx.send(json);
            }
            let response = DaemonMessage::ScriptResult {
                success: true,
                error: None,
            };
            ctx.send_message(&response).await;
        }
        ClientCommand::UpdateBreakpointRules { rules } => {
            info!(
                "Breakpoint rules updated from client: {} rules",
                rules.len()
            );
            if let Err(e) = s.channels.breakpoint_tx.send(rules.clone()) {
                warn!("브레이크포인트 규칙 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::BreakpointRulesUpdated { rules };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if s.event_tx.receiver_count() > 0 {
                    if let Err(e) = s.event_tx.send(json) {
                        warn!("브레이크포인트 규칙 broadcast 전송 실패: {}", e);
                    }
                }
            }
        }
        ClientCommand::ResolveBreakpoint { id, action } => {
            info!("Resolving breakpoint: {} -> {:?}", id, action);
            if let Err(e) = s.breakpoint_manager.resolve(&id, action).await {
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
            ctx.send_message(&msg).await;
        }
        ClientCommand::LoadSession { path } => {
            info!("LoadSession command received: path={}", path);
            // NOTE: SaveSession/LoadSession are primarily handled by MCP server directly.
            // This handler sends an acknowledgment message but does not perform the actual save/load.
            let msg = DaemonMessage::SessionLoaded {
                path: path.clone(),
                transaction_count: 0,
            };
            ctx.send_message(&msg).await;
        }
        ClientCommand::HealthCheck => {
            let uptime_secs = s.metrics.started_at.elapsed().as_secs();
            let active_conns = s.client_count.load(std::sync::atomic::Ordering::Acquire) as u32;
            let total_txns = s
                .metrics
                .total_transactions
                .load(std::sync::atomic::Ordering::Relaxed);
            let response = DaemonMessage::HealthCheckResult {
                uptime_secs,
                active_connections: active_conns,
                total_transactions: total_txns,
            };
            ctx.send_message(&response).await;
        }
        ClientCommand::GetTlsPassthroughList => {
            let list = s.tls_passthrough.list_bypassed().await;
            let entries: Vec<TlsPassthroughEntry> = list
                .into_iter()
                .map(|(host, failure_count)| TlsPassthroughEntry {
                    host,
                    failure_count,
                })
                .collect();
            let response = DaemonMessage::TlsPassthroughUpdated { entries };
            ctx.send_message(&response).await;
        }
        ClientCommand::RemoveTlsPassthrough { host } => {
            info!("TLS Passthrough 바이패스 해제: {}", host);
            s.tls_passthrough.clear_domain(&host).await;
        }
        ClientCommand::ClearTlsPassthrough => {
            info!("TLS Passthrough 전체 초기화");
            s.tls_passthrough.clear_all().await;
        }
        ClientCommand::UpdateRequestClientCert { config } => {
            info!(
                "Request client cert config updated: enabled={:?}",
                config.as_ref().map(|c| c.enabled)
            );
            if let Err(e) = s.channels.request_client_cert_tx.send(config.clone()) {
                warn!("클라이언트 인증서 요청 설정 watch 채널 전송 실패: {}", e);
            }
            let broadcast_msg = DaemonMessage::RequestClientCertUpdated { config };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                if s.event_tx.receiver_count() > 0 {
                    if let Err(e) = s.event_tx.send(json) {
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
            s.connection_strategy
                .store(strategy_value, std::sync::atomic::Ordering::Release);
            info!("Connection strategy updated: {}", strategy);
        }
        ClientCommand::GetMetrics => {
            let snapshot = s.metrics.metrics_aggregator.get_metrics_snapshot();
            let uptime = s.metrics.metrics_aggregator.uptime_secs();
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
            ctx.send_message(&response).await;
        }
        ClientCommand::GetDomainStats { domain } => {
            let stats = s
                .metrics
                .metrics_aggregator
                .get_domain_stats(domain.as_deref())
                .await;
            let response = DaemonMessage::DomainStatsResult { stats };
            ctx.send_message(&response).await;
        }
        ClientCommand::GetRecentErrors { limit } => {
            let errors = s.metrics.metrics_aggregator.get_recent_errors(limit).await;
            let response = DaemonMessage::RecentErrorsResult { errors };
            ctx.send_message(&response).await;
        }
        ClientCommand::Stop => {
            return true;
        }
    }
    false
}
