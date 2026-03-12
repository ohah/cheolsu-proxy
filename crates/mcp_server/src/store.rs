use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;
use proxy_daemon::{BreakpointRule, HostMapping, InterceptRule};
use proxy_v2_models::{RequestInfo, WsConnectionEvent, WsMessageInfo};

pub const MAX_TRANSACTIONS: usize = 1000;
pub const MAX_WS_MESSAGES: usize = 5000;

#[derive(Clone)]
pub struct Store {
    pub transactions: Arc<Mutex<VecDeque<RequestInfo>>>,
    pub ws_messages: Arc<Mutex<VecDeque<WsMessageInfo>>>,
    pub ws_connections: Arc<Mutex<Vec<WsConnectionEvent>>>,
    pub rules: Arc<Mutex<Vec<InterceptRule>>>,
    pub breakpoint_rules: Arc<Mutex<Vec<BreakpointRule>>>,
    pub host_mappings: Arc<Mutex<Vec<HostMapping>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_TRANSACTIONS))),
            ws_messages: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_WS_MESSAGES))),
            ws_connections: Arc::new(Mutex::new(Vec::new())),
            rules: Arc::new(Mutex::new(Vec::new())),
            breakpoint_rules: Arc::new(Mutex::new(Vec::new())),
            host_mappings: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn push_transaction(&self, info: RequestInfo) {
        let mut txns = self.transactions.lock();
        if txns.len() >= MAX_TRANSACTIONS {
            txns.pop_front();
        }
        txns.push_back(info);
    }

    pub fn push_ws_message(&self, msg: WsMessageInfo) {
        let mut msgs = self.ws_messages.lock();
        if msgs.len() >= MAX_WS_MESSAGES {
            msgs.pop_front();
        }
        msgs.push_back(msg);
    }

    pub fn push_ws_connection(&self, event: WsConnectionEvent) {
        self.ws_connections.lock().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proxy_v2_models::{WsContentType, WsDirection, WsMessageType};

    fn make_request_info() -> RequestInfo {
        RequestInfo(None, None)
    }

    fn make_ws_message(seq: u64) -> WsMessageInfo {
        WsMessageInfo {
            connection_id: "ws://localhost".to_string(),
            sequence: seq,
            direction: WsDirection::ClientToServer,
            message_type: WsMessageType::Text,
            payload: "hello".to_string(),
            size: 5,
            time: 0,
            is_binary: false,
            content_type: WsContentType::default(),
            mqtt_version: None,
        }
    }

    fn make_ws_connection_event(id: &str) -> WsConnectionEvent {
        WsConnectionEvent::Connected {
            connection_id: id.to_string(),
            uri: format!("ws://{}", id),
            time: 0,
        }
    }

    #[test]
    fn new_store_is_empty() {
        let store = Store::new();
        assert!(store.transactions.lock().is_empty());
        assert!(store.ws_messages.lock().is_empty());
        assert!(store.ws_connections.lock().is_empty());
        assert!(store.rules.lock().is_empty());
        assert!(store.breakpoint_rules.lock().is_empty());
        assert!(store.host_mappings.lock().is_empty());
    }

    #[test]
    fn push_transaction_adds_entry() {
        let store = Store::new();
        store.push_transaction(make_request_info());
        assert_eq!(store.transactions.lock().len(), 1);
    }

    #[test]
    fn push_transaction_evicts_oldest_at_capacity() {
        let store = Store::new();
        for _ in 0..MAX_TRANSACTIONS + 5 {
            store.push_transaction(make_request_info());
        }
        assert_eq!(store.transactions.lock().len(), MAX_TRANSACTIONS);
    }

    #[test]
    fn push_ws_message_adds_entry() {
        let store = Store::new();
        store.push_ws_message(make_ws_message(1));
        assert_eq!(store.ws_messages.lock().len(), 1);
    }

    #[test]
    fn push_ws_message_evicts_oldest_at_capacity() {
        let store = Store::new();
        for i in 0..(MAX_WS_MESSAGES + 10) as u64 {
            store.push_ws_message(make_ws_message(i));
        }
        let msgs = store.ws_messages.lock();
        assert_eq!(msgs.len(), MAX_WS_MESSAGES);
        assert_eq!(msgs.front().unwrap().sequence, 10);
    }

    #[test]
    fn push_ws_connection_adds_entry() {
        let store = Store::new();
        store.push_ws_connection(make_ws_connection_event("conn1"));
        store.push_ws_connection(make_ws_connection_event("conn2"));
        assert_eq!(store.ws_connections.lock().len(), 2);
    }

    #[test]
    fn rules_can_be_modified() {
        let store = Store::new();
        store.rules.lock().push(InterceptRule {
            id: "r1".to_string(),
            name: "test".to_string(),
            enabled: true,
            pattern: "*.example.com".to_string(),
            method: None,
            action: proxy_daemon::InterceptAction::Block {
                status_code: 403,
                body: String::new(),
            },
        });
        assert_eq!(store.rules.lock().len(), 1);
    }

    #[test]
    fn breakpoint_rules_can_be_modified() {
        let store = Store::new();
        store.breakpoint_rules.lock().push(BreakpointRule {
            id: "bp1".to_string(),
            pattern: "/api/*".to_string(),
            break_on_request: true,
            break_on_response: false,
            enabled: true,
        });
        assert_eq!(store.breakpoint_rules.lock().len(), 1);
        assert!(store.breakpoint_rules.lock()[0].break_on_request);
    }

    #[test]
    fn host_mappings_can_be_modified() {
        let store = Store::new();
        store.host_mappings.lock().push(HostMapping {
            id: "hm1".to_string(),
            source_host: "api.example.com".to_string(),
            source_port: None,
            target_host: "127.0.0.1".to_string(),
            target_port: Some(8080),
            enabled: true,
        });
        assert_eq!(store.host_mappings.lock().len(), 1);
    }

    #[test]
    fn clone_shares_state() {
        let store = Store::new();
        let cloned = store.clone();
        store.push_transaction(make_request_info());
        assert_eq!(cloned.transactions.lock().len(), 1);
    }

    #[test]
    fn concurrent_access() {
        let store = Store::new();
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let s = store.clone();
                std::thread::spawn(move || {
                    for j in 0..100 {
                        s.push_transaction(make_request_info());
                        s.push_ws_message(make_ws_message(i * 100 + j));
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(store.transactions.lock().len(), MAX_TRANSACTIONS);
        assert_eq!(store.ws_messages.lock().len(), 1000);
    }
}
