/// Analytics 탭 키 핸들러
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

impl App {
    pub(crate) fn handle_analytics_key(&mut self, key: KeyEvent) {
        match key.code {
            // 새로고침
            KeyCode::Char('r') => {
                self.refresh_analytics();
                self.set_status("분석 새로고침 완료");
            }
            // 스크롤 다운
            KeyCode::Char('j') | KeyCode::Down => {
                self.analytics_scroll = self.analytics_scroll.saturating_add(1);
            }
            // 스크롤 업
            KeyCode::Char('k') | KeyCode::Up => {
                self.analytics_scroll = self.analytics_scroll.saturating_sub(1);
            }
            // 맨 위로
            KeyCode::Char('g') | KeyCode::Home => {
                self.analytics_scroll = 0;
            }
            // 맨 아래로
            KeyCode::Char('G') | KeyCode::End => {
                self.analytics_scroll = u16::MAX; // draw 시 max_scroll로 클램핑됨
            }
            _ => {}
        }
    }
}
