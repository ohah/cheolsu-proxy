use proxy_daemon::{connect_to_daemon, is_daemon_running, DaemonConnection};

use crate::store::Store;

pub async fn try_connect_daemon(store: &Store) -> Option<DaemonConnection> {
    if is_daemon_running().is_none() {
        return None;
    }

    let store = store.clone();
    match connect_to_daemon(move |msg| store.handle_daemon_message(msg)).await {
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
