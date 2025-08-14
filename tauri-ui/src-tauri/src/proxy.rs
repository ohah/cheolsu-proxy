use proxyapi::Proxy;
use std::net::SocketAddr;
use tokio::sync::oneshot::Sender;

use tauri::{
    async_runtime::Mutex,
    AppHandle, Runtime, State, Emitter,
};

use proxyapi_models::RequestInfo;

pub type ProxyState = Mutex<Option<(Sender<()>, tauri::async_runtime::JoinHandle<()>)>>;

#[tauri::command]
pub async fn start_proxy<R: Runtime>(
    app: AppHandle<R>,
    proxy: State<'_, ProxyState>,
    addr: SocketAddr,
) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let (close_tx, close_rx) = tokio::sync::oneshot::channel();
    let thread = tauri::async_runtime::spawn(async move {
        if let Err(e) = Proxy::new(addr, Some(tx.clone()))
            .start(async move {
                let _ = close_rx.await;
            })
            .await
        {
            eprintln!("Error running proxy on {:?}: {e}", addr);
        }
    });

    let mut proxy = proxy.lock().await;
    proxy.replace((close_tx, thread));

    tauri::async_runtime::spawn(async move {
        for exchange in rx.iter() {
            let (request, response) = exchange.to_parts();
            app.emit("proxy_event", RequestInfo(request, response)).unwrap();
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_proxy(proxy: State<'_, ProxyState>) -> Result<(), String> {
    let mut proxy = proxy.lock().await;
    assert!(proxy.is_some());
    proxy.take();
    Ok(())
}

#[tauri::command]
pub async fn proxy_status(proxy: State<'_, ProxyState>) -> Result<bool, String> {
    Ok(proxy.lock().await.is_some())
}

