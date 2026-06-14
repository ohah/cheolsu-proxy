use std::env;
use std::process::Command;
use std::sync::Mutex;
use tracing::info;

use crate::error::DaemonError;

/// 프록시 활성화 직전의 시스템 프록시 설정(해제 시 복원용).
static PREVIOUS_PROXY: Mutex<Option<PreviousProxy>> = Mutex::new(None);

#[derive(Clone, Default)]
struct ProxySetting {
    enabled: bool,
    server: String,
    port: String,
}

#[derive(Clone, Default)]
struct PreviousProxy {
    web: ProxySetting,
    secure: ProxySetting,
}

/// networksetup -getwebproxy/-getsecurewebproxy 출력에서 현재 프록시 설정을 읽는다.
fn read_proxy_setting(service: &str, get_kind: &str) -> ProxySetting {
    let mut s = ProxySetting::default();
    if let Ok(out) = Command::new("networksetup")
        .args([get_kind, service])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("Enabled: ") {
                s.enabled = v.trim().eq_ignore_ascii_case("yes");
            } else if let Some(v) = line.strip_prefix("Server: ") {
                s.server = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Port: ") {
                s.port = v.trim().to_string();
            }
        }
    }
    s
}

/// 저장된 프록시 설정을 복원한다(활성화돼 있었으면 서버/포트 복원, 아니면 off).
fn restore_proxy_setting(service: &str, set_kind: &str, state_kind: &str, s: &ProxySetting) {
    if s.enabled && !s.server.is_empty() {
        let _ = Command::new("networksetup")
            .args([set_kind, service, &s.server, &s.port])
            .status();
        let _ = Command::new("networksetup")
            .args([state_kind, service, "on"])
            .status();
    } else {
        let _ = Command::new("networksetup")
            .args([state_kind, service, "off"])
            .status();
    }
}

/// 현재 활성 네트워크 서비스 이름 가져오기 (macOS)
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

/// macOS 시스템 프록시 설정/해제
pub fn set_proxy(enable: bool, port: u16) -> Result<(), DaemonError> {
    let is_proxy = env::var("IS_PROXY").unwrap_or_else(|_| "true".to_string());
    // NOTE: IS_PROXY 환경변수가 없으면 프록시 설정 안함
    if is_proxy == "false" {
        return Ok(());
    }

    let service = get_active_service();
    if let Some(service) = service {
        let service = service.as_str();
        if enable {
            let port_str = port.to_string();

            // cheolsu 프록시로 덮어쓰기 전에 기존 설정을 저장(해제 시 복원용).
            // 이미 cheolsu(127.0.0.1)가 설정된 경우(재활성화)는 자기 자신을 저장하지 않는다.
            let prev_web = read_proxy_setting(service, "-getwebproxy");
            let prev_secure = read_proxy_setting(service, "-getsecurewebproxy");
            if prev_web.server != "127.0.0.1" {
                *PREVIOUS_PROXY.lock().unwrap_or_else(|e| e.into_inner()) = Some(PreviousProxy {
                    web: prev_web,
                    secure: prev_secure,
                });
            }

            // HTTP 프록시 켜기
            Command::new("networksetup")
                .args(["-setwebproxy", service, "127.0.0.1", &port_str])
                .status()
                .map_err(DaemonError::Io)?;

            // HTTPS 프록시 켜기
            Command::new("networksetup")
                .args(["-setsecurewebproxy", service, "127.0.0.1", &port_str])
                .status()
                .map_err(DaemonError::Io)?;

            info!("프록시 설정 완료 - HTTP, HTTPS 프록시 활성화됨");
            info!("   프록시 주소: 127.0.0.1:{}", port);
        } else {
            // 이전 설정이 저장돼 있으면 복원하고, 없으면 off로 둔다.
            // (과거엔 무조건 off로 만들어 cheolsu 시작 전 사용자가 쓰던 프록시를 잃어버렸다)
            let prev = PREVIOUS_PROXY
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take();
            if let Some(prev) = prev {
                restore_proxy_setting(service, "-setwebproxy", "-setwebproxystate", &prev.web);
                restore_proxy_setting(
                    service,
                    "-setsecurewebproxy",
                    "-setsecurewebproxystate",
                    &prev.secure,
                );
                info!("프록시 해제 - 이전 시스템 프록시 설정 복원 완료");
            } else {
                // HTTP 프록시 끄기
                Command::new("networksetup")
                    .args(["-setwebproxystate", service, "off"])
                    .status()
                    .map_err(DaemonError::Io)?;

                // HTTPS 프록시 끄기
                Command::new("networksetup")
                    .args(["-setsecurewebproxystate", service, "off"])
                    .status()
                    .map_err(DaemonError::Io)?;

                info!("프록시 설정 해제 완료 - HTTP, HTTPS 프록시 비활성화됨");
            }
        }
    }
    Ok(())
}

/// 현재 프록시 설정 상태 확인
pub fn get_proxy_status() -> Result<ProxyStatus, DaemonError> {
    let service = get_active_service();
    if let Some(service) = service {
        let service = service.as_str();

        // HTTP 프록시 상태 확인
        let http_output = Command::new("networksetup")
            .args(["-getwebproxy", service])
            .output()
            .map_err(DaemonError::Io)?;

        // HTTPS 프록시 상태 확인
        let https_output = Command::new("networksetup")
            .args(["-getsecurewebproxy", service])
            .output()
            .map_err(DaemonError::Io)?;

        // SOCKS 프록시 상태 확인
        let socks_output = Command::new("networksetup")
            .args(["-getsocksfirewallproxy", service])
            .output()
            .map_err(DaemonError::Io)?;

        let http_enabled = String::from_utf8_lossy(&http_output.stdout).contains("Enabled: Yes");
        let https_enabled = String::from_utf8_lossy(&https_output.stdout).contains("Enabled: Yes");
        let socks_enabled = String::from_utf8_lossy(&socks_output.stdout).contains("Enabled: Yes");

        Ok(ProxyStatus {
            http: http_enabled,
            https: https_enabled,
            websocket: socks_enabled,
        })
    } else {
        Err(DaemonError::Daemon(
            "활성 네트워크 서비스를 찾을 수 없습니다".to_string(),
        ))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyStatus {
    pub http: bool,
    pub https: bool,
    pub websocket: bool,
}
