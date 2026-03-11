use crate::proxy_v2::ProxyV2State;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Emitter, Listener, Manager, Position, Rect, Runtime, Size, State, WebviewUrl,
    WebviewWindowBuilder,
};

// ─── TrayState: 트레이 ↔ 메인 윈도우 공유 상태 (Rust 백엔드 중개) ─────────

/// 트레이 패널과 메인 윈도우가 Rust 백엔드를 통해 공유하는 상태.
/// `emitTo`/`listen` 없이 `invoke`로만 접근하여 macOS WKWebView 데드락을 방지.
pub struct TrayState {
    /// 프록시를 통해 처리된 총 트랜잭션 수 (이벤트 이미터에서 카운트)
    pub transaction_count: AtomicU64,
    /// 녹화 일시정지 여부
    pub recording_paused: AtomicBool,
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            transaction_count: AtomicU64::new(0),
            recording_paused: AtomicBool::new(false),
        }
    }
}

// ─── 종료 로직 ─────────────────────────────────────────────

/// 앱을 실제로 종료하는 내부 함수
fn do_exit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(panel) = app.get_webview_window("tray-panel") {
        let _ = panel.close();
    }
    app.exit(0);
}

/// 자동 세션 저장 이벤트를 전송하고, 완료 이벤트 수신 시 즉시 종료하는 공통 함수
/// 타임아웃(10초) 내에 완료 이벤트를 받지 못하면 강제 종료
fn graceful_quit<R: Runtime>(app: AppHandle<R>) {
    const AUTOSAVE_TIMEOUT_SECS: u64 = 10;

    // 메인 윈도우에 종료 이벤트 전송하여 자동 세션 저장 트리거
    let _ = app.emit_to("main", "app_quit_requested", ());

    let app_for_listen = app.clone();
    let app_for_timeout = app.clone();

    // autosave 완료 이벤트를 수신하면 즉시 종료
    let handler_id = app.listen("autosave_completed", move |_| {
        tracing::info!("autosave 완료 이벤트 수신, 앱 종료");
        do_exit(&app_for_listen);
    });

    // 타임아웃: 이벤트를 받지 못하면 강제 종료
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(AUTOSAVE_TIMEOUT_SECS));
        tracing::warn!(
            timeout_secs = AUTOSAVE_TIMEOUT_SECS,
            "autosave 완료 타임아웃, 데이터 유실 가능성 있음 — 강제 종료"
        );
        app_for_timeout.unlisten(handler_id);
        do_exit(&app_for_timeout);
    });
}

// ─── 트레이 패널 윈도우 ────────────────────────────────────

const PANEL_WIDTH: f64 = 300.0;
const PANEL_HEIGHT: f64 = 310.0;

/// 트레이 클릭 중 포커스 잃음 이벤트를 무시하기 위한 플래그
static SUPPRESS_FOCUS_LOST: AtomicBool = AtomicBool::new(false);

/// 트레이 Rect에서 패널 위치를 계산.
/// 멀티 모니터 + HiDPI 환경에서도 정확하도록 모든 좌표를 논리(logical) 좌표로 통일.
fn calc_panel_position(tray_rect: &Rect, scale_factor: f64) -> Position {
    let (tray_x, tray_y) = match &tray_rect.position {
        Position::Physical(p) => (p.x as f64 / scale_factor, p.y as f64 / scale_factor),
        Position::Logical(p) => (p.x, p.y),
    };
    let (tray_w, tray_h) = match &tray_rect.size {
        Size::Physical(s) => (
            s.width as f64 / scale_factor,
            s.height as f64 / scale_factor,
        ),
        Size::Logical(s) => (s.width, s.height),
    };

    let x = tray_x + (tray_w / 2.0) - (PANEL_WIDTH / 2.0);
    let y = if cfg!(target_os = "macos") {
        tray_y + tray_h
    } else {
        tray_y - PANEL_HEIGHT
    };

    Position::Logical(tauri::LogicalPosition::new(x, y))
}

