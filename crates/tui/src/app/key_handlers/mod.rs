/// 키 핸들러 모듈 - 탭별 키 입력 처리를 서브모듈로 분리
mod breakpoint;
mod logs;
mod network;
mod rules;
mod script;
mod settings;
mod websocket;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::App;
use crate::tabs::Tab;

impl App {
    pub(crate) async fn handle_key(&mut self, key: KeyEvent) {
        // If breakpoint edit form is open, handle form input
        if self.bp_edit_form.is_some() {
            self.handle_bp_edit_form_key(key).await;
            return;
        }

        // If breakpoint add form is open, handle form input
        if self.bp_add_form.is_some() {
            self.handle_bp_add_form_key(key).await;
            return;
        }

        // If session save path is in editing mode, handle it
        if self.tab == Tab::Network && self.session_save_editing {
            self.handle_session_save_key(key);
            return;
        }

        // If session load path is in editing mode, handle it
        if self.tab == Tab::Network && self.session_load_editing {
            self.handle_session_load_key(key);
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

        // If log filter is in editing mode, handle it
        if self.tab == Tab::Logs && self.log_filter_editing {
            self.handle_logs_key(key);
            return;
        }

        // If SSL Proxying add form is open, handle it
        if self.tab == Tab::Settings && self.ssl_proxying_add_form.is_some() {
            self.handle_settings_key(key).await;
            return;
        }

        // If host mapping form is open, handle it
        if self.tab == Tab::Settings && self.host_mapping_form.is_some() {
            self.handle_settings_key(key).await;
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
            KeyCode::Char('7') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.tab = Tab::Logs;
                self.refresh_log_files();
                self.refresh_log_content();
                return;
            }
            _ => {}
        }

        // 탭별 키 처리
        match self.tab {
            Tab::Network => self.handle_network_key(key).await,
            Tab::WebSocket => self.handle_ws_key(key),
            Tab::InterceptRules => self.handle_rules_key(key).await,
            Tab::Script => self.handle_script_key(key).await,
            Tab::Breakpoint => self.handle_breakpoint_key(key).await,
            Tab::Settings => self.handle_settings_key(key).await,
            Tab::Logs => self.handle_logs_key(key),
        }
    }

    /// 세션 저장 경로 입력 키 핸들러
    fn handle_session_save_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.session_save_editing = false;
            }
            KeyCode::Enter => {
                self.session_save_editing = false;
                self.save_session();
            }
            KeyCode::Char(c) => {
                self.session_save_path_input.push(c);
            }
            KeyCode::Backspace => {
                self.session_save_path_input.pop();
            }
            _ => {}
        }
    }

    /// 세션 로드 경로 입력 키 핸들러
    fn handle_session_load_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.session_load_editing = false;
            }
            KeyCode::Enter => {
                self.session_load_editing = false;
                self.load_session();
            }
            KeyCode::Char(c) => {
                self.session_load_path_input.push(c);
            }
            KeyCode::Backspace => {
                self.session_load_path_input.pop();
            }
            _ => {}
        }
    }
}
