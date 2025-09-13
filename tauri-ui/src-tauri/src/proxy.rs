use proxyapi::Proxy;
use std::process::Command;
use std::{env, net::SocketAddr};
use tauri_plugin_store::StoreExt;
use tokio::sync::oneshot::Sender;

use tauri::{async_runtime::Mutex, AppHandle, Emitter, Runtime, State};

use proxyapi_models::RequestInfo;

pub type ProxyState = Mutex<Option<(Sender<()>, tauri::async_runtime::JoinHandle<()>, Proxy)>>;

#[tauri::command]
pub async fn start_proxy<R: Runtime>(
    app: AppHandle<R>,
    proxy: State<'_, ProxyState>,
    addr: SocketAddr,
) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let (close_tx, close_rx) = tokio::sync::oneshot::channel();

    let store = app.store("session.json").map_err(|e| e.to_string())?;
    let sessions = store.get("sessions").unwrap_or_default();

    let proxy_server = Proxy::new(addr, Some(tx.clone()), sessions);

    let proxy_server_clone = proxy_server.clone();

    let thread = tauri::async_runtime::spawn(async move {
        if let Err(e) = proxy_server_clone
            .start(async move {
                let _ = close_rx.await;
            })
            .await
        {
            eprintln!("Error running proxy on {:?}: {e}", addr);
        }
    });

    let mut proxy = proxy.lock().await;
    proxy.replace((close_tx, thread, proxy_server));

    tauri::async_runtime::spawn(async move {
        for exchange in rx.iter() {
            let (request, response) = exchange.to_parts();
            app.emit("proxy_event", RequestInfo(request, response))
                .unwrap();
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

#[tauri::command]
pub async fn store_changed<R: Runtime>(
    app: AppHandle<R>,
    proxy: State<'_, ProxyState>,
) -> Result<(), String> {
    let mut proxy = proxy.lock().await;

    if proxy.is_none() {
        println!("store_changed: Proxy is not running, ignoring session update");
        return Ok(());
    }

    let store = app.store("session.json").map_err(|e| e.to_string())?;
    let sessions = store.get("sessions").unwrap_or_default();

    proxy.as_mut().unwrap().2.update_sessions(sessions);
    Ok(())
}

#[tauri::command]
pub async fn get_proxy_status_command() -> Result<ProxyStatus, String> {
    get_proxy_status()
}

pub fn get_active_service() -> Option<String> {
    // 1. 기본 네트워크 인터페이스 이름 가져오기 (en0, en1 등)
    let route_output = Command::new("sh")
        .arg("-c")
        .arg("route get default | grep interface | awk '{print $2}'")
        .output()
        .ok()?;
    let interface = String::from_utf8_lossy(&route_output.stdout)
        .trim()
        .to_string();

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
    let is_proxy = env::var("IS_PROXY").unwrap_or_else(|_| "true".to_string());
    // NOTE: IS_PROXY 환경변수가 없으면 프록시 설정 안함
    if is_proxy == "false" {
        return Ok(());
    }

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

            // WebSocket 연결 오류가 발생한 도메인들을 제외 처리
            // TLS 서명 알고리즘 확장 문제가 있는 도메인들을 제외
            let bypass_domains = "*.pusher.com,*.pusherapp.com,*.amazonaws.com,*.icloud.com,*.apple.com,localhost,127.0.0.1";

            // WebSocket 포트(443, 80)를 사용하는 특정 도메인들을 추가로 제외
            // localhost와 127.0.0.1은 모든 포트가 자동으로 제외됨
            let bypass_with_ports = "ws-ap3.pusher.com:443,ws-ap3.pusher.com:80,gateway.icloud.com:443,gateway.icloud.com:80";

            // 모든 제외 도메인을 하나로 합쳐서 설정
            let all_bypass = format!("{},{}", bypass_domains, bypass_with_ports);
            Command::new("networksetup")
                .args(["-setproxybypassdomains", service, &all_bypass])
                .status()
                .map_err(|e| e.to_string())?;

            println!("✅ 프록시 설정 완료 - HTTP, HTTPS 프록시 활성화됨");
            println!("   🔌 WebSocket 오류 도메인 제외: {}", bypass_domains);
            println!("   🔌 WebSocket 포트별 제외: {}", bypass_with_ports);
            println!("   🏠 localhost, 127.0.0.1: 모든 포트 제외됨");
            println!("   💡 WebSocket 연결은 직접 연결로 처리됨");
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

            // 프록시 제외 도메인도 정리
            Command::new("networksetup")
                .args(["-setproxybypassdomains", service, ""])
                .status()
                .map_err(|e| e.to_string())?;

            println!("✅ 프록시 설정 해제 완료 - HTTP, HTTPS 프록시 비활성화됨");
            println!("   🔌 WebSocket 제외 도메인도 정리됨");
        }
    }
    Ok(())
}

/// 현재 프록시 설정 상태 확인
pub fn get_proxy_status() -> Result<ProxyStatus, String> {
    let service = get_active_service();
    if let Some(service) = service {
        let service = service.as_str();

        // HTTP 프록시 상태 확인
        let http_output = Command::new("networksetup")
            .args(["-getwebproxy", service])
            .output()
            .map_err(|e| e.to_string())?;

        // HTTPS 프록시 상태 확인
        let https_output = Command::new("networksetup")
            .args(["-getsecurewebproxy", service])
            .output()
            .map_err(|e| e.to_string())?;

        // SOCKS 프록시 상태 확인
        let socks_output = Command::new("networksetup")
            .args(["-getsocksfirewallproxy", service])
            .output()
            .map_err(|e| e.to_string())?;

        let http_enabled = String::from_utf8_lossy(&http_output.stdout).contains("Enabled: Yes");
        let https_enabled = String::from_utf8_lossy(&https_output.stdout).contains("Enabled: Yes");
        let socks_enabled = String::from_utf8_lossy(&socks_output.stdout).contains("Enabled: Yes");

        Ok(ProxyStatus {
            http: http_enabled,
            https: https_enabled,
            websocket: socks_enabled,
        })
    } else {
        Err("활성 네트워크 서비스를 찾을 수 없습니다".to_string())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyStatus {
    pub http: bool,
    pub https: bool,
    pub websocket: bool,
}
