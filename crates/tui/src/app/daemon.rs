/// 데몬 연결, 메시지 처리, 커맨드 전송 로직
use proxy_daemon::{BreakpointAction, ClientCommand, DaemonMessage};
use proxy_v2_models::WsConnectionEvent;
use tokio::sync::mpsc;

use super::forms::{ScriptLogEntry, WsConnection};
use super::{App, PendingBreakpointEntry};
use crate::event::Event;

impl App {
    pub(super) async fn connect_daemon(&mut self, event_tx: mpsc::UnboundedSender<Event>) {
        let port = self.port;
        let host = self.host.clone();

        match proxy_daemon::ensure_daemon(port, &host, move |msg| {
            let _ = event_tx.send(Event::Daemon(msg));
        })
        .await
        {
            Ok(conn) => {
                self.connected = true;
                self.conn = Some(conn);
            }
            Err(e) => {
                self.connected = false;
                self.set_status(&format!("Daemon error: {}", e));
            }
        }
    }

    pub(super) fn handle_daemon_message(&mut self, msg: DaemonMessage) {
        match msg {
            DaemonMessage::Event { data } => {
                if !self.paused {
                    self.transactions.push(data);
                    // Keep max 5000
                    if self.transactions.len() > 5000 {
                        self.transactions.drain(0..1000);
                        // 선택 인덱스 조정
                        if let Some(ref mut idx) = self.selected_transaction {
                            *idx = idx.saturating_sub(1000);
                        }
                    }
                }
            }
            DaemonMessage::WsMessage { data } => {
                self.ws_messages.push(data);
                if self.ws_messages.len() > 5000 {
                    self.ws_messages.drain(0..1000);
                }
            }
            DaemonMessage::WsConnection { data } => match data {
                WsConnectionEvent::Connected {
                    connection_id,
                    uri,
                    time,
                } => {
                    self.ws_connections.push(WsConnection {
                        connection_id,
                        uri,
                        time,
                        active: true,
                    });
                }
                WsConnectionEvent::Disconnected { connection_id, .. } => {
                    if let Some(conn) = self
                        .ws_connections
                        .iter_mut()
                        .find(|c| c.connection_id == connection_id)
                    {
                        conn.active = false;
                    }
                }
            },
            DaemonMessage::InterceptRulesUpdated { rules } => {
                self.rules = rules;
            }
            DaemonMessage::HostMappingsUpdated { mappings } => {
                self.host_mappings = mappings;
            }
            DaemonMessage::ScriptLog { level, message } => {
                self.script_logs.push(ScriptLogEntry {
                    level,
                    message,
                    time: std::time::Instant::now(),
                });
                // 최대 1000개 유지
                if self.script_logs.len() > 1000 {
                    self.script_logs.drain(0..500);
                }
            }
            DaemonMessage::ScriptStatus {
                active,
                path,
                message,
            } => {
                self.script_active = active;
                self.script_path = path;
                self.set_status(&message);
            }
            DaemonMessage::ScriptResult { success, error } => {
                if success {
                    self.set_status("Script loaded");
                } else if let Some(e) = error {
                    self.set_status(&format!("Script error: {}", e));
                }
            }
            DaemonMessage::SslProxyingListUpdated { mode, entries } => {
                self.ssl_proxying_mode = mode;
                self.ssl_proxying_entries = entries;
            }
            DaemonMessage::ClientCertificateUpdated { config } => {
                if let Some(cfg) = config {
                    self.client_cert_form.enabled = cfg.enabled;
                    self.client_cert_form.cert_path = cfg.cert_path;
                    self.client_cert_form.key_path = cfg.key_path;
                }
            }
            DaemonMessage::BreakpointRulesUpdated { rules } => {
                self.breakpoint_rules = rules;
            }
            DaemonMessage::BreakpointHit {
                id,
                transaction_id,
                phase,
                data,
            } => {
                self.pending_breakpoints.push(PendingBreakpointEntry {
                    id,
                    transaction_id,
                    phase,
                    data,
                });
                self.set_status("Breakpoint hit!");
            }
            _ => {}
        }
    }

    // --- 커맨드 전송 메서드들 ---

    pub(crate) async fn send_upstream_update(&self) {
        if let Some(conn) = &self.conn {
            let config = self.upstream_form.to_config();
            let cmd = ClientCommand::UpdateUpstreamProxy { config };
            let _ = conn.send_command(&cmd).await;
        }
    }

    pub(crate) async fn send_proxy_auth_update(&mut self) {
        if let Some(conn) = &self.conn {
            let config = self.proxy_auth_form.to_config();
            let cmd = ClientCommand::UpdateProxyAuth { config };
            let _ = conn.send_command(&cmd).await;
            if self.proxy_auth_form.enabled {
                self.set_status("Proxy Auth: ON");
            } else {
                self.set_status("Proxy Auth: OFF");
            }
        }
    }

    pub(crate) async fn send_throttle_update(&mut self) {
        if let Some(conn) = &self.conn {
            let config = self.throttle_form.to_config();
            let cmd = ClientCommand::UpdateThrottle { config };
            let _ = conn.send_command(&cmd).await;
            let label = self.throttle_form.preset.label();
            if self.throttle_form.enabled {
                self.set_status(&format!("Throttle: {}", label));
            } else {
                self.set_status("Throttle: OFF");
            }
        }
    }

