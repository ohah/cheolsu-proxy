use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;
use proxy_daemon::InterceptRule;
use proxy_v2_models::{RequestInfo, WsConnectionEvent, WsMessageInfo};

pub const MAX_TRANSACTIONS: usize = 1000;
pub const MAX_WS_MESSAGES: usize = 5000;

#[derive(Clone)]
pub struct Store {
    pub transactions: Arc<Mutex<VecDeque<RequestInfo>>>,
    pub ws_messages: Arc<Mutex<VecDeque<WsMessageInfo>>>,
    pub ws_connections: Arc<Mutex<Vec<WsConnectionEvent>>>,
    pub rules: Arc<Mutex<Vec<InterceptRule>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_TRANSACTIONS))),
            ws_messages: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_WS_MESSAGES))),
            ws_connections: Arc::new(Mutex::new(Vec::new())),
            rules: Arc::new(Mutex::new(Vec::new())),
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
