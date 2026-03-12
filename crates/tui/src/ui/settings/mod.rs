/// Settings 탭 UI - 서브모듈로 분리
mod info;
mod keybindings;
mod list_sections;
mod sections;

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, SettingsSection};

use info::{draw_ca_cert, draw_proxy_info, draw_remote_device_cert, qr_widget_height};
use keybindings::draw_keybindings;
use list_sections::{draw_host_mapping, draw_ssl_proxying};
use sections::{
    draw_client_certificate, draw_proxy_auth, draw_quick_settings, draw_throttle,
    draw_upstream_proxy,
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    // QR 코드 높이를 동적으로 계산
    let primary_ip = app
        .local_ips
        .first()
        .cloned()
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let qr_url = format!("http://{}:{}/ssl", primary_ip, app.port);
    let qr_height = if app.connected {
        // QR 코드 높이와 우측 텍스트 높이(12줄 + 2 border) 중 큰 값 사용
        qr_widget_height(&qr_url).max(14)
    } else {
        5
    };

    // 터미널 높이에 따라 동적 레이아웃 조정
    let fixed_min = 10 + 5 + qr_height + 3 + 12 + 5; // 최소 필요 높이 (keybindings 최소 5줄)
    let (form_height, kb_constraint) = if area.height < fixed_min {
        // 작은 터미널: form을 축소하고 keybindings 최소 보장
        let available = area.height.saturating_sub(10 + 5 + qr_height + 3 + 5);
        (Constraint::Length(available.max(4)), Constraint::Min(5))
    } else {
        (Constraint::Length(12), Constraint::Min(5))
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),        // Proxy info
            Constraint::Length(5),         // CA certificate
            Constraint::Length(qr_height), // Remote device cert (QR code)
            Constraint::Length(3),         // Section tabs
            form_height,                   // Form (upstream or throttle)
            kb_constraint,                 // Keybindings (최소 5줄 보장)
        ])
        .split(area);

    draw_proxy_info(f, app, chunks[0]);
    draw_ca_cert(f, app, chunks[1]);
    draw_remote_device_cert(f, app, chunks[2]);
    draw_section_tabs(f, app, chunks[3]);
    match app.settings_section {
        SettingsSection::UpstreamProxy => draw_upstream_proxy(f, app, chunks[4]),
        SettingsSection::ProxyAuth => draw_proxy_auth(f, app, chunks[4]),
        SettingsSection::Throttle => draw_throttle(f, app, chunks[4]),
        SettingsSection::HostMapping => draw_host_mapping(f, app, chunks[4]),
        SettingsSection::QuickSettings => draw_quick_settings(f, app, chunks[4]),
        SettingsSection::SslProxying => draw_ssl_proxying(f, app, chunks[4]),
        SettingsSection::ClientCertificate => draw_client_certificate(f, app, chunks[4]),
    }
    draw_keybindings(f, app, chunks[5]);
}

/// 섹션 탭 바 그리기
fn draw_section_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tab_item = |section: SettingsSection, label: &str| -> Span {
        if app.settings_section == section {
            Span::styled(
                format!(" \u{25cf} {} ", label),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!(" \u{25cb} {} ", label),
                Style::default().fg(Color::DarkGray),
            )
        }
    };

    let titles: Vec<Line> = vec![
        Line::from(vec![
            tab_item(SettingsSection::UpstreamProxy, "Upstream"),
            Span::raw(" "),
            tab_item(SettingsSection::ProxyAuth, "Auth"),
            Span::raw(" "),
            tab_item(SettingsSection::Throttle, "Throttle"),
            Span::raw(" "),
            tab_item(SettingsSection::HostMapping, "Mapping"),
            Span::raw(" "),
            tab_item(SettingsSection::QuickSettings, "Quick"),
            Span::raw(" "),
            tab_item(SettingsSection::SslProxying, "SSL"),
            Span::raw(" "),
            tab_item(SettingsSection::ClientCertificate, "Cert"),
        ]),
        Line::from(Span::styled(
            "  h/l: switch section",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));

    let paragraph = Paragraph::new(titles).block(block);
    f.render_widget(paragraph, area);
}

/// 텍스트 필드 렌더링 헬퍼 (다른 서브모듈에서도 사용)
pub(super) fn render_text_field<'a>(value: &'a str, is_editing: bool) -> Span<'a> {
    if is_editing {
        Span::styled(
            format!("{}█", value),
            Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 50)),
        )
    } else if value.is_empty() {
        Span::styled("(empty)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(value, Style::default().fg(Color::White))
    }
}
