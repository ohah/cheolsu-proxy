use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;
use proxy_daemon::{
    BreakpointRule, HostMapping, InterceptRule, ReverseProxyRule, ServerReplayEntry,
};
use proxy_v2_models::{
    RequestInfo, SseConnectionEvent, SseEventInfo, WsConnectionEvent, WsMessageInfo,
};

pub const MAX_TRANSACTIONS: usize = 1000;
pub const MAX_WS_MESSAGES: usize = 5000;
pub const MAX_SSE_EVENTS: usize = 5000;

/// Store의 용량 설정
pub struct StoreConfig {
    pub max_transactions: usize,
    pub max_ws_messages: usize,
    pub max_sse_events: usize,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            max_transactions: MAX_TRANSACTIONS,
            max_ws_messages: MAX_WS_MESSAGES,
            max_sse_events: MAX_SSE_EVENTS,
        }
    }
}

#[derive(Clone)]
pub struct Store {
    pub transactions: Arc<Mutex<VecDeque<RequestInfo>>>,
    pub ws_messages: Arc<Mutex<VecDeque<WsMessageInfo>>>,
    pub ws_connections: Arc<Mutex<Vec<WsConnectionEvent>>>,
    pub rules: Arc<Mutex<Vec<InterceptRule>>>,
    pub breakpoint_rules: Arc<Mutex<Vec<BreakpointRule>>>,
    pub host_mappings: Arc<Mutex<Vec<HostMapping>>>,
    pub sse_events: Arc<Mutex<VecDeque<SseEventInfo>>>,
    pub sse_connections: Arc<Mutex<Vec<SseConnectionEvent>>>,
    pub server_replay_entries: Arc<Mutex<Vec<ServerReplayEntry>>>,
    pub reverse_proxy_rules: Arc<Mutex<Vec<ReverseProxyRule>>>,
    max_transactions: usize,
    max_ws_messages: usize,
    max_sse_events: usize,
}

impl Store {
    pub fn new() -> Self {
        Self::with_config(StoreConfig::default())
    }

    pub fn with_config(config: StoreConfig) -> Self {
        Self {
            transactions: Arc::new(Mutex::new(VecDeque::with_capacity(config.max_transactions))),
            ws_messages: Arc::new(Mutex::new(VecDeque::with_capacity(config.max_ws_messages))),
            ws_connections: Arc::new(Mutex::new(Vec::new())),
            rules: Arc::new(Mutex::new(Vec::new())),
            breakpoint_rules: Arc::new(Mutex::new(Vec::new())),
            host_mappings: Arc::new(Mutex::new(Vec::new())),
            sse_events: Arc::new(Mutex::new(VecDeque::with_capacity(config.max_sse_events))),
            sse_connections: Arc::new(Mutex::new(Vec::new())),
            server_replay_entries: Arc::new(Mutex::new(Vec::new())),
            reverse_proxy_rules: Arc::new(Mutex::new(Vec::new())),
            max_transactions: config.max_transactions,
            max_ws_messages: config.max_ws_messages,
            max_sse_events: config.max_sse_events,
        }
    }

    pub fn push_transaction(&self, info: RequestInfo) {
        let mut txns = self.transactions.lock();
        if txns.len() >= self.max_transactions {
            txns.pop_front();
        }
        txns.push_back(info);
    }

    pub fn push_ws_message(&self, msg: WsMessageInfo) {
        let mut msgs = self.ws_messages.lock();
        if msgs.len() >= self.max_ws_messages {
            msgs.pop_front();
        }
        msgs.push_back(msg);
    }

    pub fn push_ws_connection(&self, event: WsConnectionEvent) {
        self.ws_connections.lock().push(event);
    }

    pub fn push_sse_event(&self, event: SseEventInfo) {
        let mut events = self.sse_events.lock();
        if events.len() >= self.max_sse_events {
            events.pop_front();
        }
        events.push_back(event);
    }

    pub fn push_sse_connection(&self, event: SseConnectionEvent) {
        self.sse_connections.lock().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proxy_v2_models::{WsContentType, WsDirection, WsMessageType};

    fn make_sse_event(seq: u64) -> SseEventInfo {
        SseEventInfo {
            connection_id: "sse://localhost".to_string(),
            sequence: seq,
            event_type: Some("message".to_string()),
            data: "hello".to_string(),
            id: None,
            retry: None,
            size: 5,
            time: 0,
        }
    }

    #[test]
    fn with_config_custom_limits() {
        let store = Store::with_config(StoreConfig {
            max_transactions: 5,
            max_ws_messages: 10,
            max_sse_events: 10,
        });
        for _ in 0..20 {
            store.push_transaction(make_request_info());
            store.push_ws_message(make_ws_message(0));
            store.push_sse_event(make_sse_event(0));
        }
        assert_eq!(store.transactions.lock().len(), 5);
        assert_eq!(store.ws_messages.lock().len(), 10);
        assert_eq!(store.sse_events.lock().len(), 10);
    }

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
        assert!(store.sse_events.lock().is_empty());
        assert!(store.sse_connections.lock().is_empty());
        assert!(store.server_replay_entries.lock().is_empty());
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
    fn push_sse_event_adds_entry() {
        let store = Store::new();
        store.push_sse_event(make_sse_event(1));
        assert_eq!(store.sse_events.lock().len(), 1);
    }

    #[test]
    fn push_sse_event_evicts_oldest_at_capacity() {
        let store = Store::new();
        for i in 0..(MAX_SSE_EVENTS + 10) as u64 {
            store.push_sse_event(make_sse_event(i));
        }
        let events = store.sse_events.lock();
        assert_eq!(events.len(), MAX_SSE_EVENTS);
        assert_eq!(events.front().unwrap().sequence, 10);
    }

    #[test]
    fn push_sse_connection_adds_entry() {
        let store = Store::new();
        store.push_sse_connection(SseConnectionEvent::Connected {
            connection_id: "sse1".to_string(),
            uri: "https://example.com/events".to_string(),
            time: 0,
        });
        assert_eq!(store.sse_connections.lock().len(), 1);
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
