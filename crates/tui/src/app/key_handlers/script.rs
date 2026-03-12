/// Script 탭 키 핸들러
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

impl App {
    pub(in crate::app) async fn handle_script_key(&mut self, key: KeyEvent) {
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
