use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use proxy_daemon::{
    ClientCommand, DaemonConnection, DaemonMessage, InterceptAction, InterceptRule,
    UpstreamProxyAuth, UpstreamProxyConfig,
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

    // Upstream Proxy
    pub upstream_form: UpstreamProxyForm,

    // Status message
    pub status_message: Option<(String, std::time::Instant)>,

    // Connection
    conn: Option<DaemonConnection>,
    event_tx: Option<mpsc::UnboundedSender<Event>>,
}

/// 스크립트 로그 엔트리 (TUI 표시용)
#[derive(Debug, Clone)]
pub struct ScriptLogEntry {
    pub level: String,
    pub message: String,
    pub time: std::time::Instant,
}

/// Upstream proxy settings form
#[derive(Debug, Clone)]
pub struct UpstreamProxyForm {
    pub enabled: bool,
    pub field: UpstreamProxyField,
    pub editing: bool,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub bypass: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamProxyField {
    Enabled,
    Host,
    Port,
    Username,
    Password,
    Bypass,
}

impl UpstreamProxyField {
    pub const ALL: [UpstreamProxyField; 6] = [
        Self::Enabled,
        Self::Host,
        Self::Port,
        Self::Username,
        Self::Password,
        Self::Bypass,
    ];

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|f| f == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(&self) -> Self {
        let idx = Self::ALL.iter().position(|f| f == self).unwrap_or(0);
        if idx == 0 {
            Self::ALL[Self::ALL.len() - 1]
        } else {
            Self::ALL[idx - 1]
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Enabled => "Enabled",
            Self::Host => "Host",
            Self::Port => "Port",
            Self::Username => "Username",
            Self::Password => "Password",
            Self::Bypass => "Bypass",
        }
    }
}

impl UpstreamProxyForm {
    pub fn new() -> Self {
        Self {
            enabled: false,
            field: UpstreamProxyField::Enabled,
            editing: false,
            host: String::new(),
            port: "8080".to_string(),
            username: String::new(),
            password: String::new(),
            bypass: "localhost".to_string(),
        }
    }

