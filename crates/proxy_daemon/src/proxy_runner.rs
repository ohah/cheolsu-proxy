use std::net::SocketAddr;
use tokio::sync::{broadcast, watch};
use tracing::info;

use crate::handler::{LoggingHandler, WsEvent};
use crate::protocol::{DaemonMessage, InterceptRule, ServerReplayEntry};
use crate::tls_client::create_hybrid_client;
use proxyapi_v2::upstream_proxy::UpstreamProxyConfig;
use proxyapi_v2::websocket_registry::WebSocketRegistry;

use super::daemon::app_support_dir;

pub async fn run_proxy(
    addr: SocketAddr,
    event_tx: broadcast::Sender<String>,
    mut intercept_rx: watch::Receiver<Vec<InterceptRule>>,
    upstream_rx: watch::Receiver<Option<UpstreamProxyConfig>>,
    mut server_replay_rx: watch::Receiver<Vec<ServerReplayEntry>>,
    ws_registry: WebSocketRegistry,
    script_handle: scripting::ScriptHandle,
) -> Result<(), String> {
    use proxyapi_v2::builder::ProxyBuilder;
    use proxyapi_v2::certificate_authority::{
        build_ca, generate_session_hash, get_cache_storage_dir,
    };
    use tokio::net::TcpListener;

    let ca = build_ca().map_err(|e| format!("CA build failed: {}", e))?;

    let session_hash = generate_session_hash();
    let cache_dir =
        get_cache_storage_dir(&session_hash).map_err(|e| format!("Cache dir failed: {}", e))?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<proxy_v2_models::RequestInfo>(256);
    let (tunnel_tx, mut tunnel_rx) =
        tokio::sync::mpsc::channel::<proxy_v2_models::RequestInfo>(100);

    let (ws_tx, mut ws_rx) = tokio::sync::mpsc::channel::<WsEvent>(256);

    let handler = LoggingHandler::new(tx.clone(), cache_dir)
        .with_ws_sender(ws_tx)
        .with_script_handle(script_handle);

    // 인터셉트 규칙 초기값 로드
    {
        let rules = intercept_rx.borrow().clone();
        handler.update_intercept_rules(rules).await;
    }

    let handler_for_intercept_updates = handler.clone();
    tokio::spawn(async move {
        while intercept_rx.changed().await.is_ok() {
            let rules = intercept_rx.borrow().clone();
            handler_for_intercept_updates
                .update_intercept_rules(rules)
                .await;
        }
    });

    // 서버 리플레이 엔트리 초기값 로드
    {
        let entries = server_replay_rx.borrow().clone();
        handler.update_server_replay_entries(entries).await;
    }

    let handler_for_replay_updates = handler.clone();
    tokio::spawn(async move {
        while server_replay_rx.changed().await.is_ok() {
            let entries = server_replay_rx.borrow().clone();
            handler_for_replay_updates
                .update_server_replay_entries(entries)
                .await;
        }
    });

    let initial_upstream = upstream_rx.borrow().clone();

    let hybrid_client =
        create_hybrid_client(upstream_rx).map_err(|e| format!("Client creation failed: {}", e))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Port {} bind failed: {}", addr.port(), e))?;

    // TLS 자동 학습 바이패스 초기화
    let passthrough_path = app_support_dir()
        .ok()
        .map(|dir| dir.join("tls_passthrough.json"));
    let tls_passthrough = proxyapi_v2::tls_passthrough::TlsPassthrough::new(passthrough_path);

    let proxy_ctx = proxyapi_v2::ProxyContext {
        tunnel_event_sender: Some(tunnel_tx),
        tls_passthrough: Some(tls_passthrough),
        websocket_registry: Some(ws_registry),
        upstream_proxy: initial_upstream,
        ..Default::default()
    };

    let proxy_builder = ProxyBuilder::new()
        .with_listener(listener)
        .with_ca(ca)
        .with_client(hybrid_client)
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler.clone())
        .with_proxy_context(proxy_ctx)
        .build()
        .map_err(|e| format!("Proxy build failed: {}", e))?;

    info!("Proxy listening on {}", addr);

    let event_tx_http = event_tx.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let json = serde_json::to_string(&event).unwrap_or_default();
            let msg = serde_json::to_string(&DaemonMessage::Event { data: event }).unwrap_or(json);
            let _ = event_tx_http.send(msg);
        }
    });

    let event_tx_tunnel = event_tx.clone();
    tokio::spawn(async move {
        while let Some(tunnel_event) = tunnel_rx.recv().await {
            let msg = serde_json::to_string(&DaemonMessage::Event { data: tunnel_event })
                .unwrap_or_default();
            let _ = event_tx_tunnel.send(msg);
        }
    });

    let event_tx_ws = event_tx.clone();
    tokio::spawn(async move {
        while let Some(ws_event) = ws_rx.recv().await {
            let msg = match ws_event {
                WsEvent::Message(info) => {
                    serde_json::to_string(&DaemonMessage::WsMessage { data: info })
                }
                WsEvent::Connection(event) => {
                    serde_json::to_string(&DaemonMessage::WsConnection { data: event })
                }
            };
            if let Ok(msg) = msg {
                let _ = event_tx_ws.send(msg);
            }
        }
    });

    proxy_builder
        .start()
        .await
        .map_err(|e| format!("Proxy start failed: {}", e))?;

    Ok(())
}
