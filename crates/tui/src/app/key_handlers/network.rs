/// Network 탭 키 핸들러
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::utils::{copy_to_clipboard, format_curl_command};
use crate::app::App;

impl App {
    pub(in crate::app) async fn handle_network_key(&mut self, key: KeyEvent) {
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
                            if let Some(req) = &info.request {
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
                        if let Some(req) = &info.request {
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
                            .and_then(|t| t.request.as_ref())
                            .map(|r| r.uri().to_string())
                            .unwrap_or_default();
                        self.set_status(&format!("Diff marked: #{} {}", idx, uri));
                    }
                }
            }
            KeyCode::Char('S') => {
                // Session save: Shift+S로 경로 입력 모드 진입
                self.session_save_path_input = format!(
                    "cheolsu-session-{}.cheolsu",
                    chrono::Local::now().format("%Y%m%d-%H%M%S")
                );
                self.session_save_editing = true;
            }
            KeyCode::Char('L') => {
                // Session load: Shift+L로 경로 입력 모드 진입
                self.session_load_path_input.clear();
                self.session_load_editing = true;
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
}
