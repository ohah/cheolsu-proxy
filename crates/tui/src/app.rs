use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use proxy_daemon::{ClientCommand, DaemonConnection, DaemonMessage, InterceptRule};
use proxy_v2_models::{RequestInfo, WsConnectionEvent, WsMessageInfo};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::event::{Event, EventHandler};
use crate::tabs::Tab;
use crate::ui;

/// TUI 앱 상태
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
            conn: None,
            event_tx: None,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 터미널 초기화
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // 이벤트 핸들러
        let (mut events, event_tx) = EventHandler::new(Duration::from_millis(250));
        self.event_tx = Some(event_tx.clone());

        // 데몬 연결
        self.connect_daemon(event_tx.clone()).await;

        // 메인 루프
        while self.running {
            terminal.draw(|f| ui::draw(f, self))?;

            if let Some(event) = events.next().await {
                match event {
                    Event::Key(key) => self.handle_key(key).await,
                    Event::Daemon(msg) => self.handle_daemon_message(msg),
                    Event::Resize => {} // ratatui가 자동 처리
                    Event::Tick => {}
                }
            }
        }

        // 정리
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
            Err(_e) => {
                self.connected = false;
            }
        }
    }

    fn handle_daemon_message(&mut self, msg: DaemonMessage) {
        match msg {
            DaemonMessage::Event { data } => {
                if !self.paused {
                    self.transactions.push(data);
                    // 최대 5000개 유지
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
            _ => {}
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        // 글로벌 키: q/Ctrl+c로 종료
        if key.code == KeyCode::Char('q')
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.running = false;
            return;
        }

        // 탭 전환
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

        // 탭별 키 처리
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
            KeyCode::Char('t') => {
                // 토글 활성화/비활성화
                if let Some(idx) = self.selected_rule {
                    if idx < self.rules.len() {
                        self.rules[idx].enabled = !self.rules[idx].enabled;
                        self.send_rules_update().await;
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                // 규칙 삭제
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
                // 모든 규칙 삭제
                self.rules.clear();
                self.selected_rule = None;
                self.send_rules_update().await;
            }
            _ => {}
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
