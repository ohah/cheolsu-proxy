mod ca_cert;
mod daemon;
mod diff;
mod forms;
mod key_handlers;
mod utils;

pub use forms::*;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use proxy_daemon::{
    BreakpointData, BreakpointPhase, BreakpointRule, DaemonConnection, HostMapping, InterceptRule,
    SslProxyingEntry, SslProxyingMode,
};
use proxy_v2_models::{RequestInfo, WsMessageInfo};
use ratatui::prelude::*;
use ratatui::widgets::TableState;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::event::{Event, EventHandler};
use crate::tabs::Tab;
use crate::ui;

/// 대기 중인 breakpoint 엔트리 (TUI 표시용)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PendingBreakpointEntry {
    pub id: String,
    pub transaction_id: String,
    pub phase: BreakpointPhase,
    pub data: BreakpointData,
}

/// Breakpoint 탭 포커스 영역
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointFocus {
    Rules,
    Pending,
}

/// Breakpoint 규칙 추가 폼
#[derive(Debug, Clone)]
pub struct BreakpointAddForm {
    pub pattern: String,
    pub break_on_request: bool,
    pub break_on_response: bool,
    pub field: BreakpointFormField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointFormField {
    Pattern,
    BreakOnRequest,
    BreakOnResponse,
}

impl BreakpointAddForm {
    pub fn new() -> Self {
        Self {
            pattern: String::new(),
            break_on_request: true,
            break_on_response: false,
            field: BreakpointFormField::Pattern,
        }
    }
}

/// 로그 파일 엔트리 (TUI 표시용)
#[derive(Debug, Clone)]
pub struct LogFileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
}

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

    // Breakpoint
    pub breakpoint_rules: Vec<BreakpointRule>,
    pub pending_breakpoints: Vec<PendingBreakpointEntry>,
    pub selected_bp_rule: Option<usize>,
    pub selected_pending_bp: Option<usize>,
    pub bp_rules_table_state: TableState,
    pub bp_pending_table_state: TableState,
    pub bp_focus: BreakpointFocus,
    pub bp_add_form: Option<BreakpointAddForm>,

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

    // Proxy Authentication
    pub proxy_auth_form: ProxyAuthForm,

    // Throttle
    pub throttle_form: ThrottleForm,

    // Quick Settings
    pub quick_settings_form: QuickSettingsForm,

    // Client Certificate (mTLS)
    pub client_cert_form: ClientCertForm,

    // Host Mapping
    pub host_mappings: Vec<HostMapping>,
    pub selected_host_mapping: Option<usize>,
    pub host_mapping_form: Option<HostMappingForm>,
    pub host_mapping_table_state: TableState,

    // SSL Proxying
    pub ssl_proxying_mode: SslProxyingMode,
    pub ssl_proxying_entries: Vec<SslProxyingEntry>,
    pub selected_ssl_proxying: Option<usize>,
    pub ssl_proxying_add_form: Option<SslProxyingAddForm>,
    pub ssl_proxying_table_state: TableState,

    // CA Certificate
    pub ca_cert_installed: bool,
    pub ca_cert_path: Option<String>,

    // Remote device cert info
    pub local_ips: Vec<String>,

    // Session save/load
    pub session_save_editing: bool,
    pub session_save_path_input: String,
    pub session_load_editing: bool,
    pub session_load_path_input: String,

    // Logs viewer
    pub log_files: Vec<LogFileEntry>,
    pub selected_log_file: Option<usize>,
    pub log_content_lines: Vec<String>,
    pub log_scroll: usize,
    pub log_filter: String,
    pub log_filter_editing: bool,
    pub log_last_refresh: Option<std::time::Instant>,

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
            breakpoint_rules: Vec::new(),
            pending_breakpoints: Vec::new(),
            selected_bp_rule: None,
            selected_pending_bp: None,
            bp_rules_table_state: TableState::default(),
            bp_pending_table_state: TableState::default(),
            bp_focus: BreakpointFocus::Rules,
            bp_add_form: None,
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
            proxy_auth_form: ProxyAuthForm::new(),
            throttle_form: ThrottleForm::new(),
            quick_settings_form: QuickSettingsForm::new(),
            client_cert_form: ClientCertForm::new(),
            host_mappings: Vec::new(),
            selected_host_mapping: None,
            host_mapping_form: None,
            host_mapping_table_state: TableState::default(),
            ssl_proxying_mode: SslProxyingMode::default(),
            ssl_proxying_entries: Vec::new(),
            selected_ssl_proxying: None,
            ssl_proxying_add_form: None,
            ssl_proxying_table_state: TableState::default(),
            ca_cert_installed: false,
            ca_cert_path: None,
            local_ips: proxy_daemon::get_local_ips(),
            session_save_editing: false,
            session_save_path_input: String::new(),
            session_load_editing: false,
            session_load_path_input: String::new(),
            log_files: Vec::new(),
            selected_log_file: None,
            log_content_lines: Vec::new(),
            log_scroll: 0,
            log_filter: String::new(),
            log_filter_editing: false,
            log_last_refresh: None,
            status_message: None,
            conn: None,
            event_tx: None,
        }
    }

    /// 로그 파일 목록을 새로고침
    pub fn refresh_log_files(&mut self) {
        let mut files = Vec::new();

        if let Ok(support_dir) = proxy_daemon::daemon::app_support_dir() {
            // daemon.log
            let daemon_log = support_dir.join("daemon.log");
            if daemon_log.exists() {
                if let Ok(meta) = std::fs::metadata(&daemon_log) {
                    files.push(LogFileEntry {
                        name: "daemon.log".to_string(),
                        path: daemon_log.display().to_string(),
                        size: meta.len(),
                    });
                }
            }

            // logs/ 디렉토리 내 로그 파일들
            let log_dir = support_dir.join("logs");
            if log_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&log_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map(|e| e == "log").unwrap_or(false) {
                            if let Ok(meta) = entry.metadata() {
                                files.push(LogFileEntry {
                                    name: entry.file_name().to_string_lossy().to_string(),
                                    path: path.display().to_string(),
                                    size: meta.len(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // 이름 역순 정렬 (최신 파일 먼저)
        files.sort_by(|a, b| b.name.cmp(&a.name));
        self.log_files = files;

        // 선택된 파일이 없으면 첫 번째 선택
        if self.selected_log_file.is_none() && !self.log_files.is_empty() {
            self.selected_log_file = Some(0);
        }

        self.log_last_refresh = Some(std::time::Instant::now());
    }

    /// 선택된 로그 파일 내용 읽기
    pub fn refresh_log_content(&mut self) {
        if let Some(idx) = self.selected_log_file {
            if let Some(file) = self.log_files.get(idx) {
                if let Ok(content) = std::fs::read_to_string(&file.path) {
                    let lines: Vec<String> = content.lines().map(String::from).collect();
                    // 마지막 1000줄만 유지
                    let start = lines.len().saturating_sub(1000);
                    self.log_content_lines = lines[start..].to_vec();
                    // 자동 스크롤
                    self.log_scroll = self.log_content_lines.len().saturating_sub(1);
                } else {
                    self.log_content_lines = Vec::new();
                    self.log_scroll = 0;
                }
            }
        }
    }

    /// 선택된 로그 파일 초기화
    pub fn clear_selected_log(&mut self) {
        if let Some(idx) = self.selected_log_file {
            if let Some(file) = self.log_files.get(idx) {
                let _ = std::fs::write(&file.path, "");
                self.log_content_lines.clear();
                self.log_scroll = 0;
            }
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
            self.bp_rules_table_state.select(self.selected_bp_rule);
            self.bp_pending_table_state.select(self.selected_pending_bp);
            self.host_mapping_table_state
                .select(self.selected_host_mapping);
            self.ssl_proxying_table_state
                .select(self.selected_ssl_proxying);

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
                        // Logs 탭: 5초마다 자동 새로고침
                        if self.tab == Tab::Logs {
                            let should_refresh = self
                                .log_last_refresh
                                .map(|t| t.elapsed() > Duration::from_secs(5))
                                .unwrap_or(true);
                            if should_refresh {
                                self.refresh_log_files();
                                self.refresh_log_content();
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

    pub(crate) fn save_session(&mut self) {
        if self.transactions.is_empty() {
            self.set_status("No transactions to save");
            return;
        }

        let path = if self.session_save_path_input.is_empty() {
            format!(
                "cheolsu-session-{}.cheolsu",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            )
        } else {
            proxy_daemon::ensure_extension(&self.session_save_path_input)
        };

        let session = proxy_daemon::SessionFile::from_traffic(
            self.port,
            &self.transactions,
            &self.ws_messages,
            &self.rules,
            &[],
            None,
        );

        match session.save(std::path::Path::new(&path)) {
            Ok(_) => {
                self.set_status(&format!(
                    "Session saved: {} ({} transactions)",
                    path,
                    self.transactions.len()
                ));
            }
            Err(e) => {
                self.set_status(&format!("Session save failed: {}", e));
            }
        }
    }

    pub(crate) fn load_session(&mut self) {
        let path = &self.session_load_path_input;
        if path.is_empty() {
            self.set_status("No path specified");
            return;
        }

        match proxy_daemon::SessionFile::load(std::path::Path::new(path)) {
            Ok(session) => {
                let tx_count = session.transactions.len();
                let loaded_transactions = session.extract_transactions();
                self.transactions = loaded_transactions;
                self.selected_transaction = None;

                // intercept rules도 복원
                if !session.intercept_rules.is_empty() {
                    self.rules = session.intercept_rules;
                }

                // WebSocket 메시지 복원
                if !session.websocket_messages.is_empty() {
                    self.ws_messages = session.websocket_messages;
                }

                self.set_status(&format!(
                    "Session loaded: {} ({} transactions)",
                    path, tx_count
                ));
            }
            Err(e) => {
                self.set_status(&format!("Session load failed: {}", e));
            }
        }
    }
}
