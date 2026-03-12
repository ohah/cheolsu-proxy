/// WebSocket 탭 키 핸들러
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::utils::{copy_to_clipboard, format_ws_messages};
use crate::app::App;

impl App {
    pub(in crate::app) fn handle_ws_key(&mut self, key: KeyEvent) {
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
}
