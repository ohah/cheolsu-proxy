use proxyapi::Proxy;
use std::net::SocketAddr;
use tokio::sync::oneshot::Sender;
use std::process::Command;

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


pub fn get_active_service() -> Option<String> {
    // 1. 기본 네트워크 인터페이스 이름 가져오기 (en0, en1 등)
    let route_output = Command::new("sh")
        .arg("-c")
        .arg("route get default | grep interface | awk '{print $2}'")
        .output()
        .ok()?;
    let interface = String::from_utf8_lossy(&route_output.stdout).trim().to_string();

    // 2. 인터페이스 -> 서비스 이름 매핑
    let list_output = Command::new("networksetup")
        .arg("-listnetworkserviceorder")
        .output()
        .ok()?;
    let list_str = String::from_utf8_lossy(&list_output.stdout);

    for line in list_str.lines() {
        if line.contains(&interface) {
            if let Some(start) = line.find("Hardware Port: ") {
                let end = line[start + 15..].find(',').unwrap_or(0) + start + 15;
                return Some(line[start + 15..end].to_string());
            }
        }
    }
    None
}

pub fn set_proxy(enable: bool) -> Result<(), String> {
    let service = get_active_service();
    if let Some(service) = service {
        let service = service.as_str();
        if enable {
            // HTTP 프록시 켜기
            Command::new("networksetup")
                .args(["-setwebproxy", service, "127.0.0.1", "8100"])
                .status()
                .map_err(|e| e.to_string())?;

            // HTTPS 프록시 켜기
            Command::new("networksetup")
                .args(["-setsecurewebproxy", service, "127.0.0.1", "8100"])
                .status()
                .map_err(|e| e.to_string())?;
        } else {
            // HTTP 프록시 끄기
            Command::new("networksetup")
                .args(["-setwebproxystate", service, "off"])
                .status()
                .map_err(|e| e.to_string())?;

            // HTTPS 프록시 끄기
            Command::new("networksetup")
                .args(["-setsecurewebproxystate", service, "off"])
                .status()
                .map_err(|e| e.to_string())?;
        }
    }   
    Ok(())
}

