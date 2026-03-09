mod ca_cert;
mod forms;
mod key_handlers;
mod utils;

pub use forms::*;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use proxy_daemon::{
    diff_headers, diff_json, diff_text, format_diff_text, BodyDiff, ClientCommand,
    DaemonConnection, DaemonMessage, InterceptRule, TrafficDiff, TransactionPartDiff,
};
use proxy_v2_models::{RequestInfo, WsConnectionEvent, WsMessageInfo};
use ratatui::prelude::*;
use ratatui::widgets::TableState;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::event::{Event, EventHandler};
use crate::tabs::Tab;
use crate::ui;

/// TUI app state
pub struct App {
    pub port: u16,
    pub host: String,
    pub running: bool,
    pub tab: Tab,
    pub connected: bool,

    // Network
    pub transactions: Vec<RequestInfo>,
    pub selected_transaction: Option<usize>,
    pub show_detail: bool,
    pub detail_scroll: u16,
    pub paused: bool,

    // Diff
    pub diff_mark: Option<usize>,
    pub diff_result: Option<String>,
    pub show_diff: bool,
    pub diff_scroll: u16,

    // WebSocket
    pub ws_connections: Vec<WsConnection>,
    pub ws_messages: Vec<WsMessageInfo>,
    pub selected_ws_conn: Option<usize>,

    // Intercept Rules
    pub rules: Vec<InterceptRule>,
    pub selected_rule: Option<usize>,

    // Rule form
    pub rule_form: Option<RuleForm>,

    // Table states (for scroll)
    pub network_table_state: TableState,
    pub ws_conn_table_state: TableState,
    pub rules_table_state: TableState,

    // Script
    pub script_active: bool,
    pub script_path: Option<String>,
    pub script_path_input: String,
    pub script_editing: bool,
    pub script_logs: Vec<ScriptLogEntry>,
    pub script_log_scroll: usize,

    // Settings section
    pub settings_section: SettingsSection,

    // Upstream Proxy
    pub upstream_form: UpstreamProxyForm,

    // Throttle
    pub throttle_form: ThrottleForm,

    // CA Certificate
    pub ca_cert_installed: bool,
    pub ca_cert_path: Option<String>,

    // Remote device cert info
    pub local_ips: Vec<String>,

    // Status message
    pub status_message: Option<(String, std::time::Instant)>,

    // Connection
    conn: Option<DaemonConnection>,
    event_tx: Option<mpsc::UnboundedSender<Event>>,
}

