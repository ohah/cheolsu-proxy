use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;
use proxy_daemon::{
    BreakpointRule, ClientCommand, DaemonConnection, HostMapping, InterceptRule, ReverseProxyRule,
    ServerReplayEntry,
};
use proxy_v2_models::{
    RequestInfo, SseConnectionEvent, SseEventInfo, WsConnectionEvent, WsMessageInfo,
};
use tokio::sync::Mutex as TokioMutex;

use crate::helpers::with_daemon_conn;

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
