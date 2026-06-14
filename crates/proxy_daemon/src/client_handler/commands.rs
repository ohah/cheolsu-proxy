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
    /// 현재 실행 중인 파일 감시 태스크 핸들(클라이언트 종료/스크립트 교체 시 abort용)
    pub(super) watcher_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
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
            s.broadcast_event(&DaemonMessage::InterceptRulesUpdated { rules });
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
            s.broadcast_event(&DaemonMessage::HostMappingsUpdated { mappings });
        }
        ClientCommand::UpdateReverseProxyRules { rules } => {
            info!(
                "Reverse proxy rules updated from client: {} rules",
                rules.len()
            );
            if let Err(e) = s.channels.reverse_proxy_tx.send(rules.clone()) {
                warn!("리버스 프록시 규칙 watch 채널 전송 실패: {}", e);
            }
            s.broadcast_event(&DaemonMessage::ReverseProxyRulesUpdated { rules });
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
            s.broadcast_event(&DaemonMessage::SslProxyingListUpdated { mode, entries });
        }
        ClientCommand::UpdateDefaultPassthroughDomains { entries } => {
            info!(
                "Default passthrough domains updated from client: {} entries",
                entries.len()
            );
            if let Err(e) = s.channels.default_passthrough_tx.send(entries.clone()) {
                warn!("기본 패스스루 도메인 watch 채널 전송 실패: {}", e);
            }
            s.broadcast_event(&DaemonMessage::DefaultPassthroughDomainsUpdated { entries });
        }
        ClientCommand::UpdateClientCertificate { config } => {
            info!(
                "Client certificate config updated: enabled={:?}",
                config.as_ref().map(|c| c.enabled)
            );
            if let Err(e) = s.channels.client_cert_tx.send(config.clone()) {
                warn!("클라이언트 인증서 설정 watch 채널 전송 실패: {}", e);
            }
            s.broadcast_event(&DaemonMessage::ClientCertificateUpdated { config });
        }
        ClientCommand::UpdateQuickSettings {
            no_caching,
            block_cookies,
            no_gzip,
            block_quic,
        } => {
            info!(
                "Quick settings updated: no_caching={}, block_cookies={}, no_gzip={}, block_quic={}",
                no_caching, block_cookies, no_gzip, block_quic
            );
            {
                let settings = crate::handler::QuickSettings {
                    no_caching,
                    block_cookies,
                    no_gzip,
                    block_quic,
                };
                s.quick_settings
                    .store(settings.to_bits(), std::sync::atomic::Ordering::Release);
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
                        {
                            let mut wp = ctx.watched_path.lock().await;
                            *wp = Some(file_path.clone());
                        }
                        let new_handle = start_file_watcher(
                            file_path.clone(),
                            s.script_handle.clone(),
                            ctx.writer.clone(),
                            ctx.watched_path.clone(),
                            s.event_tx.clone(),
                        );
                        // 이전 감시 태스크를 정리해 태스크/소켓 FD 누수를 방지한다.
                        if let Some(old) = ctx.watcher_handle.lock().await.replace(new_handle) {
                            old.abort();
                        }
                    }
                    // 스크립트 상태 브로드캐스트
                    s.broadcast_event(&DaemonMessage::ScriptStatus {
                        active: true,
                        path: path.clone(),
                        message: "스크립트 로드 완료".to_string(),
                    });
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
            s.broadcast_event(&DaemonMessage::ScriptStatus {
                active: false,
                path: None,
                message: "스크립트 언로드됨".to_string(),
            });
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
            s.broadcast_event(&DaemonMessage::BreakpointRulesUpdated { rules });
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
            let list = s.tls_passthrough.list_bypassed();
            let entries: Vec<TlsPassthroughEntry> = list
                .into_iter()
                .map(|entry| TlsPassthroughEntry {
                    host: entry.host,
                    failure_count: entry.failure_count,
                    active: entry.active,
                    blocked_by_never_passthrough: entry.blocked_by_never_passthrough,
                    last_failure_unix_secs: entry.last_failure_unix_secs,
                    expires_at_unix_secs: entry.expires_at_unix_secs,
                })
                .collect();
            let response = DaemonMessage::TlsPassthroughUpdated { entries };
            ctx.send_message(&response).await;
        }
        ClientCommand::RemoveTlsPassthrough { host } => {
            info!("TLS Passthrough 바이패스 해제: {}", host);
            s.tls_passthrough.clear_domain(&host);
        }
        ClientCommand::ClearTlsPassthrough => {
            info!("TLS Passthrough 전체 초기화");
            s.tls_passthrough.clear_all();
        }
        ClientCommand::UpdateRequestClientCert { config } => {
            info!(
                "Request client cert config updated: enabled={:?}",
                config.as_ref().map(|c| c.enabled)
            );
            if let Err(e) = s.channels.request_client_cert_tx.send(config.clone()) {
                warn!("클라이언트 인증서 요청 설정 watch 채널 전송 실패: {}", e);
            }
            s.broadcast_event(&DaemonMessage::RequestClientCertUpdated { config });
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
        ClientCommand::LoadContractSpec { path } => {
            info!("Loading contract spec: {}", path);
            match s.contract_validator.load_spec(&path) {
                Ok(spec_info) => {
                    let response = DaemonMessage::ContractSpecLoaded { spec: spec_info };
                    ctx.send_message(&response).await;
                    // 전체 목록도 브로드캐스트
                    let specs = s.contract_validator.list_specs();
                    let broadcast_msg = DaemonMessage::ContractSpecsUpdated { specs };
                    if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                        let _ = s.event_tx.send(json);
                    }
                }
                Err(e) => {
                    error!("Failed to load contract spec: {}", e);
                    let response = DaemonMessage::ContractSpecError { error: e };
                    ctx.send_message(&response).await;
                }
            }
        }
        ClientCommand::UnloadContractSpec { id } => {
            info!("Unloading contract spec: {}", id);
            s.contract_validator.unload_spec(&id);
            let specs = s.contract_validator.list_specs();
            let broadcast_msg = DaemonMessage::ContractSpecsUpdated { specs };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                let _ = s.event_tx.send(json);
            }
        }
        ClientCommand::ToggleContractSpec { id, enabled } => {
            info!("Toggle contract spec: {} -> {}", id, enabled);
            s.contract_validator.toggle_spec(&id, enabled);
            let specs = s.contract_validator.list_specs();
            let broadcast_msg = DaemonMessage::ContractSpecsUpdated { specs };
            if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                let _ = s.event_tx.send(json);
            }
        }
        ClientCommand::GetContractSpecs => {
            let specs = s.contract_validator.list_specs();
            let response = DaemonMessage::ContractSpecsUpdated { specs };
            ctx.send_message(&response).await;
        }
        ClientCommand::GetGraphqlOperations => {
            // GraphQL 오퍼레이션은 클라이언트 측에서 기존 트랜잭션 데이터를 필터링하여 표시하므로
            // 별도의 서버 측 처리가 필요하지 않습니다.
            info!("GraphQL 오퍼레이션 목록 조회 요청 수신");
        }
        ClientCommand::GetTlsConfigRules => {
            if let Some(tls_config) = &s.tls_config {
                let guard = tls_config.read().await;
                let rules = guard.rules().to_vec();
                let response = DaemonMessage::TlsConfigRules { rules };
                ctx.send_message(&response).await;
            } else {
                let response = DaemonMessage::TlsConfigRules { rules: vec![] };
                ctx.send_message(&response).await;
            }
        }
        ClientCommand::UpdateTlsConfigRules { rules } => {
            info!("TLS config rules updated: {} rules", rules.len());
            if let Some(tls_config) = &s.tls_config {
                let mut guard = tls_config.write().await;
                let old_count = guard.rules().len();
                guard.replace_rules(rules.clone());
                info!(
                    "TLS config rules replaced: {} -> {} rules",
                    old_count,
                    guard.rules().len()
                );
            }
            let response = DaemonMessage::TlsConfigRules { rules };
            ctx.send_message(&response).await;
        }
        ClientCommand::GetLearnedTlsStrategies => {
            if let Some(cache) = &s.tls_strategy_cache {
                let guard = cache.read().await;
                let strategies: Vec<proxyapi_v2::hybrid_tls_handler::LearnedTlsStrategy> = guard
                    .iter()
                    .map(|(domain, &strategy)| {
                        proxyapi_v2::hybrid_tls_handler::LearnedTlsStrategy {
                            domain: domain.clone(),
                            strategy,
                        }
                    })
                    .collect();
                let response = DaemonMessage::LearnedTlsStrategies { strategies };
                ctx.send_message(&response).await;
            } else {
                let response = DaemonMessage::LearnedTlsStrategies { strategies: vec![] };
                ctx.send_message(&response).await;
            }
        }
        ClientCommand::ClearLearnedTlsStrategies => {
            if let Some(cache) = &s.tls_strategy_cache {
                let mut guard = cache.write().await;
                let count = guard.len();
                guard.clear();
                info!("Learned TLS strategies cleared: {} entries removed", count);
            }
        }
        ClientCommand::UpdateNeverPassthroughDomains { entries } => {
            info!("Never Passthrough 도메인 업데이트: {}개", entries.len());
            s.tls_passthrough.set_never_passthrough(entries.clone());
            s.broadcast_event(&DaemonMessage::NeverPassthroughDomainsUpdated { entries });
        }
        ClientCommand::GetNeverPassthroughDomains => {
            let entries = s.tls_passthrough.list_never_passthrough();
            let response = DaemonMessage::NeverPassthroughDomainsUpdated { entries };
            ctx.send_message(&response).await;
        }
        ClientCommand::Stop => {
            return true;
        }
    }
    false
}
