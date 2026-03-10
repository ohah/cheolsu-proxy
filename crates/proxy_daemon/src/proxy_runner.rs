use std::net::SocketAddr;
use tokio::sync::{broadcast, watch};
use tracing::info;

use crate::breakpoint::BreakpointManager;
use crate::error::DaemonError;
use crate::handler::{LoggingHandler, QuickSettings, WsEvent};
use crate::protocol::{
    BreakpointRule, ClientCertConfig, DaemonMessage, HostMapping, InterceptRule, ServerReplayEntry,
    SslProxyingEntry,
};
use crate::tls_client::create_hybrid_client_with_cert;
use proxyapi_v2::certificate_authority::CertificateAuthority;
use proxyapi_v2::throttle::ThrottleConfig;
use proxyapi_v2::upstream_proxy::UpstreamProxyConfig;
use proxyapi_v2::websocket_registry::WebSocketRegistry;

use super::daemon::app_support_dir;

pub async fn run_proxy(
    addr: SocketAddr,
    event_tx: broadcast::Sender<String>,
    mut intercept_rx: watch::Receiver<Vec<InterceptRule>>,
    upstream_rx: watch::Receiver<Option<UpstreamProxyConfig>>,
    mut server_replay_rx: watch::Receiver<Vec<ServerReplayEntry>>,
    throttle_rx: watch::Receiver<Option<ThrottleConfig>>,
    mut breakpoint_rx: watch::Receiver<Vec<BreakpointRule>>,
    breakpoint_manager: BreakpointManager,
    mut host_mapping_rx: watch::Receiver<Vec<HostMapping>>,
    mut ssl_proxying_rx: watch::Receiver<Vec<SslProxyingEntry>>,
    client_cert_rx: watch::Receiver<Option<ClientCertConfig>>,
    ws_registry: WebSocketRegistry,
    script_handle: scripting::ScriptHandle,
    quick_settings: std::sync::Arc<tokio::sync::RwLock<QuickSettings>>,
    proxy_auth: std::sync::Arc<parking_lot::RwLock<Option<crate::protocol::ProxyAuthConfig>>>,
    shutdown_signal: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), DaemonError> {
    use proxyapi_v2::builder::ProxyBuilder;
    use proxyapi_v2::certificate_authority::{
        build_ca, generate_session_hash, get_cache_storage_dir,
    };
    use tokio::net::TcpListener;

    let ca = build_ca().map_err(|e| DaemonError::Proxy(format!("CA build failed: {}", e)))?;

    let session_hash = generate_session_hash();
    let cache_dir = get_cache_storage_dir(&session_hash)
        .map_err(|e| DaemonError::Proxy(format!("Cache dir failed: {}", e)))?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<proxy_v2_models::RequestInfo>(256);
    let (tunnel_tx, mut tunnel_rx) =
        tokio::sync::mpsc::channel::<proxy_v2_models::RequestInfo>(256);

    let (ws_tx, mut ws_rx) = tokio::sync::mpsc::channel::<WsEvent>(256);

    let ca_cert_der = ca.get_ca_cert_der().unwrap_or_default();

    let handler = LoggingHandler::new(tx.clone(), cache_dir)
        .with_ws_sender(ws_tx)
        .with_script_handle(script_handle)
        .with_ca_cert_der(ca_cert_der)
        .with_breakpoint_manager(breakpoint_manager.clone())
        .with_quick_settings(quick_settings)
        .with_proxy_auth(proxy_auth);

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

    // Breakpoint rules initial load + watch
    {
        let rules = breakpoint_rx.borrow().clone();
        breakpoint_manager.update_rules(rules).await;
    }
    let bp_mgr_for_updates = breakpoint_manager.clone();
    tokio::spawn(async move {
        while breakpoint_rx.changed().await.is_ok() {
            let rules = breakpoint_rx.borrow().clone();
            bp_mgr_for_updates.update_rules(rules).await;
        }
    });

    // Host mapping initial load and watcher
    {
        let mappings = host_mapping_rx.borrow().clone();
        handler.update_host_mappings(mappings).await;
    }

    let handler_for_mapping_updates = handler.clone();
    tokio::spawn(async move {
        while host_mapping_rx.changed().await.is_ok() {
            let mappings = host_mapping_rx.borrow().clone();
            handler_for_mapping_updates
                .update_host_mappings(mappings)
                .await;
        }
    });

    // SSL Proxying 화이트리스트 초기값 로드 및 감시
    {
        let entries = ssl_proxying_rx.borrow().clone();
        handler.update_ssl_proxying_entries(entries).await;
    }

    let handler_for_ssl_updates = handler.clone();
    tokio::spawn(async move {
        while ssl_proxying_rx.changed().await.is_ok() {
            let entries = ssl_proxying_rx.borrow().clone();
            handler_for_ssl_updates
                .update_ssl_proxying_entries(entries)
                .await;
        }
    });

    let initial_upstream = upstream_rx.borrow().clone();
    let initial_client_cert = client_cert_rx.borrow().clone();

    let hybrid_client =
        create_hybrid_client_with_cert(upstream_rx, initial_client_cert.as_ref())
            .map_err(|e| DaemonError::Proxy(format!("Client creation failed: {}", e)))?;

    // 클라이언트 인증서 변경 감시 - 변경 시 클라이언트에 알림 전송
    // rustls ClientConfig는 빌드 시 고정되므로 인증서 변경 시 프록시 재시작 필요
    let event_tx_cert = event_tx.clone();
    tokio::spawn(async move {
        let mut cert_rx = client_cert_rx;
        while cert_rx.changed().await.is_ok() {
            tracing::warn!(
                "클라이언트 인증서 설정이 변경되었습니다. 변경 사항을 적용하려면 프록시를 재시작해야 합니다."
            );
            if let Ok(msg) = serde_json::to_string(&DaemonMessage::ScriptLog {
                level: "warn".to_string(),
                message: "클라이언트 인증서 설정이 변경되었습니다. 프록시를 재시작해야 적용됩니다."
                    .to_string(),
            }) {
                let _ = event_tx_cert.send(msg);
            }
        }
    });

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| DaemonError::Proxy(format!("Port {} bind failed: {}", addr.port(), e)))?;

    // TLS 자동 학습 바이패스 초기화
    let passthrough_path = app_support_dir()
        .ok()
        .map(|dir| dir.join("tls_passthrough.json"));
    let tls_passthrough = proxyapi_v2::tls_passthrough::TlsPassthrough::new(passthrough_path);

    let throttle_rx_arc = std::sync::Arc::new(throttle_rx);

    let proxy_ctx = proxyapi_v2::ProxyContext {
        tunnel_event_sender: Some(tunnel_tx),
        tls_passthrough: Some(tls_passthrough),
        websocket_registry: Some(ws_registry),
        upstream_proxy: initial_upstream,
        throttle_rx: Some(throttle_rx_arc),
        ..Default::default()
    };

    let proxy_builder = ProxyBuilder::new()
        .with_listener(listener)
        .with_ca(ca)
        .with_client(hybrid_client)
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler.clone())
        .with_graceful_shutdown(async {
            let _ = shutdown_signal.await;
        })
        .with_proxy_context(proxy_ctx)
        .build()
        .map_err(|e| DaemonError::Proxy(format!("Proxy build failed: {}", e)))?;

    info!("Proxy listening on {}", addr);

    let event_tx_http = event_tx.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&DaemonMessage::Event { data: event }) {
                let _ = event_tx_http.send(msg);
            }
        }
    });

    let event_tx_tunnel = event_tx.clone();
    tokio::spawn(async move {
        while let Some(tunnel_event) = tunnel_rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&DaemonMessage::Event { data: tunnel_event }) {
                let _ = event_tx_tunnel.send(msg);
            }
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
        .map_err(|e| DaemonError::Proxy(format!("Proxy start failed: {}", e)))?;

    Ok(())
}
