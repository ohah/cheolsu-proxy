use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;
use proxy_daemon::{
    BreakpointRule, ClientCommand, DaemonConnection, DaemonMessage, HostMapping, InterceptRule,
    ReverseProxyRule, ServerReplayEntry,
};
use proxy_v2_models::{
    RequestInfo, SseConnectionEvent, SseEventInfo, WsConnectionEvent, WsMessageInfo,
};
use tokio::sync::Mutex as TokioMutex;

use crate::helpers::with_daemon_conn;

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

/// MCP/CLI 공유 데이터 저장소.
#[derive(Clone)]
pub struct OpsStore {
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

impl OpsStore {
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

    /// DaemonMessage를 처리하여 Store를 업데이트한다.
    pub fn handle_daemon_message(&self, msg: DaemonMessage) {
        match msg {
            DaemonMessage::Event { data } => self.push_transaction(data),
            DaemonMessage::WsMessage { data } => self.push_ws_message(data),
            DaemonMessage::WsConnection { data } => self.push_ws_connection(data),
            DaemonMessage::InterceptRulesUpdated { rules } => {
                *self.rules.lock() = rules;
            }
            DaemonMessage::BreakpointRulesUpdated { rules } => {
                *self.breakpoint_rules.lock() = rules;
            }
            DaemonMessage::HostMappingsUpdated { mappings } => {
                *self.host_mappings.lock() = mappings;
            }
            DaemonMessage::SseEvent { data } => self.push_sse_event(data),
            DaemonMessage::SseConnection { data } => self.push_sse_connection(data),
            _ => {}
        }
    }
}

/// Store + DaemonConnection을 묶은 공유 컨텍스트.
#[derive(Clone)]
pub struct OpsContext {
    pub store: OpsStore,
    pub daemon_conn: Arc<TokioMutex<Option<DaemonConnection>>>,
}

impl OpsContext {
    pub async fn send_rules(&self) -> Result<(), String> {
        let cmd = {
            let rules = self.store.rules.lock();
            ClientCommand::UpdateInterceptRules {
                rules: rules.clone(),
            }
        };
        with_daemon_conn(&self.daemon_conn, &cmd).await
    }

    pub async fn send_breakpoint_rules(&self) -> Result<(), String> {
        let cmd = {
            let rules = self.store.breakpoint_rules.lock();
            ClientCommand::UpdateBreakpointRules {
                rules: rules.clone(),
            }
        };
        with_daemon_conn(&self.daemon_conn, &cmd).await
    }

    pub async fn send_host_mappings(&self) -> Result<(), String> {
        let cmd = {
            let mappings = self.store.host_mappings.lock();
            ClientCommand::UpdateHostMappings {
                mappings: mappings.clone(),
            }
        };
        with_daemon_conn(&self.daemon_conn, &cmd).await
    }

    pub async fn send_reverse_proxy_rules(&self) -> Result<(), String> {
        let cmd = {
            let rules = self.store.reverse_proxy_rules.lock();
            ClientCommand::UpdateReverseProxyRules {
                rules: rules.clone(),
            }
        };
        with_daemon_conn(&self.daemon_conn, &cmd).await
    }

    pub async fn send_server_replay(&self) -> Result<(), String> {
        let cmd = {
            let entries = self.store.server_replay_entries.lock();
            ClientCommand::UpdateServerReplay {
                entries: entries.clone(),
            }
        };
        with_daemon_conn(&self.daemon_conn, &cmd).await
    }
}
