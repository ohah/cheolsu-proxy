use crate::proxy_v2::ProxyV2State;
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Manager, Position, Rect, Runtime, Size, State, WebviewUrl, WebviewWindowBuilder,
};

/// Position에서 (x, y) f64 추출
fn position_to_f64(pos: &Position) -> (f64, f64) {
    match pos {
        Position::Physical(p) => (p.x as f64, p.y as f64),
        Position::Logical(p) => (p.x, p.y),
    }
}

/// Size에서 (w, h) f64 추출
fn size_to_f64(size: &Size) -> (f64, f64) {
    match size {
        Size::Physical(s) => (s.width as f64, s.height as f64),
        Size::Logical(s) => (s.width, s.height),
    }
}

const PANEL_WIDTH: f64 = 300.0;
const PANEL_HEIGHT: f64 = 380.0;

/// 트레이 패널 윈도우 토글 (좌클릭)
fn toggle_tray_panel<R: Runtime>(app: &AppHandle<R>, tray_rect: Rect) {
    let (rect_x, rect_y) = position_to_f64(&tray_rect.position);
    let (rect_w, rect_h) = size_to_f64(&tray_rect.size);

    // 트레이 아이콘 중앙 기준으로 패널 위치 계산
    let x = rect_x + (rect_w / 2.0) - (PANEL_WIDTH / 2.0);
    let y = if cfg!(target_os = "macos") {
        rect_y + rect_h
    } else {
        rect_y - PANEL_HEIGHT
    };

    if let Some(panel) = app.get_webview_window("tray-panel") {
        if panel.is_visible().unwrap_or(false) {
            let _ = panel.hide();
        } else {
            // 위치 업데이트 후 표시
            let _ = panel.set_position(tauri::LogicalPosition::new(x, y));
            let _ = panel.show();
            let _ = panel.set_focus();
        }
    }
}

/// 시스템 트레이 설정
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // 트레이 전용 아이콘 로드
    let tray_icon = Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;

    // 우클릭 컨텍스트 메뉴
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
                if let Some(panel) = app.get_webview_window("tray-panel") {
                    let _ = panel.close();
                }
                app.exit(0);
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

    // 트레이 패널 윈도우를 미리 생성 (숨김 상태, 투명 배경)
    let panel = WebviewWindowBuilder::new(app, "tray-panel", WebviewUrl::App("/tray.html".into()))
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
        .build()?;

    // 패널이 포커스를 잃으면 숨기기 (닫지 않고 재사용)
    let panel_clone = panel.clone();
    panel.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            let _ = panel_clone.hide();
        }
    });

    Ok(())
}

/// 트레이 패널에 표시할 정보
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrayInfo {
    pub is_connected: bool,
    pub ca_installed: bool,
    pub port: u16,
}

/// 트레이 패널에서 호출하는 상태 조회 커맨드
#[tauri::command]
pub async fn tray_get_info(proxy: State<'_, ProxyV2State>) -> Result<TrayInfo, String> {
    let is_connected = proxy.lock().await.is_some();
    let ca_installed = crate::proxy_v2::check_ca_installed().unwrap_or(false);
    let port = 8100;

    Ok(TrayInfo {
        is_connected,
        ca_installed,
        port,
    })
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
    if let Some(panel) = app.get_webview_window("tray-panel") {
        let _ = panel.close();
    }
    app.exit(0);
    Ok(())
}
