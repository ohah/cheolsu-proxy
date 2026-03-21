use std::collections::VecDeque;
use std::sync::Arc;

use cheolsu_ops::context::{OpsContext, OpsStore};
use parking_lot::Mutex;
use proxy_daemon::{connect_to_daemon, is_daemon_running, DaemonMessage};
use tokio::sync::Mutex as TokioMutex;

/// daemon에 연결하고 OpsContext를 생성한다.
/// 연결 실패 시 에러를 반환한다.
pub async fn connect() -> Result<OpsContext, String> {
    if is_daemon_running().is_none() {
        return Err("Cheolsu Proxy 데몬이 실행 중이 아닙니다. 먼저 프록시를 시작해주세요.".into());
    }

    let store = OpsStore {
        transactions: Arc::new(Mutex::new(VecDeque::new())),
        ws_messages: Arc::new(Mutex::new(VecDeque::new())),
        ws_connections: Arc::new(Mutex::new(Vec::new())),
        rules: Arc::new(Mutex::new(Vec::new())),
        breakpoint_rules: Arc::new(Mutex::new(Vec::new())),
        host_mappings: Arc::new(Mutex::new(Vec::new())),
        sse_events: Arc::new(Mutex::new(VecDeque::new())),
        sse_connections: Arc::new(Mutex::new(Vec::new())),
        server_replay_entries: Arc::new(Mutex::new(Vec::new())),
        reverse_proxy_rules: Arc::new(Mutex::new(Vec::new())),
    };

    let store_clone = store.clone();
    let conn = connect_to_daemon(move |msg| match msg {
        DaemonMessage::Event { data } => {
            let mut txns = store_clone.transactions.lock();
            if txns.len() >= 1000 {
                txns.pop_front();
            }
            txns.push_back(data);
        }
        DaemonMessage::WsMessage { data } => {
            let mut msgs = store_clone.ws_messages.lock();
            if msgs.len() >= 5000 {
                msgs.pop_front();
            }
            msgs.push_back(data);
        }
        DaemonMessage::WsConnection { data } => {
            store_clone.ws_connections.lock().push(data);
        }
        DaemonMessage::InterceptRulesUpdated { rules } => {
            *store_clone.rules.lock() = rules;
        }
        DaemonMessage::BreakpointRulesUpdated { rules } => {
            *store_clone.breakpoint_rules.lock() = rules;
        }
        DaemonMessage::HostMappingsUpdated { mappings } => {
            *store_clone.host_mappings.lock() = mappings;
        }
        DaemonMessage::SseEvent { data } => {
            let mut events = store_clone.sse_events.lock();
            if events.len() >= 5000 {
                events.pop_front();
            }
            events.push_back(data);
        }
        DaemonMessage::SseConnection { data } => {
            store_clone.sse_connections.lock().push(data);
        }
        _ => {}
    })
    .await
    .map_err(|e| format!("데몬 연결 실패: {}", e))?;

    Ok(OpsContext {
        store,
        daemon_conn: Arc::new(TokioMutex::new(Some(conn))),
    })
}

/// daemon에 연결 후 초기 데이터 동기화를 위해 잠시 대기한다.
pub async fn connect_and_sync() -> Result<OpsContext, String> {
    let ctx = connect().await?;
    // daemon이 초기 상태를 브로드캐스트할 시간을 준다
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    Ok(ctx)
}
