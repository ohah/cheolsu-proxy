use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use proxy_daemon::BreakpointAction;

use super::forms::RuleForm;
use super::utils::{copy_to_clipboard, format_curl_command, format_ws_messages};
use super::{App, BreakpointAddForm, BreakpointFocus, BreakpointFormField};
use crate::tabs::Tab;

impl App {
    pub(crate) async fn handle_key(&mut self, key: KeyEvent) {
        // If breakpoint add form is open, handle form input
        if self.bp_add_form.is_some() {
            self.handle_bp_add_form_key(key).await;
            return;
        }

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
                self.tab = Tab::Breakpoint;
                return;
            }
            KeyCode::Char('6') if key.modifiers.contains(KeyModifiers::ALT) => {
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
            Tab::Breakpoint => self.handle_breakpoint_key(key).await,
            Tab::Settings => self.handle_settings_key(key).await,
        }
    }

    async fn handle_network_key(&mut self, key: KeyEvent) {
        // Diff view: Esc to go back, j/k to scroll
        if self.show_diff {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.show_diff = false;
                    self.diff_result = None;
                    self.diff_scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.diff_scroll = self.diff_scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.diff_scroll = self.diff_scroll.saturating_sub(1);
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.diff_scroll = 0;
                }
                _ => {}
            }
            return;
        }

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
            KeyCode::Char('e') => {
                self.export_har();
            }
            KeyCode::Char('D') => {
                // Diff: mark first transaction or run diff with second
                if let Some(idx) = self.selected_transaction {
                    if let Some(mark_idx) = self.diff_mark {
                        if mark_idx != idx {
                            self.run_diff(mark_idx, idx);
                        } else {
                            self.diff_mark = None;
                            self.set_status("Diff mark cleared");
                        }
                    } else {
                        self.diff_mark = Some(idx);
                        let uri = self
                            .transactions
                            .get(idx)
                            .and_then(|t| t.0.as_ref())
                            .map(|r| r.uri().to_string())
                            .unwrap_or_default();
                        self.set_status(&format!("Diff marked: #{} {}", idx, uri));
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
        let Some(form) = self.rule_form.as_mut() else {
            return;
        };

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
                    super::forms::RuleFormField::ActionType => match key.code {
                        KeyCode::Left => form.action_type = form.action_type.prev(),
                        KeyCode::Right => form.action_type = form.action_type.next(),
                        _ => {}
                    },
                    super::forms::RuleFormField::Method => match key.code {
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
                            super::forms::RuleFormField::Name => &mut form.name,
                            super::forms::RuleFormField::Pattern => &mut form.pattern,
                            super::forms::RuleFormField::StatusCode => &mut form.status_code,
                            super::forms::RuleFormField::Body => &mut form.body,
                            super::forms::RuleFormField::TargetUrl => &mut form.target_url,
                            super::forms::RuleFormField::FilePath => &mut form.file_path,
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

    async fn handle_settings_key(&mut self, key: KeyEvent) {
        use super::forms::{SettingsSection, ThrottleField, UpstreamProxyField};

        // Text editing mode (upstream or throttle)
        let is_editing = match self.settings_section {
            SettingsSection::UpstreamProxy => self.upstream_form.editing,
            SettingsSection::Throttle => self.throttle_form.editing,
        };

        if is_editing {
            match self.settings_section {
                SettingsSection::UpstreamProxy => match key.code {
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
                },
                SettingsSection::Throttle => {
                    match key.code {
                        KeyCode::Esc => {
                            self.throttle_form.editing = false;
                        }
                        KeyCode::Enter => {
                            self.throttle_form.editing = false;
                            self.send_throttle_update().await;
                        }
                        KeyCode::Char(c) => {
                            let field = match self.throttle_form.field {
                                ThrottleField::Download => &mut self.throttle_form.download,
                                ThrottleField::Upload => &mut self.throttle_form.upload,
                                ThrottleField::Latency => &mut self.throttle_form.latency,
                                _ => return,
                            };
                            // 숫자만 허용
                            if c.is_ascii_digit() {
                                field.push(c);
                            }
                        }
                        KeyCode::Backspace => {
                            let field = match self.throttle_form.field {
                                ThrottleField::Download => &mut self.throttle_form.download,
                                ThrottleField::Upload => &mut self.throttle_form.upload,
                                ThrottleField::Latency => &mut self.throttle_form.latency,
                                _ => return,
                            };
                            field.pop();
                        }
                        _ => {}
                    }
                }
            }
            return;
        }

        // Navigation mode
        match key.code {
            // 섹션 전환: H/L 또는 Left/Right
            KeyCode::Char('H') | KeyCode::Char('h') if !is_editing => {
                self.settings_section = self.settings_section.prev();
            }
            KeyCode::Char('L') | KeyCode::Char('l') if !is_editing => {
                self.settings_section = self.settings_section.next();
            }

            KeyCode::Up | KeyCode::Char('k') => match self.settings_section {
                SettingsSection::UpstreamProxy => {
                    self.upstream_form.field = self.upstream_form.field.prev();
                }
                SettingsSection::Throttle => {
                    self.throttle_form.field = self.throttle_form.field.prev();
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match self.settings_section {
                SettingsSection::UpstreamProxy => {
                    self.upstream_form.field = self.upstream_form.field.next();
                }
                SettingsSection::Throttle => {
                    self.throttle_form.field = self.throttle_form.field.next();
                }
            },
            KeyCode::Enter | KeyCode::Char(' ') => match self.settings_section {
                SettingsSection::UpstreamProxy => {
                    if self.upstream_form.field == UpstreamProxyField::Enabled {
                        self.upstream_form.enabled = !self.upstream_form.enabled;
                        self.send_upstream_update().await;
                    } else {
                        self.upstream_form.editing = true;
                    }
                }
                SettingsSection::Throttle => match self.throttle_form.field {
                    ThrottleField::Enabled => {
                        self.throttle_form.enabled = !self.throttle_form.enabled;
                        self.send_throttle_update().await;
                    }
                    ThrottleField::Preset => {
                        self.throttle_form.preset = self.throttle_form.preset.next();
                        self.send_throttle_update().await;
                    }
                    ThrottleField::Download | ThrottleField::Upload | ThrottleField::Latency => {
                        self.throttle_form.editing = true;
                    }
                },
            },
            KeyCode::Left => {
                if self.settings_section == SettingsSection::Throttle
                    && self.throttle_form.field == ThrottleField::Preset
                {
                    self.throttle_form.preset = self.throttle_form.preset.prev();
                    self.send_throttle_update().await;
                } else {
                    self.settings_section = self.settings_section.prev();
                }
            }
            KeyCode::Right => {
                if self.settings_section == SettingsSection::Throttle
                    && self.throttle_form.field == ThrottleField::Preset
                {
                    self.throttle_form.preset = self.throttle_form.preset.next();
                    self.send_throttle_update().await;
                } else {
                    self.settings_section = self.settings_section.next();
                }
            }
            KeyCode::Char('i') => {
                self.install_ca_cert();
            }
            KeyCode::Char('U') => {
                self.uninstall_ca_cert();
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
}
