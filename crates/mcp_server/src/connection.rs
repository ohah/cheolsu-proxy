use proxy_daemon::{connect_to_daemon, is_daemon_running, DaemonConnection, DaemonMessage};

use crate::store::Store;

pub async fn try_connect_daemon(store: &Store) -> Option<DaemonConnection> {
    if is_daemon_running().is_none() {
        return None;
    }

    let store = store.clone();
    match connect_to_daemon(move |msg| match msg {
        DaemonMessage::Event { data } => store.push_transaction(data),
        DaemonMessage::WsMessage { data } => store.push_ws_message(data),
        DaemonMessage::WsConnection { data } => store.push_ws_connection(data),
        DaemonMessage::InterceptRulesUpdated { rules } => {
            *store.rules.lock() = rules;
        }
        DaemonMessage::BreakpointRulesUpdated { rules } => {
            *store.breakpoint_rules.lock() = rules;
        }
        DaemonMessage::HostMappingsUpdated { mappings } => {
            *store.host_mappings.lock() = mappings;
        }
        _ => {}
    })
    .await
    {
        Ok(conn) => {
            tracing::info!("Connected to proxy daemon on port {}", conn.port);
            Some(conn)
        }
        Err(e) => {
            tracing::warn!("Failed to connect to daemon: {}", e);
            None
        }
    }
}