    pub fn to_config(&self) -> Option<UpstreamProxyConfig> {
        if !self.enabled || self.host.is_empty() {
            return None;
        }
        let auth = if !self.username.is_empty() {
            Some(UpstreamProxyAuth {
                username: self.username.clone(),
                password: self.password.clone(),
            })
        } else {
            None
        };
        let bypass = self
            .bypass
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Some(UpstreamProxyConfig {
            host: self.host.clone(),
            port: self.port.parse().unwrap_or(8080),
            auth,
            bypass,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WsConnection {
    pub connection_id: String,
    pub uri: String,
    pub time: i64,
    pub active: bool,
}

/// Rule creation form
#[derive(Debug, Clone)]
pub struct RuleForm {
    pub field: RuleFormField,
    pub name: String,
    pub pattern: String,
    pub method: Option<String>,
    pub action_type: ActionType,
    pub status_code: String,
    pub body: String,
    pub target_url: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleFormField {
    Name,
    Pattern,
    Method,
    ActionType,
    StatusCode,
    Body,
    TargetUrl,
    FilePath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    Block,
    ModifyRequest,
    ModifyResponse,
    MapLocal,
    MapRemote,
}

impl ActionType {
    pub const ALL: [ActionType; 5] = [
        ActionType::Block,
        ActionType::ModifyRequest,
        ActionType::ModifyResponse,
        ActionType::MapLocal,
        ActionType::MapRemote,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ActionType::Block => "Block",
            ActionType::ModifyRequest => "Modify Request",
            ActionType::ModifyResponse => "Modify Response",
            ActionType::MapLocal => "Map Local",
            ActionType::MapRemote => "Map Remote",
        }
    }

    pub fn next(&self) -> ActionType {
        let idx = Self::ALL.iter().position(|a| a == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(&self) -> ActionType {
        let idx = Self::ALL.iter().position(|a| a == self).unwrap_or(0);
        if idx == 0 {
            Self::ALL[Self::ALL.len() - 1]
        } else {
            Self::ALL[idx - 1]
        }
    }
}

impl RuleFormField {
    fn next(&self, action_type: ActionType) -> RuleFormField {
        match self {
            RuleFormField::Name => RuleFormField::Pattern,
            RuleFormField::Pattern => RuleFormField::Method,
            RuleFormField::Method => RuleFormField::ActionType,
            RuleFormField::ActionType => match action_type {
                ActionType::Block => RuleFormField::StatusCode,
                ActionType::ModifyRequest | ActionType::ModifyResponse => RuleFormField::Body,
                ActionType::MapLocal => RuleFormField::FilePath,
                ActionType::MapRemote => RuleFormField::TargetUrl,
            },
            RuleFormField::StatusCode => RuleFormField::Body,
            RuleFormField::Body => RuleFormField::Name,
            RuleFormField::TargetUrl => RuleFormField::Name,
            RuleFormField::FilePath => RuleFormField::StatusCode,
        }
    }

    fn prev(&self, action_type: ActionType) -> RuleFormField {
        match self {
            RuleFormField::Name => match action_type {
                ActionType::Block | ActionType::ModifyResponse => RuleFormField::Body,
                ActionType::ModifyRequest => RuleFormField::Body,
                ActionType::MapLocal => RuleFormField::StatusCode,
                ActionType::MapRemote => RuleFormField::TargetUrl,
            },
            RuleFormField::Pattern => RuleFormField::Name,
            RuleFormField::Method => RuleFormField::Pattern,
            RuleFormField::ActionType => RuleFormField::Method,
            RuleFormField::StatusCode => match action_type {
                ActionType::MapLocal => RuleFormField::FilePath,
                _ => RuleFormField::ActionType,
            },
            RuleFormField::Body => match action_type {
                ActionType::Block => RuleFormField::StatusCode,
                _ => RuleFormField::ActionType,
            },
            RuleFormField::TargetUrl => RuleFormField::ActionType,
            RuleFormField::FilePath => RuleFormField::ActionType,
        }
    }
}

impl RuleForm {
    fn new() -> Self {
        Self {
            field: RuleFormField::Name,
            name: String::new(),
            pattern: String::new(),
            method: None,
            action_type: ActionType::Block,
            status_code: "403".to_string(),
            body: String::new(),
            target_url: String::new(),
            file_path: String::new(),
        }
    }

    fn to_rule(&self) -> Option<InterceptRule> {
        if self.pattern.is_empty() {
            return None;
        }

        let action = match self.action_type {
            ActionType::Block => InterceptAction::Block {
                status_code: self.status_code.parse().unwrap_or(403),
                body: self.body.clone(),
            },
            ActionType::ModifyRequest => InterceptAction::ModifyRequest {
                add_headers: std::collections::HashMap::new(),
                remove_headers: Vec::new(),
                set_body: if self.body.is_empty() {
                    None
                } else {
                    Some(self.body.clone())
                },
            },
            ActionType::ModifyResponse => InterceptAction::ModifyResponse {
                set_status: self.status_code.parse().ok(),
                add_headers: std::collections::HashMap::new(),
                remove_headers: Vec::new(),
                set_body: if self.body.is_empty() {
                    None
                } else {
                    Some(self.body.clone())
                },
            },
            ActionType::MapLocal => InterceptAction::MapLocal {
                file_path: self.file_path.clone(),
                status_code: self.status_code.parse().unwrap_or(200),
                headers: std::collections::HashMap::new(),
            },
            ActionType::MapRemote => InterceptAction::MapRemote {
                target_url: self.target_url.clone(),
                preserve_path: true,
            },
        };

        Some(InterceptRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: if self.name.is_empty() {
                self.pattern.clone()
            } else {
                self.name.clone()
            },
            enabled: true,
            pattern: self.pattern.clone(),
            method: self.method.clone(),
            action,
        })
    }
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
            upstream_form: UpstreamProxyForm::new(),
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

    fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), std::time::Instant::now()));
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        // If rule form is open, handle form input
        if self.rule_form.is_some() {
            self.handle_rule_form_key(key).await;
            return;
        }

        // If script path is in editing mode, handle it
        if self.tab == Tab::Script && self.script_editing {
            self.handle_script_key(key).await;
            return;
        }

        // If upstream proxy form is in editing mode, handle it
        if self.tab == Tab::Settings && self.upstream_form.editing {
            self.handle_settings_key(key).await;
            return;
        }

        // Global keys: q/Ctrl+c to quit
        if key.code == KeyCode::Char('q')
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.running = false;
            return;
        }

        // Tab switching
        match key.code {
            KeyCode::Tab => {
                self.tab = self.tab.next();
                return;
            }
            KeyCode::BackTab => {
                self.tab = self.tab.prev();
                return;
            }
            KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.tab = Tab::Network;
                return;
            }
            KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.tab = Tab::WebSocket;
                return;
            }
            KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.tab = Tab::InterceptRules;
                return;
            }
            KeyCode::Char('4') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.tab = Tab::Script;
                return;
            }
            KeyCode::Char('5') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.tab = Tab::Settings;
                return;
            }
            _ => {}
        }

        // Tab-specific keys
        match self.tab {
            Tab::Network => self.handle_network_key(key).await,
            Tab::WebSocket => self.handle_ws_key(key),
            Tab::InterceptRules => self.handle_rules_key(key).await,
            Tab::Script => self.handle_script_key(key).await,
            Tab::Settings => self.handle_settings_key(key).await,
        }
    }

    async fn handle_network_key(&mut self, key: KeyEvent) {
        // Detail view: Esc or Enter to go back, j/k to scroll
        if self.show_detail {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.show_detail = false;
                    self.detail_scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.detail_scroll = 0;
                }
                KeyCode::Char('y') => {
                    // Copy URL
                    if let Some(idx) = self.selected_transaction {
                        if let Some(info) = self.transactions.get(idx) {
                            if let Some(req) = &info.0 {
                                if copy_to_clipboard(&req.uri().to_string()) {
                                    self.set_status("URL copied to clipboard");
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('c') => {
                    // Copy as cURL
                    if let Some(idx) = self.selected_transaction {
                        if let Some(info) = self.transactions.get(idx) {
                            let curl = format_curl_command(info);
                            if copy_to_clipboard(&curl) {
                                self.set_status("cURL command copied to clipboard");
                            }
                        }
                    }
                }
                KeyCode::Char('r') => {
                    // Replay request
                    self.replay_selected_request();
                }
                _ => {}
            }
            return;
        }

        let len = self.transactions.len();
        if len == 0 {
            return;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut idx) = self.selected_transaction {
                    *idx = idx.saturating_sub(1);
                } else {
                    self.selected_transaction = Some(len.saturating_sub(1));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut idx) = self.selected_transaction {
                    if *idx + 1 < len {
                        *idx += 1;
                    }
                } else {
                    self.selected_transaction = Some(0);
                }
            }
            KeyCode::Enter => {
                self.show_detail = !self.show_detail;
                self.detail_scroll = 0;
            }
            KeyCode::Char(' ') => {
                self.paused = !self.paused;
            }
            KeyCode::Char('x') => {
                self.transactions.clear();
                self.selected_transaction = None;
                self.set_status("All requests cleared");
            }
            KeyCode::Char('y') => {
                // Copy selected request URL to clipboard
                if let Some(idx) = self.selected_transaction {
                    if let Some(info) = self.transactions.get(idx) {
                        if let Some(req) = &info.0 {
                            if copy_to_clipboard(&req.uri().to_string()) {
                                self.set_status("URL copied to clipboard");
                            }
                        }
                    }
                }
            }
            KeyCode::Char('c') => {
                // Copy as cURL
                if let Some(idx) = self.selected_transaction {
                    if let Some(info) = self.transactions.get(idx) {
                        let curl = format_curl_command(info);
                        if copy_to_clipboard(&curl) {
                            self.set_status("cURL command copied to clipboard");
                        }
                    }
                }
            }
            KeyCode::Char('r') => {
                self.replay_selected_request();
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected_transaction = Some(0);
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.selected_transaction = Some(len.saturating_sub(1));
            }
            _ => {}
        }
    }

    fn handle_ws_key(&mut self, key: KeyEvent) {
        let len = self.ws_connections.len();
        if len == 0 {
            return;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut idx) = self.selected_ws_conn {
                    *idx = idx.saturating_sub(1);
                } else {
                    self.selected_ws_conn = Some(len.saturating_sub(1));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut idx) = self.selected_ws_conn {
                    if *idx + 1 < len {
                        *idx += 1;
                    }
                } else {
                    self.selected_ws_conn = Some(0);
                }
            }
            KeyCode::Char('c') => {
                self.ws_connections.clear();
                self.ws_messages.clear();
                self.selected_ws_conn = None;
            }
            KeyCode::Char('y') => {
                // Copy selected connection URI to clipboard
                if let Some(idx) = self.selected_ws_conn {
                    if let Some(conn) = self.ws_connections.get(idx) {
                        let uri = conn.uri.clone();
                        if copy_to_clipboard(&uri) {
                            self.set_status("WebSocket URI copied to clipboard");
                        }
                    }
                }
            }
            KeyCode::Char('Y') => {
                // Copy all messages for selected connection
                if let Some(idx) = self.selected_ws_conn {
                    if let Some(conn) = self.ws_connections.get(idx) {
                        let conn_id = &conn.connection_id;
                        let detail = format_ws_messages(conn_id, &conn.uri, &self.ws_messages);
                        if copy_to_clipboard(&detail) {
                            self.set_status("WebSocket messages copied to clipboard");
                        }
                    }
                }
            }
            _ => {}
        }
    }

    async fn handle_rules_key(&mut self, key: KeyEvent) {
        let len = self.rules.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                if let Some(ref mut idx) = self.selected_rule {
                    *idx = idx.saturating_sub(1);
                } else {
                    self.selected_rule = Some(len.saturating_sub(1));
                }
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                if let Some(ref mut idx) = self.selected_rule {
                    if *idx + 1 < len {
                        *idx += 1;
                    }
                } else {
                    self.selected_rule = Some(0);
                }
            }
            KeyCode::Char('a') => {
                // Open add rule form
                self.rule_form = Some(RuleForm::new());
            }
            KeyCode::Char('t') => {
                // Toggle enabled/disabled
                if let Some(idx) = self.selected_rule {
                    if idx < self.rules.len() {
                        self.rules[idx].enabled = !self.rules[idx].enabled;
                        self.send_rules_update().await;
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                // Delete rule
                if let Some(idx) = self.selected_rule {
                    if idx < self.rules.len() {
                        self.rules.remove(idx);
                        if self.rules.is_empty() {
                            self.selected_rule = None;
                        } else if idx >= self.rules.len() {
                            self.selected_rule = Some(self.rules.len() - 1);
                        }
                        self.send_rules_update().await;
                    }
                }
            }
            KeyCode::Char('C') => {
                // Clear all rules
                self.rules.clear();
                self.selected_rule = None;
                self.send_rules_update().await;
            }
            _ => {}
        }
    }

    async fn handle_rule_form_key(&mut self, key: KeyEvent) {
        let form = self.rule_form.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                self.rule_form = None;
            }
            KeyCode::Tab => {
                form.field = form.field.next(form.action_type);
            }
            KeyCode::BackTab => {
                form.field = form.field.prev(form.action_type);
            }
            KeyCode::Enter => {
                // Submit form
                if let Some(rule) = form.to_rule() {
                    self.rules.push(rule);
                    self.send_rules_update().await;
                    self.set_status("Rule added");
                    self.rule_form = None;
                } else {
                    self.set_status("Pattern is required");
                }
            }
            _ => {
                // Handle text input for current field
                match form.field {
                    RuleFormField::ActionType => match key.code {
                        KeyCode::Left => form.action_type = form.action_type.prev(),
                        KeyCode::Right => form.action_type = form.action_type.next(),
                        _ => {}
                    },
                    RuleFormField::Method => match key.code {
                        KeyCode::Left | KeyCode::Right => {
                            let methods = [
                                None,
                                Some("GET"),
                                Some("POST"),
                                Some("PUT"),
                                Some("DELETE"),
                                Some("PATCH"),
                            ];
                            let cur = methods
                                .iter()
                                .position(|m| *m == form.method.as_deref())
                                .unwrap_or(0);
                            let next = if key.code == KeyCode::Right {
                                (cur + 1) % methods.len()
                            } else if cur == 0 {
                                methods.len() - 1
                            } else {
                                cur - 1
                            };
                            form.method = methods[next].map(|s| s.to_string());
                        }
                        _ => {}
                    },
                    _ => {
                        let field = match form.field {
                            RuleFormField::Name => &mut form.name,
                            RuleFormField::Pattern => &mut form.pattern,
                            RuleFormField::StatusCode => &mut form.status_code,
                            RuleFormField::Body => &mut form.body,
                            RuleFormField::TargetUrl => &mut form.target_url,
                            RuleFormField::FilePath => &mut form.file_path,
                            _ => return,
                        };
                        match key.code {
                            KeyCode::Char(c) => field.push(c),
                            KeyCode::Backspace => {
                                field.pop();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
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
                        let client = reqwest::Client::builder()
                            .danger_accept_invalid_certs(true)
                            .build()
                            .unwrap();
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

    async fn handle_settings_key(&mut self, key: KeyEvent) {
        if self.upstream_form.editing {
            // Text input mode
            match key.code {
                KeyCode::Esc => {
                    self.upstream_form.editing = false;
                }
                KeyCode::Enter => {
                    self.upstream_form.editing = false;
                    self.send_upstream_update().await;
                }
                KeyCode::Char(c) => {
                    let field = match self.upstream_form.field {
                        UpstreamProxyField::Host => &mut self.upstream_form.host,
                        UpstreamProxyField::Port => &mut self.upstream_form.port,
                        UpstreamProxyField::Username => &mut self.upstream_form.username,
                        UpstreamProxyField::Password => &mut self.upstream_form.password,
                        UpstreamProxyField::Bypass => &mut self.upstream_form.bypass,
                        _ => return,
                    };
                    field.push(c);
                }
                KeyCode::Backspace => {
                    let field = match self.upstream_form.field {
                        UpstreamProxyField::Host => &mut self.upstream_form.host,
                        UpstreamProxyField::Port => &mut self.upstream_form.port,
                        UpstreamProxyField::Username => &mut self.upstream_form.username,
                        UpstreamProxyField::Password => &mut self.upstream_form.password,
                        UpstreamProxyField::Bypass => &mut self.upstream_form.bypass,
                        _ => return,
                    };
                    field.pop();
                }
                _ => {}
            }
            return;
        }

        // Navigation mode
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.upstream_form.field = self.upstream_form.field.prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.upstream_form.field = self.upstream_form.field.next();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.upstream_form.field == UpstreamProxyField::Enabled {
                    self.upstream_form.enabled = !self.upstream_form.enabled;
                    self.send_upstream_update().await;
                } else {
                    self.upstream_form.editing = true;
                }
            }
            _ => {}
        }
    }

    async fn handle_script_key(&mut self, key: KeyEvent) {
        if self.script_editing {
            // 파일 경로 입력 모드
            match key.code {
                KeyCode::Esc => {
                    self.script_editing = false;
                }
                KeyCode::Enter => {
                    self.script_editing = false;
                    let path = self.script_path_input.clone();
                    if !path.is_empty() {
                        self.send_script_load(&path).await;
                    }
                }
                KeyCode::Char(c) => {
                    self.script_path_input.push(c);
                }
                KeyCode::Backspace => {
                    self.script_path_input.pop();
                }
                _ => {}
            }
            return;
        }

        // 일반 모드
        match key.code {
            KeyCode::Char('l') => {
                // 스크립트 파일 로드 (경로 입력 모드 진입)
                self.script_editing = true;
            }
            KeyCode::Char('u') => {
                // 스크립트 언로드
                if self.script_active {
                    self.send_script_unload().await;
                }
            }
            KeyCode::Char('r') => {
                // 스크립트 리로드
                if let Some(path) = self.script_path.clone() {
                    self.send_script_load(&path).await;
                }
            }
            KeyCode::Char('c') => {
                // 로그 초기화
                self.script_logs.clear();
                self.script_log_scroll = 0;
                self.set_status("Script logs cleared");
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.script_log_scroll = self.script_log_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.script_log_scroll + 1 < self.script_logs.len() {
                    self.script_log_scroll += 1;
                }
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.script_log_scroll = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.script_log_scroll = self.script_logs.len().saturating_sub(1);
            }
            _ => {}
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

    async fn send_rules_update(&self) {
        if let Some(conn) = &self.conn {
            let cmd = ClientCommand::UpdateInterceptRules {
                rules: self.rules.clone(),
            };
            let _ = conn.send_command(&cmd).await;
        }
    }
}

fn copy_to_clipboard(text: &str) -> bool {
    use std::process::{Command, Stdio};

    // macOS
    if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        return child.wait().map(|s| s.success()).unwrap_or(false);
    }

    // Linux (xclip)
    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        return child.wait().map(|s| s.success()).unwrap_or(false);
    }

    false
}

fn format_ws_messages(conn_id: &str, uri: &str, messages: &[WsMessageInfo]) -> String {
    let mut out = format!("WebSocket: {}\n\n", uri);
    for msg in messages.iter().filter(|m| m.connection_id == conn_id) {
        let dir = match msg.direction {
            proxy_v2_models::WsDirection::ClientToServer => "->",
            proxy_v2_models::WsDirection::ServerToClient => "<-",
        };
        let msg_type = match msg.message_type {
            proxy_v2_models::WsMessageType::Text => "TXT",
            proxy_v2_models::WsMessageType::Binary => "BIN",
            proxy_v2_models::WsMessageType::Ping => "PING",
            proxy_v2_models::WsMessageType::Pong => "PONG",
            proxy_v2_models::WsMessageType::Close => "CLOSE",
        };
        out.push_str(&format!(
            "[{}] {} {} {}B\n{}\n\n",
            dir, msg_type, msg.size, msg.size, msg.payload
        ));
    }
    out
}

fn format_curl_command(info: &RequestInfo) -> String {
    let Some(req) = &info.0 else {
        return String::new();
    };

    let mut parts = vec![format!("curl -X {} '{}'", req.method(), req.uri())];

    for (name, value) in req.headers().iter() {
        let name_str = name.as_str();
        // host, content-length 등은 curl이 자동으로 설정
        if name_str == "host" || name_str == "content-length" {
            continue;
        }
        if let Ok(v) = value.to_str() {
            parts.push(format!("  -H '{}: {}'", name_str, v.replace('\'', "'\\''")));
        }
    }

    if let Some(body) = req.body() {
        if !body.is_empty() {
            if let Ok(body_str) = std::str::from_utf8(body) {
                parts.push(format!("  -d '{}'", body_str.replace('\'', "'\\''")));
            } else {
                parts.push("  --data-binary @-".to_string());
            }
        }
    }

    parts.join(" \\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- UpstreamProxyField --

    #[test]
    fn field_next_wraps_around() {
        assert_eq!(UpstreamProxyField::Enabled.next(), UpstreamProxyField::Host);
        assert_eq!(
            UpstreamProxyField::Bypass.next(),
            UpstreamProxyField::Enabled
        );
    }

    #[test]
    fn field_prev_wraps_around() {
        assert_eq!(
            UpstreamProxyField::Enabled.prev(),
            UpstreamProxyField::Bypass
        );
        assert_eq!(UpstreamProxyField::Host.prev(), UpstreamProxyField::Enabled);
    }

    #[test]
    fn field_next_prev_full_cycle() {
        let mut field = UpstreamProxyField::Enabled;
        for _ in 0..UpstreamProxyField::ALL.len() {
            field = field.next();
        }
        assert_eq!(field, UpstreamProxyField::Enabled);

        for _ in 0..UpstreamProxyField::ALL.len() {
            field = field.prev();
        }
        assert_eq!(field, UpstreamProxyField::Enabled);
    }

    #[test]
    fn field_labels_not_empty() {
        for field in UpstreamProxyField::ALL {
            assert!(!field.label().is_empty());
        }
    }

    // -- UpstreamProxyForm defaults --

    #[test]
    fn form_new_defaults() {
        let form = UpstreamProxyForm::new();
        assert!(!form.enabled);
        assert_eq!(form.field, UpstreamProxyField::Enabled);
        assert!(!form.editing);
        assert!(form.host.is_empty());
        assert_eq!(form.port, "8080");
        assert!(form.username.is_empty());
        assert!(form.password.is_empty());
        assert_eq!(form.bypass, "localhost");
    }

    // -- to_config --

    #[test]
    fn to_config_returns_none_when_disabled() {
        let mut form = UpstreamProxyForm::new();
        form.host = "proxy.example.com".to_string();
        assert!(form.to_config().is_none());
    }

    #[test]
    fn to_config_returns_none_when_host_empty() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        assert!(form.to_config().is_none());
    }

    #[test]
    fn to_config_basic() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.port = "3128".to_string();

        let config = form.to_config().unwrap();
        assert_eq!(config.host, "proxy.example.com");
        assert_eq!(config.port, 3128);
        assert!(config.auth.is_none());
        assert_eq!(config.bypass, vec!["localhost"]);
    }

    #[test]
    fn to_config_with_auth() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.username = "user".to_string();
        form.password = "pass".to_string();

        let config = form.to_config().unwrap();
        let auth = config.auth.unwrap();
        assert_eq!(auth.username, "user");
        assert_eq!(auth.password, "pass");
    }

    #[test]
    fn to_config_no_auth_when_username_empty() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.password = "pass".to_string();

        let config = form.to_config().unwrap();
        assert!(config.auth.is_none());
    }

    #[test]
    fn to_config_bypass_parsing() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.bypass = "localhost, *.internal.com, 10.0.0.1".to_string();

        let config = form.to_config().unwrap();
        assert_eq!(
            config.bypass,
            vec!["localhost", "*.internal.com", "10.0.0.1"]
        );
    }

    #[test]
    fn to_config_bypass_empty_string() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.bypass = "".to_string();

        let config = form.to_config().unwrap();
        assert!(config.bypass.is_empty());
    }

    #[test]
    fn to_config_bypass_trims_whitespace() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.bypass = "  localhost ,  *.test.com  ".to_string();

        let config = form.to_config().unwrap();
        assert_eq!(config.bypass, vec!["localhost", "*.test.com"]);
    }

    #[test]
    fn to_config_invalid_port_defaults_to_8080() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.port = "not_a_number".to_string();

        let config = form.to_config().unwrap();
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn to_config_bypass_filters_empty_entries() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.bypass = "localhost,,, *.test.com, ,".to_string();

        let config = form.to_config().unwrap();
        assert_eq!(config.bypass, vec!["localhost", "*.test.com"]);
    }
}
