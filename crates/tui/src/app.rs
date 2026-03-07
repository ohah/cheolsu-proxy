use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use proxy_daemon::{
    ClientCommand, DaemonConnection, DaemonMessage, InterceptAction, InterceptRule,
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

    // Status message
    pub status_message: Option<(String, std::time::Instant)>,

    // Connection
    conn: Option<DaemonConnection>,
    event_tx: Option<mpsc::UnboundedSender<Event>>,
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
            Tab::Settings => {}
        }
    }

    async fn handle_network_key(&mut self, key: KeyEvent) {
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
            }
            KeyCode::Char(' ') => {
                self.paused = !self.paused;
            }
            KeyCode::Char('c') => {
                self.transactions.clear();
                self.selected_transaction = None;
            }
            KeyCode::Char('y') => {
                // Copy selected request URL to clipboard
                if let Some(idx) = self.selected_transaction {
                    if let Some(info) = self.transactions.get(idx) {
                        if let Some(req) = &info.0 {
                            let url = req.uri().to_string();
                            if copy_to_clipboard(&url) {
                                self.set_status("URL copied to clipboard");
                            }
                        }
                    }
                }
            }
            KeyCode::Char('Y') => {
                // Copy full request/response detail
                if let Some(idx) = self.selected_transaction {
                    if let Some(info) = self.transactions.get(idx) {
                        let detail = format_transaction_detail(info);
                        if copy_to_clipboard(&detail) {
                            self.set_status("Request/Response copied to clipboard");
                        }
                    }
                }
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

fn format_transaction_detail(info: &RequestInfo) -> String {
    let mut out = String::new();

    if let Some(req) = &info.0 {
        out.push_str(&format!("{} {}\n", req.method(), req.uri()));
        out.push_str(&format!("Version: {:?}\n", req.version()));
        for (name, value) in req.headers().iter() {
            out.push_str(&format!(
                "{}: {}\n",
                name,
                value.to_str().unwrap_or("<binary>")
            ));
        }
        out.push('\n');
    }

    if let Some(res) = &info.1 {
        out.push_str(&format!("Status: {}\n", res.status()));
        for (name, value) in res.headers().iter() {
            out.push_str(&format!(
                "{}: {}\n",
                name,
                value.to_str().unwrap_or("<binary>")
            ));
        }
    }

    out
}
