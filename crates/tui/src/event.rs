use crossterm::event::{Event as CrosstermEvent, KeyEvent};
use futures_util::StreamExt;
use proxy_daemon::DaemonMessage;
use std::time::Duration;
use tokio::sync::mpsc;

/// TUI에서 처리하는 이벤트
pub enum Event {
    /// 키보드 입력
    Key(KeyEvent),
    /// 터미널 리사이즈
    Resize,
    /// 데몬에서 온 메시지
    Daemon(DaemonMessage),
    /// 주기적 틱 (UI 갱신용)
    Tick,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> (Self, mpsc::UnboundedSender<Event>) {
        let (tx, rx) = mpsc::unbounded_channel();

        let event_tx = tx.clone();
        tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                tokio::select! {
                    _ = tick_interval.tick() => {
                        let _ = event_tx.send(Event::Tick);
                    }
                    Some(Ok(evt)) = reader.next() => {
                        match evt {
                            CrosstermEvent::Key(key) => {
                                let _ = event_tx.send(Event::Key(key));
                            }
                            CrosstermEvent::Resize(_, _) => {
                                let _ = event_tx.send(Event::Resize);
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        (Self { rx }, tx)
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
