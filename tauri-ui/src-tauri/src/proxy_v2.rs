use std::net::SocketAddr;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

/// hudsucker를 사용하는 프록시 상태
pub struct ProxyV2State(Arc<Mutex<bool>>);

impl Default for ProxyV2State {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(false)))
    }
}

impl ProxyV2State {
    pub async fn set_running(&self, running: bool) {
        let mut guard = self.0.lock().await;
        *guard = running;
    }

    pub async fn get_running(&self) -> bool {
        let guard = self.0.lock().await;
        *guard
    }
}

/// hudsucker 프록시 시작 (현재는 상태만 변경)
#[tauri::command]
pub async fn start_proxy_v2(app: tauri::AppHandle, _addr: SocketAddr) -> Result<(), String> {
    // 프록시 상태를 true로 설정
    let proxy_state: tauri::State<ProxyV2State> = app.state();
    proxy_state.set_running(true).await;

    // TODO: 실제 hudsucker 프록시 구현
    println!("프록시 V2 시작됨 (상태만 변경)");

    Ok(())
}

/// hudsucker 프록시 중지
#[tauri::command]
pub async fn stop_proxy_v2(app: tauri::AppHandle) -> Result<(), String> {
    let proxy_state: tauri::State<ProxyV2State> = app.state();
    proxy_state.set_running(false).await;

    println!("프록시 V2 중지됨");
    Ok(())
}

/// hudsucker 프록시 상태 확인
#[tauri::command]
pub async fn proxy_v2_status(app: tauri::AppHandle) -> Result<bool, String> {
    let proxy_state: tauri::State<ProxyV2State> = app.state();
    Ok(proxy_state.get_running().await)
}
