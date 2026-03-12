/// Logs 탭 키 핸들러
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

impl App {
    pub(crate) fn handle_logs_key(&mut self, key: KeyEvent) {
        // 필터 입력 모드
        if self.log_filter_editing {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.log_filter_editing = false;
                }
                KeyCode::Backspace => {
                    self.log_filter.pop();
                }
                KeyCode::Char(c) => {
                    self.log_filter.push(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            // 스크롤
            KeyCode::Char('j') | KeyCode::Down => {
                self.log_scroll = self.log_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.log_scroll = self.log_scroll.saturating_sub(1);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.log_scroll = 0;
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.log_scroll = self.log_content_lines.len().saturating_sub(1);
            }
            // 파일 선택
            KeyCode::Char('h') | KeyCode::Left => {
                if let Some(idx) = self.selected_log_file {
                    if idx > 0 {
                        self.selected_log_file = Some(idx - 1);
                        self.refresh_log_content();
                    }
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if let Some(idx) = self.selected_log_file {
                    if idx + 1 < self.log_files.len() {
                        self.selected_log_file = Some(idx + 1);
                        self.refresh_log_content();
                    }
                }
            }
            // 새로고침
            KeyCode::Char('r') => {
                self.refresh_log_files();
                self.refresh_log_content();
                self.status_message =
                    Some(("Logs refreshed".to_string(), std::time::Instant::now()));
            }
            // 필터
            KeyCode::Char('/') => {
                self.log_filter_editing = true;
            }
            // 필터 클리어
            KeyCode::Char('c') => {
                self.log_filter.clear();
            }
            // 파일 클리어
            KeyCode::Char('C') => {
                self.clear_selected_log();
                self.status_message =
                    Some(("Log file cleared".to_string(), std::time::Instant::now()));
            }
            _ => {}
        }
    }
}
