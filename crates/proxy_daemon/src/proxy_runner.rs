use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, watch, Semaphore};
use tracing::info;

use crate::breakpoint::BreakpointManager;
use crate::error::DaemonError;
use crate::handler::{LoggingHandler, QuickSettings, SseEvent, WsEvent};
use crate::protocol::{
    BreakpointRule, ClientCertConfig, DaemonMessage, HostMapping, InterceptRule,
    RequestClientCertConfig, ServerReplayEntry, SslProxyingEntry, SslProxyingMode,
};
use crate::tls_client::create_hybrid_client_with_cert;
use proxyapi_v2::certificate_authority::{CertificateAuthority, ClientCertVerifyConfig};
use proxyapi_v2::throttle::ThrottleConfig;
use proxyapi_v2::upstream_proxy::UpstreamProxyConfig;
use proxyapi_v2::websocket_registry::WebSocketRegistry;

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
    mut ssl_proxying_rx: watch::Receiver<(SslProxyingMode, Vec<SslProxyingEntry>)>,
    client_cert_rx: watch::Receiver<Option<ClientCertConfig>>,
    ws_registry: WebSocketRegistry,
    script_handle: scripting::ScriptHandle,
    quick_settings: std::sync::Arc<tokio::sync::RwLock<QuickSettings>>,
    proxy_auth: std::sync::Arc<tokio::sync::RwLock<Option<crate::protocol::ProxyAuthConfig>>>,
    shutdown_signal: tokio::sync::oneshot::Receiver<()>,
    max_concurrent_connections: Option<usize>,
    max_body_size: Option<usize>,
    tls_passthrough: proxyapi_v2::tls_passthrough::TlsPassthrough,
    request_client_cert_rx: watch::Receiver<Option<RequestClientCertConfig>>,
    connection_strategy: std::sync::Arc<std::sync::atomic::AtomicU8>,
    metrics_collector: std::sync::Arc<proxyapi_v2::metrics::MetricsCollector>,
    total_transactions: std::sync::Arc<std::sync::atomic::AtomicU64>,
    mut default_passthrough_rx: watch::Receiver<Vec<SslProxyingEntry>>,
) -> Result<(), DaemonError> {
    use proxyapi_v2::builder::ProxyBuilder;
    use proxyapi_v2::certificate_authority::{
        build_ca, generate_session_hash, get_cache_storage_dir,
    };
    use tokio::net::TcpListener;

    let ca = build_ca().map_err(|e| DaemonError::Proxy(format!("CA build failed: {}", e)))?;

    // request_client_cert 초기값 적용
    {
        let initial_config = request_client_cert_rx.borrow().clone();
        if let Some(ref config) = initial_config {
            let verify_config = convert_request_client_cert_config(config).await;
            ca.set_client_cert_verify(Some(verify_config)).await;
        }
    }

    // request_client_cert 설정 변경 감시
    // client_cert_verify + cache 핸들을 함께 사용하여 캐시 무효화도 수행
    // 변환을 먼저 수행하고 성공 시에만 lock을 획득하여, 변환 실패가 lock 상태를 오염시키지 않도록 함
    let client_cert_verify_handle = ca.get_client_cert_verify_handle();
    let cache_handle = ca.get_cache_handle();
    let mut req_cert_rx = request_client_cert_rx;
    tokio::spawn(async move {
        while req_cert_rx.changed().await.is_ok() {
            let config = req_cert_rx.borrow().clone();
            match config {
                Some(ref cfg) => {
                    // 변환을 먼저 수행 — 파일 읽기/파싱 실패 시 lock은 오염되지 않음
                    let verify_config = convert_request_client_cert_config(cfg).await;
                    *client_cert_verify_handle.write().await = Some(verify_config);
                    cache_handle.invalidate_all();
                    info!(
                        "Request client cert 설정 업데이트됨: enabled={}, required={}",
                        cfg.enabled, cfg.required
                    );
                }
                None => {
                    *client_cert_verify_handle.write().await = None;
                    cache_handle.invalidate_all();
                    info!("Request client cert 설정 해제됨");
                }
            }
        }
    });

    let session_hash = generate_session_hash();
    let cache_dir = get_cache_storage_dir(&session_hash)
        .map_err(|e| DaemonError::Proxy(format!("Cache dir failed: {}", e)))?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<proxy_v2_models::RequestInfo>(256);
    let (tunnel_tx, mut tunnel_rx) =
        tokio::sync::mpsc::channel::<proxy_v2_models::RequestInfo>(256);

    let (ws_tx, mut ws_rx) = tokio::sync::mpsc::channel::<WsEvent>(256);
    let (sse_tx, mut sse_rx) = tokio::sync::mpsc::channel::<SseEvent>(256);

    let ca_cert_der = ca.get_ca_cert_der().unwrap_or_default();

    let handler = LoggingHandler::new(tx.clone(), cache_dir)
        .with_ws_sender(ws_tx)
        .with_sse_sender(sse_tx)
        .with_script_handle(script_handle)
        .with_ca_cert_der(ca_cert_der)
        .with_breakpoint_manager(breakpoint_manager.clone())
        .with_quick_settings(quick_settings)
        .with_proxy_auth(proxy_auth)
        .with_max_body_size(max_body_size);

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

    // SSL Proxying 초기값 로드 및 감시
    {
        let (mode, entries) = ssl_proxying_rx.borrow().clone();
        handler.update_ssl_proxying(mode, entries).await;
    }

    let handler_for_ssl_updates = handler.clone();
    tokio::spawn(async move {
        while ssl_proxying_rx.changed().await.is_ok() {
            let (mode, entries) = ssl_proxying_rx.borrow().clone();
            handler_for_ssl_updates
                .update_ssl_proxying(mode, entries)
                .await;
        }
    });

    // 기본 패스스루 도메인 초기값 로드 및 감시
    {
        let entries = default_passthrough_rx.borrow().clone();
        handler.update_default_passthrough(entries).await;
    }

    let handler_for_passthrough_updates = handler.clone();
    tokio::spawn(async move {
        while default_passthrough_rx.changed().await.is_ok() {
            let entries = default_passthrough_rx.borrow().clone();
            handler_for_passthrough_updates
                .update_default_passthrough(entries)
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

    let throttle_rx_arc = std::sync::Arc::new(throttle_rx);

    // 동시 연결 수 제한 세마포어 생성
    let connection_semaphore = max_concurrent_connections.map(|max| Arc::new(Semaphore::new(max)));

    // Detached 태스크(passthrough 등)의 graceful shutdown을 위한 채널
    let (passthrough_shutdown_tx, passthrough_shutdown_rx) = watch::channel(false);

    let proxy_ctx = proxyapi_v2::ProxyContext {
        tunnel_event_sender: Some(tunnel_tx),
        tls_passthrough: Some(tls_passthrough),
        websocket_registry: Some(ws_registry),
        upstream_proxy: initial_upstream,
        throttle_rx: Some(throttle_rx_arc),
        connection_semaphore,
        connection_strategy: Some(connection_strategy),
        metrics: Some(metrics_collector),
        shutdown_rx: Some(passthrough_shutdown_rx),
        ..Default::default()
    };

    let proxy_builder = ProxyBuilder::new()
        .with_listener(listener)
        .with_ca(ca)
        .with_client(hybrid_client)
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler.clone())
        .with_graceful_shutdown(async move {
            let _ = shutdown_signal.await;
            // 모든 detached passthrough 태스크에 종료 신호 전송
            let _ = passthrough_shutdown_tx.send(true);
        })
        .with_proxy_context(proxy_ctx)
        .build()
        .map_err(|e| DaemonError::Proxy(format!("Proxy build failed: {}", e)))?;

    info!("Proxy listening on {}", addr);

    let event_tx_http = event_tx.clone();
    let tx_counter_http = total_transactions.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&DaemonMessage::Event { data: event }) {
                let _ = event_tx_http.send(msg);
                tx_counter_http.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    });

    let event_tx_tunnel = event_tx.clone();
    let tx_counter_tunnel = total_transactions;
    tokio::spawn(async move {
        while let Some(tunnel_event) = tunnel_rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&DaemonMessage::Event { data: tunnel_event }) {
                let _ = event_tx_tunnel.send(msg);
                tx_counter_tunnel.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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

    let event_tx_sse = event_tx.clone();
    tokio::spawn(async move {
        while let Some(sse_event) = sse_rx.recv().await {
            let msg = match sse_event {
                SseEvent::Event(info) => {
                    serde_json::to_string(&DaemonMessage::SseEvent { data: info })
                }
                SseEvent::Connection(event) => {
                    serde_json::to_string(&DaemonMessage::SseConnection { data: event })
                }
            };
            if let Ok(msg) = msg {
                let _ = event_tx_sse.send(msg);
            }
        }
    });

    proxy_builder
        .start()
        .await
        .map_err(|e| DaemonError::Proxy(format!("Proxy start failed: {}", e)))?;

    Ok(())
}

/// RequestClientCertConfig (프로토콜 레벨) → ClientCertVerifyConfig (CA 레벨) 변환
async fn convert_request_client_cert_config(
    config: &RequestClientCertConfig,
) -> ClientCertVerifyConfig {
    use tokio_rustls::rustls::pki_types::CertificateDer;

    let mut ca_certs = Vec::new();

    if let Some(ref ca_cert_path) = config.ca_cert_path {
        match tokio::fs::read(ca_cert_path).await {
            Ok(pem_data) => {
                // PEM 형식의 CA 인증서를 DER로 변환
                let mut reader = std::io::BufReader::new(pem_data.as_slice());
                let certs = rustls_pemfile::certs(&mut reader);
                for cert_result in certs {
                    match cert_result {
                        Ok(cert) => {
                            ca_certs.push(CertificateDer::from(cert.to_vec()));
                        }
                        Err(e) => {
                            tracing::warn!("CA 인증서 파싱 실패 ({}): {}", ca_cert_path, e);
                        }
                    }
                }
                if ca_certs.is_empty() {
                    // PEM이 아닌 DER 형식일 수 있음
                    ca_certs.push(CertificateDer::from(pem_data));
                    tracing::info!("CA 인증서를 DER 형식으로 로드: {}", ca_cert_path);
                } else {
                    tracing::info!("CA 인증서 {} 개 로드: {}", ca_certs.len(), ca_cert_path);
                }
            }
            Err(e) => {
                tracing::error!("CA 인증서 파일 읽기 실패 ({}): {}", ca_cert_path, e);
            }
        }
    }

    ClientCertVerifyConfig {
        enabled: config.enabled,
        ca_certs,
        required: config.required,
    }
}
