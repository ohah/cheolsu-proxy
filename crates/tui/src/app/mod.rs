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

/// Breakpoint 수정 후 전달 폼
#[derive(Debug, Clone)]
pub struct BreakpointEditForm {
    pub bp_id: String,
    pub phase: BreakpointPhase,
    pub headers_text: String,
    pub body: String,
    pub status: String,
    pub field: BreakpointEditField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointEditField {
    Headers,
    Body,
    Status,
}

impl BreakpointEditForm {
    pub fn from_pending(entry: &PendingBreakpointEntry) -> Self {
        let headers_text = entry
            .data
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");
        Self {
            bp_id: entry.id.clone(),
            phase: entry.phase.clone(),
            headers_text,
            body: entry.data.body.clone().unwrap_or_default(),
            status: entry.data.status.map(|s| s.to_string()).unwrap_or_default(),
            field: BreakpointEditField::Headers,
        }
    }

    pub fn parse_headers(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        for line in self.headers_text.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim();
                if !k.is_empty() {
                    map.insert(k.to_string(), v.trim().to_string());
                }
            }
        }
        map
    }
}

/// 로그 파일 엔트리 (TUI 표시용)
#[derive(Debug, Clone)]
pub struct LogFileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
}

/// 분석 결과 캐시
#[derive(Debug, Clone, Default)]
pub struct AnalyticsResult {
    /// 총 요청 수
    pub total_requests: usize,
    /// 에러율 (%)
    pub error_rate: f64,
    /// 평균 응답시간 (ms)
    pub avg_duration_ms: f64,
    /// P95 응답시간 (ms)
    pub p95_duration_ms: u64,
    /// 느린 요청 Top 10
    pub slow_requests: Vec<SlowRequest>,
    /// 에러 분포 (status_code, count)
    pub error_distribution: Vec<(u16, usize)>,
    /// 중복 요청
    pub duplicate_requests: Vec<DuplicateRequest>,
    /// N+1 패턴
    pub n_plus_one_patterns: Vec<NPlusOnePattern>,
}

#[derive(Debug, Clone)]
pub struct SlowRequest {
    pub method: String,
    pub url: String,
    pub duration_ms: u64,
    pub status: u16,
}

#[derive(Debug, Clone)]
pub struct DuplicateRequest {
    pub method: String,
    pub url: String,
    pub count: usize,
    pub window_secs: u64,
}

#[derive(Debug, Clone)]
pub struct NPlusOnePattern {
    pub pattern: String,
    pub count: usize,
    pub suggestion: String,
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
    pub bp_edit_form: Option<BreakpointEditForm>,

    // Table states (for scroll)
    pub network_table_state: TableState,
    pub ws_conn_table_state: TableState,
    pub rules_table_state: TableState,

    // 렌더링 상태 추적 (레이아웃 전환 시 화면 클리어 용도)
    prev_tab: Option<Tab>,
    prev_show_detail: bool,
    prev_show_diff: bool,
    prev_settings_section: Option<SettingsSection>,

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

    // Analytics
    pub analytics_result: Option<AnalyticsResult>,
    pub analytics_scroll: u16,

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
            bp_edit_form: None,
            network_table_state: TableState::default(),
            ws_conn_table_state: TableState::default(),
            rules_table_state: TableState::default(),
            prev_tab: None,
            prev_show_detail: false,
            prev_show_diff: false,
            prev_settings_section: None,
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
            analytics_result: None,
            analytics_scroll: 0,
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