/// 트레이 패널 윈도우 토글 (좌클릭)
fn toggle_tray_panel<R: Runtime>(app: &AppHandle<R>, tray_rect: Rect) {
    if let Some(panel) = app.get_webview_window("tray-panel") {
        if panel.is_visible().unwrap_or(false) {
            let _ = panel.hide();
        } else {
            let scale_factor = panel.scale_factor().unwrap_or(1.0);
            let panel_pos = calc_panel_position(&tray_rect, scale_factor);
            SUPPRESS_FOCUS_LOST.store(true, Ordering::Relaxed);
            let _ = panel.set_position(panel_pos);
            let _ = panel.show();
            let _ = panel.set_focus();
            let app_clone = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
                SUPPRESS_FOCUS_LOST.store(false, Ordering::Relaxed);
                if let Some(panel) = app_clone.get_webview_window("tray-panel") {
                    if !panel.is_focused().unwrap_or(true) {
                        let _ = panel.hide();
                    }
                }
            });
        }
    }
}

/// 앱 시작 시 트레이 패널을 hidden 상태로 미리 생성
fn precreate_tray_panel(app: &tauri::App) {
    if let Ok(panel) =
        WebviewWindowBuilder::new(app, "tray-panel", WebviewUrl::App("/tray.html".into()))
            .title("Cheolsu Proxy")
            .inner_size(PANEL_WIDTH, PANEL_HEIGHT)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .closable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .visible(false)
            .build()
    {
        let panel_clone = panel.clone();
        panel.on_window_event(move |event| {
            if let tauri::WindowEvent::Focused(false) = event {
                if !SUPPRESS_FOCUS_LOST.load(Ordering::Relaxed) {
                    let _ = panel_clone.hide();
                }
            }
        });
    }
}

/// 시스템 트레이 설정
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    precreate_tray_panel(app);

    let tray_icon = Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;

    let show_item = MenuItemBuilder::with_id("show", "메인 창 열기").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "종료").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&show_item, &quit_item])
        .build()?;

    let _tray = TrayIconBuilder::new()
        .icon(tray_icon)
        .icon_as_template(true)
        .tooltip("Cheolsu Proxy")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                graceful_quit(app.clone());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                toggle_tray_panel(tray.app_handle(), rect);
            }
        })
        .build(app)?;

    Ok(())
}

// ─── Tauri Commands ────────────────────────────────────────

/// 트레이 패널에 표시할 정보
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrayInfo {
    pub is_connected: bool,
    pub ca_installed: bool,
    pub port: u16,
    pub transaction_count: u64,
    pub recording_paused: bool,
}

/// 트레이 패널에서 호출하는 상태 조회 커맨드
#[tauri::command]
pub async fn tray_get_info(
    proxy: State<'_, ProxyV2State>,
    tray_state: State<'_, Arc<TrayState>>,
) -> Result<TrayInfo, String> {
    let is_connected = proxy.lock().await.is_some();
    let ca_installed = crate::proxy_v2::check_ca_installed().unwrap_or(false);
    let port = 8100;
    let transaction_count = tray_state.transaction_count.load(Ordering::Relaxed);
    let recording_paused = tray_state.recording_paused.load(Ordering::Relaxed);

    Ok(TrayInfo {
        is_connected,
        ca_installed,
        port,
        transaction_count,
        recording_paused,
    })
}

/// 녹화 일시정지 토글 커맨드 (트레이 또는 메인 윈도우에서 호출)
/// Rust 백엔드를 중개자로 사용하여 양쪽 윈도우에 안전하게 전파.
#[tauri::command]
pub fn tray_toggle_recording<R: Runtime>(
    app: AppHandle<R>,
    tray_state: State<'_, Arc<TrayState>>,
) -> Result<bool, String> {
    let prev = tray_state.recording_paused.load(Ordering::Relaxed);
    let new_paused = !prev;
    tray_state
        .recording_paused
        .store(new_paused, Ordering::Relaxed);

    // run_on_main_thread로 안전하게 이벤트 전파 (macOS 데드락 방지)
    let _ = app.emit("recording_paused_changed", new_paused);

    Ok(new_paused)
}

/// 녹화 일시정지 상태를 명시적으로 설정 (메인 윈도우에서 로컬 토글 후 동기화용)
#[tauri::command]
pub fn tray_set_recording_paused(
    tray_state: State<'_, Arc<TrayState>>,
    paused: bool,
) -> Result<(), String> {
    tray_state.recording_paused.store(paused, Ordering::Relaxed);
    Ok(())
}

/// 메인 창 표시 커맨드
#[tauri::command]
pub fn tray_show_main_window<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        Ok(())
    } else {
        Err("메인 윈도우를 찾을 수 없습니다".to_string())
    }
}

/// 앱 완전 종료 커맨드
#[tauri::command]
pub fn tray_quit_app<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    graceful_quit(app);
    Ok(())
}