    pub(crate) async fn send_quick_settings_update(&mut self) {
        if let Some(conn) = &self.conn {
            let cmd = ClientCommand::UpdateQuickSettings {
                no_caching: self.quick_settings_form.no_caching,
                block_cookies: self.quick_settings_form.block_cookies,
                no_gzip: self.quick_settings_form.no_gzip,
                block_quic: self.quick_settings_form.block_quic,
            };
            let _ = conn.send_command(&cmd).await;
            let parts: Vec<&str> = [
                if self.quick_settings_form.no_caching {
                    Some("No Caching")
                } else {
                    None
                },
                if self.quick_settings_form.block_cookies {
                    Some("Block Cookies")
                } else {
                    None
                },
                if self.quick_settings_form.no_gzip {
                    Some("No Gzip")
                } else {
                    None
                },
                if self.quick_settings_form.block_quic {
                    Some("Block QUIC")
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            .collect();
            if parts.is_empty() {
                self.set_status("Quick Settings: OFF");
            } else {
                self.set_status(&format!("Quick Settings: {}", parts.join(", ")));
            }
        }
    }

    pub(crate) async fn send_rules_update(&self) {
        if let Some(conn) = &self.conn {
            let cmd = ClientCommand::UpdateInterceptRules {
                rules: self.rules.clone(),
            };
            let _ = conn.send_command(&cmd).await;
        }
    }

    pub(crate) async fn send_breakpoint_rules_update(&self) {
        if let Some(conn) = &self.conn {
            let cmd = ClientCommand::UpdateBreakpointRules {
                rules: self.breakpoint_rules.clone(),
            };
            let _ = conn.send_command(&cmd).await;
        }
    }

    pub(crate) async fn send_breakpoint_resolve(&mut self, action: BreakpointAction) {
        if let Some(idx) = self.selected_pending_bp {
            if idx < self.pending_breakpoints.len() {
                let bp = &self.pending_breakpoints[idx];
                let id = bp.id.clone();
                if let Some(conn) = &self.conn {
                    let cmd = ClientCommand::ResolveBreakpoint {
                        id,
                        action: action.clone(),
                    };
                    let _ = conn.send_command(&cmd).await;
                }
                self.pending_breakpoints.remove(idx);
                if self.pending_breakpoints.is_empty() {
                    self.selected_pending_bp = None;
                } else if idx >= self.pending_breakpoints.len() {
                    self.selected_pending_bp = Some(self.pending_breakpoints.len() - 1);
                }
                let action_name = match action {
                    BreakpointAction::Forward => "forwarded",
                    BreakpointAction::ModifyAndForward { .. } => "modified & forwarded",
                    BreakpointAction::Drop => "dropped",
                    BreakpointAction::Abort => "aborted",
                };
                self.set_status(&format!("Breakpoint {}", action_name));
            }
        }
    }

    pub(crate) async fn send_script_load(&mut self, path: &str) {
        if let Some(conn) = &self.conn {
            let cmd = ClientCommand::LoadScript {
                path: Some(path.to_string()),
                code: None,
            };
            match conn.send_command(&cmd).await {
                Ok(()) => {
                    self.script_path_input = path.to_string();
                    self.set_status(&format!("Loading script: {}", path));
                }
                Err(e) => self.set_status(&format!("Failed to send: {}", e)),
            }
        }
    }

    pub(crate) async fn send_script_unload(&mut self) {
        if let Some(conn) = &self.conn {
            let cmd = ClientCommand::UnloadScript;
            let _ = conn.send_command(&cmd).await;
            self.script_active = false;
            self.script_path = None;
            self.set_status("Script unloaded");
        }
    }

    pub(crate) async fn send_client_cert_update(&mut self) {
        if let Some(conn) = &self.conn {
            let config = self.client_cert_form.to_config();
            let cmd = ClientCommand::UpdateClientCertificate { config };
            let _ = conn.send_command(&cmd).await;
            if self.client_cert_form.enabled {
                self.set_status("Client Certificate: ON");
            } else {
                self.set_status("Client Certificate: OFF");
            }
        }
    }

    pub(crate) async fn send_ssl_proxying_update(&mut self) {
        if let Some(conn) = &self.conn {
            let cmd = ClientCommand::UpdateSslProxyingList {
                mode: self.ssl_proxying_mode.clone(),
                entries: self.ssl_proxying_entries.clone(),
            };
            let _ = conn.send_command(&cmd).await;
            let count = self.ssl_proxying_entries.len();
            self.set_status(&format!(
                "SSL Proxying ({:?}): {} entries",
                self.ssl_proxying_mode, count
            ));
        }
    }

    pub(crate) async fn send_host_mappings_update(&mut self) {
        if let Some(conn) = &self.conn {
            let cmd = ClientCommand::UpdateHostMappings {
                mappings: self.host_mappings.clone(),
            };
            let _ = conn.send_command(&cmd).await;
            let count = self.host_mappings.len();
            self.set_status(&format!("Host mappings: {} entries", count));
        }
    }
}