impl App {
    pub fn new(port: u16, host: String) -> Self {
        Self {
            port,
            host,
            running: true,
            tab: Tab::Network,
            connected: false,
            transactions: Vec::new(),
            selected_transaction: None,
            show_detail: false,
            detail_scroll: 0,
            paused: false,
            diff_mark: None,
            diff_result: None,
            show_diff: false,
            diff_scroll: 0,
            ws_connections: Vec::new(),
            ws_messages: Vec::new(),
            selected_ws_conn: None,
            rules: Vec::new(),
            selected_rule: None,
            rule_form: None,
            network_table_state: TableState::default(),
            ws_conn_table_state: TableState::default(),
            rules_table_state: TableState::default(),
            script_active: false,
            script_path: None,
            script_path_input: String::new(),
            script_editing: false,
            script_logs: Vec::new(),
            script_log_scroll: 0,
            settings_section: SettingsSection::UpstreamProxy,
            upstream_form: UpstreamProxyForm::new(),
            throttle_form: ThrottleForm::new(),
            ca_cert_installed: false,
            ca_cert_path: None,
            local_ips: proxy_daemon::get_local_ips(),
            status_message: None,
            conn: None,
            event_tx: None,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Event handler
        let (mut events, event_tx) = EventHandler::new(Duration::from_millis(250));
        self.event_tx = Some(event_tx.clone());

        // Check CA certificate status
        self.check_ca_status();

        // Connect to daemon
        self.connect_daemon(event_tx.clone()).await;

        // Main loop
        while self.running {
            // Sync table states for scroll
            self.network_table_state.select(self.selected_transaction);
            self.ws_conn_table_state.select(self.selected_ws_conn);
            self.rules_table_state.select(self.selected_rule);

            terminal.draw(|f| ui::draw(f, self))?;

            if let Some(event) = events.next().await {
                match event {
                    Event::Key(key) => self.handle_key(key).await,
                    Event::Daemon(msg) => self.handle_daemon_message(msg),
                    Event::Resize => {} // ratatui handles automatically
                    Event::Tick => {
                        // Clear expired status messages
                        if let Some((_, time)) = &self.status_message {
                            if time.elapsed() > Duration::from_secs(3) {
                                self.status_message = None;
                            }
                        }
                    }
                }
            }
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        if let Some(conn) = self.conn.take() {
            conn.disconnect().await;
        }

        Ok(())
    }

    async fn connect_daemon(&mut self, event_tx: mpsc::UnboundedSender<Event>) {
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

    fn handle_daemon_message(&mut self, msg: DaemonMessage) {
        match msg {
            DaemonMessage::Event { data } => {
                if !self.paused {
                    self.transactions.push(data);
                    // Keep max 5000
                    if self.transactions.len() > 5000 {
                        self.transactions.drain(0..1000);
                        // Adjust selection index
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
            _ => {}
        }
    }

    pub(crate) fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), std::time::Instant::now()));
    }

    fn replay_selected_request(&mut self) {
        if let Some(idx) = self.selected_transaction {
            if let Some(info) = self.transactions.get(idx) {
                if let Some(req) = &info.0 {
                    let method = req.method().to_string();
                    let uri = req.uri().to_string();
                    let headers = req.headers().clone();
                    let body = req.body().cloned();
                    self.set_status(&format!("Replaying {} {}...", method, uri));
                    tokio::spawn(async move {
                        let Ok(client) = reqwest::Client::builder()
                            .danger_accept_invalid_certs(true)
                            .build()
                        else {
                            return;
                        };
                        let method: reqwest::Method =
                            method.parse().unwrap_or(reqwest::Method::GET);
                        let mut builder = client.request(method, &uri);
                        for (name, value) in headers.iter() {
                            if let Ok(v) = value.to_str() {
                                builder = builder.header(name.as_str(), v);
                            }
                        }
                        if let Some(body) = body {
                            if !body.is_empty() {
                                builder = builder.body(body);
                            }
                        }
                        let _ = builder.send().await;
                    });
                }
            }
        }
    }

    fn export_har(&mut self) {
        if self.transactions.is_empty() {
            self.set_status("No transactions to export");
            return;
        }

        match proxy_v2_models::har::build_har_json(&self.transactions) {
            Ok(json) => {
                let path = format!(
                    "cheolsu-proxy-{}.har",
                    chrono::Local::now().format("%Y%m%d-%H%M%S")
                );
                match std::fs::write(&path, json) {
                    Ok(_) => {
                        self.set_status(&format!("HAR exported: {}", path));
                    }
                    Err(e) => {
                        self.set_status(&format!("HAR export failed: {}", e));
                    }
                }
            }
            Err(e) => {
                self.set_status(&format!("HAR serialization failed: {}", e));
            }
        }
    }

    async fn send_script_load(&mut self, path: &str) {
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

    async fn send_script_unload(&mut self) {
        if let Some(conn) = &self.conn {
            let cmd = ClientCommand::UnloadScript;
            let _ = conn.send_command(&cmd).await;
            self.script_active = false;
            self.script_path = None;
            self.set_status("Script unloaded");
        }
    }

    async fn send_upstream_update(&self) {
        if let Some(conn) = &self.conn {
            let config = self.upstream_form.to_config();
            let cmd = ClientCommand::UpdateUpstreamProxy { config };
            let _ = conn.send_command(&cmd).await;
        }
    }

    async fn send_throttle_update(&mut self) {
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

    async fn send_rules_update(&self) {
        if let Some(conn) = &self.conn {
            let cmd = ClientCommand::UpdateInterceptRules {
                rules: self.rules.clone(),
            };
            let _ = conn.send_command(&cmd).await;
        }
    }

    pub(crate) fn run_diff(&mut self, idx_a: usize, idx_b: usize) {
        let (Some(txn_a), Some(txn_b)) =
            (self.transactions.get(idx_a), self.transactions.get(idx_b))
        else {
            self.set_status("Diff failed: transaction not found");
            return;
        };

        let request_diff = match (&txn_a.0, &txn_b.0) {
            (Some(req_a), Some(req_b)) => {
                let method_diff = if req_a.method() != req_b.method() {
                    Some((req_a.method().to_string(), req_b.method().to_string()))
                } else {
                    None
                };

                let url_diff = if req_a.uri().to_string() != req_b.uri().to_string() {
                    Some((req_a.uri().to_string(), req_b.uri().to_string()))
                } else {
                    None
                };

                let headers_a: Vec<(String, String)> = req_a
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                    .collect();
                let headers_b: Vec<(String, String)> = req_b
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                    .collect();
                let header_diffs = diff_headers(&headers_a, &headers_b);

                let body_diff = Self::compute_body_diff(
                    req_a.body().map(|b| b.as_ref()),
                    req_b.body().map(|b| b.as_ref()),
                    req_a.body_size(),
                    req_b.body_size(),
                    req_a.data_type(),
                    req_b.data_type(),
                );

                if method_diff.is_none()
                    && url_diff.is_none()
                    && header_diffs.is_empty()
                    && body_diff.is_none()
                {
                    None
                } else {
                    Some(TransactionPartDiff {
                        method_diff,
                        url_diff,
                        status_diff: None,
                        header_diffs,
                        body_diff,
                    })
                }
            }
            _ => None,
        };

        let response_diff = match (&txn_a.1, &txn_b.1) {
            (Some(res_a), Some(res_b)) => {
                let status_diff = if res_a.status() != res_b.status() {
                    Some((res_a.status().as_u16(), res_b.status().as_u16()))
                } else {
                    None
                };

                let headers_a: Vec<(String, String)> = res_a
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                    .collect();
                let headers_b: Vec<(String, String)> = res_b
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                    .collect();
                let header_diffs = diff_headers(&headers_a, &headers_b);

                let body_diff = Self::compute_body_diff(
                    res_a.body().map(|b| b.as_ref()),
                    res_b.body().map(|b| b.as_ref()),
                    res_a.body_size(),
                    res_b.body_size(),
                    res_a.data_type(),
                    res_b.data_type(),
                );

                if status_diff.is_none() && header_diffs.is_empty() && body_diff.is_none() {
                    None
                } else {
                    Some(TransactionPartDiff {
                        method_diff: None,
                        url_diff: None,
                        status_diff,
                        header_diffs,
                        body_diff,
                    })
                }
            }
            _ => None,
        };

        let diff = TrafficDiff {
            request_diff,
            response_diff,
        };

        self.diff_result = Some(format_diff_text(&diff));
        self.show_diff = true;
        self.diff_scroll = 0;
        self.diff_mark = None;
    }

    fn compute_body_diff(
        body_a: Option<&[u8]>,
        body_b: Option<&[u8]>,
        size_a: usize,
        size_b: usize,
        data_type_a: &proxy_v2_models::DataType,
        data_type_b: &proxy_v2_models::DataType,
    ) -> Option<BodyDiff> {
        let bytes_a = body_a.map(|b| b.to_vec()).unwrap_or_default();
        let bytes_b = body_b.map(|b| b.to_vec()).unwrap_or_default();

        if bytes_a == bytes_b {
            return None;
        }

        let is_json = matches!(
            data_type_a,
            proxy_v2_models::DataType::Json | proxy_v2_models::DataType::GraphQL
        ) && matches!(
            data_type_b,
            proxy_v2_models::DataType::Json | proxy_v2_models::DataType::GraphQL
        );

        if is_json {
            if let (Ok(text_a), Ok(text_b)) =
                (std::str::from_utf8(&bytes_a), std::str::from_utf8(&bytes_b))
            {
                if let (Ok(json_a), Ok(json_b)) = (
                    serde_json::from_str::<serde_json::Value>(text_a),
                    serde_json::from_str::<serde_json::Value>(text_b),
                ) {
                    return Some(diff_json(&json_a, &json_b));
                }
            }
        }

        let is_text = data_type_a.is_text_based() && data_type_b.is_text_based();
        if is_text {
            if let (Ok(text_a), Ok(text_b)) =
                (std::str::from_utf8(&bytes_a), std::str::from_utf8(&bytes_b))
            {
                return Some(diff_text(text_a, text_b));
            }
        }

        Some(BodyDiff::Binary {
            old_size: size_a,
            new_size: size_b,
        })
    }
}