    /// 트래픽 데이터를 분석하여 결과를 캐시
    pub fn refresh_analytics(&mut self) {
        use std::collections::HashMap;

        let txns = &self.transactions;
        let total_requests = txns.len();

        if total_requests == 0 {
            self.analytics_result = Some(AnalyticsResult::default());
            return;
        }

        // 응답시간 수집 및 에러 카운트
        let mut durations: Vec<u64> = Vec::new();
        let mut error_count = 0usize;
        let mut error_map: HashMap<u16, usize> = HashMap::new();

        // URL별 요청 수집 (중복/N+1 탐지)
        let mut url_requests: HashMap<String, Vec<(String, i64)>> = HashMap::new();

        // 느린 요청 후보
        struct SlowCandidate {
            method: String,
            url: String,
            duration_ms: u64,
            status: u16,
        }
        let mut slow_candidates: Vec<SlowCandidate> = Vec::new();

        for info in txns.iter() {
            let method = info
                .request
                .as_ref()
                .map(|r| r.method().to_string())
                .unwrap_or_default();
            let url = info
                .request
                .as_ref()
                .map(|r| r.uri().to_string())
                .unwrap_or_default();
            let req_time = info.request.as_ref().map(|r| r.time()).unwrap_or(0);
            let status = info
                .response
                .as_ref()
                .map(|r| r.status().as_u16())
                .unwrap_or(0);

            // 타이밍 정보 수집
            let duration_ms = info
                .response
                .as_ref()
                .and_then(|r| r.timing().as_ref())
                .map(|t| t.total_ms)
                .unwrap_or_else(|| {
                    // 타이밍 정보 없으면 요청/응답 시간차로 계산
                    let res_time = info.response.as_ref().map(|r| r.time()).unwrap_or(0);
                    if res_time > 0 && req_time > 0 {
                        ((res_time - req_time).unsigned_abs()) / 1_000_000 // ns -> ms
                    } else {
                        0
                    }
                });

            durations.push(duration_ms);

            // 에러 카운트
            if status >= 400 {
                error_count += 1;
                *error_map.entry(status).or_insert(0) += 1;
            }

            // 느린 요청
            slow_candidates.push(SlowCandidate {
                method: method.clone(),
                url: url.clone(),
                duration_ms,
                status,
            });

            // URL별 요청 기록 (중복 탐지용)
            let key = format!("{} {}", method, url);
            url_requests
                .entry(key)
                .or_default()
                .push((method, req_time));
        }

        // 평균 응답시간
        let avg_duration_ms = if durations.is_empty() {
            0.0
        } else {
            durations.iter().sum::<u64>() as f64 / durations.len() as f64
        };

        // P95
        durations.sort_unstable();
        let p95_duration_ms = if durations.is_empty() {
            0
        } else {
            let p95_idx = ((durations.len() as f64) * 0.95).ceil() as usize;
            durations[p95_idx.min(durations.len() - 1)]
        };

        // 에러율
        let error_rate = (error_count as f64 / total_requests as f64) * 100.0;

        // 에러 분포 (개수 내림차순)
        let mut error_distribution: Vec<(u16, usize)> = error_map.into_iter().collect();
        error_distribution.sort_by(|a, b| b.1.cmp(&a.1));

        // 느린 요청 Top 10
        slow_candidates.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms));
        let slow_requests: Vec<SlowRequest> = slow_candidates
            .into_iter()
            .take(10)
            .map(|c| SlowRequest {
                method: c.method,
                url: c.url,
                duration_ms: c.duration_ms,
                status: c.status,
            })
            .collect();

        // 중복 요청 탐지 (같은 URL이 짧은 시간 내 3회 이상)
        let mut duplicate_requests: Vec<DuplicateRequest> = Vec::new();
        for (key, times) in &url_requests {
            if times.len() >= 3 {
                let mut sorted_times: Vec<i64> = times.iter().map(|(_, t)| *t).collect();
                sorted_times.sort_unstable();
                if let (Some(&first), Some(&last)) = (sorted_times.first(), sorted_times.last()) {
                    let window_ns = (last - first).unsigned_abs();
                    let window_secs = window_ns / 1_000_000_000;
                    if window_secs <= 10 {
                        let parts: Vec<&str> = key.splitn(2, ' ').collect();
                        let (method, url) = if parts.len() == 2 {
                            (parts[0].to_string(), parts[1].to_string())
                        } else {
                            (String::new(), key.clone())
                        };
                        duplicate_requests.push(DuplicateRequest {
                            method,
                            url,
                            count: times.len(),
                            window_secs: window_secs.max(1),
                        });
                    }
                }
            }
        }
        duplicate_requests.sort_by(|a, b| b.count.cmp(&a.count));
        duplicate_requests.truncate(10);

        // N+1 패턴 탐지 (URL 패스의 마지막 세그먼트가 변하는 패턴)
        let mut path_pattern_counts: HashMap<String, usize> = HashMap::new();
        for (key, times) in &url_requests {
            if times.len() >= 1 {
                let parts: Vec<&str> = key.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let url = parts[1];
                    // URL에서 path 추출 후 마지막 세그먼트를 {id}로 치환
                    if let Some(path) = url.split('?').next() {
                        let segments: Vec<&str> = path.split('/').collect();
                        if segments.len() >= 3 {
                            // 마지막 세그먼트가 숫자이거나 UUID-like인 경우
                            if let Some(last) = segments.last() {
                                if last.parse::<u64>().is_ok()
                                    || (last.len() > 8
                                        && last.chars().all(|c| c.is_alphanumeric() || c == '-'))
                                {
                                    let pattern_segs: Vec<&str> =
                                        segments[..segments.len() - 1].to_vec();
                                    let pattern =
                                        format!("{} {}/{{id}}", parts[0], pattern_segs.join("/"));
                                    *path_pattern_counts.entry(pattern).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut n_plus_one_patterns: Vec<NPlusOnePattern> = path_pattern_counts
            .into_iter()
            .filter(|(_, count)| *count >= 5)
            .map(|(pattern, count)| NPlusOnePattern {
                pattern,
                count,
                suggestion: "배치 API 사용 권장".to_string(),
            })
            .collect();
        n_plus_one_patterns.sort_by(|a, b| b.count.cmp(&a.count));
        n_plus_one_patterns.truncate(10);

        self.analytics_result = Some(AnalyticsResult {
            total_requests,
            error_rate,
            avg_duration_ms,
            p95_duration_ms,
            slow_requests,
            error_distribution,
            duplicate_requests,
            n_plus_one_patterns,
        });
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
            // 레이아웃 전환 시 화면 전체를 클리어하여 렌더링 잔상 방지
            let layout_changed = self.prev_tab != Some(self.tab)
                || self.prev_show_detail != self.show_detail
                || self.prev_show_diff != self.show_diff
                || self.prev_settings_section != Some(self.settings_section);
            if layout_changed {
                terminal.clear()?;
                self.prev_tab = Some(self.tab);
                self.prev_show_detail = self.show_detail;
                self.prev_show_diff = self.show_diff;
                self.prev_settings_section = Some(self.settings_section);
            }

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
                if let Some(req) = &info.request {
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
