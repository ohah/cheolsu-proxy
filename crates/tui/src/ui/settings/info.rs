/// Settings 탭 - 프록시 정보, CA 인증서, 원격 디바이스 인증서 (QR 코드) 렌더링
use qrcode::QrCode;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub(super) fn draw_proxy_info(f: &mut Frame, app: &App, area: Rect) {
    let info_lines = vec![
        Line::from(vec![
            Span::styled("Host: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.host),
        ]),
        Line::from(vec![
            Span::styled("Port: ", Style::default().fg(Color::Yellow)),
            Span::raw(app.port.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),
            if app.connected {
                Span::styled("Connected", Style::default().fg(Color::Green))
            } else {
                Span::styled("Disconnected", Style::default().fg(Color::Red))
            },
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Transactions: ", Style::default().fg(Color::Yellow)),
            Span::raw(app.transactions.len().to_string()),
        ]),
        Line::from(vec![
            Span::styled("WS Connections: ", Style::default().fg(Color::Yellow)),
            Span::raw(app.ws_connections.len().to_string()),
        ]),
        Line::from(vec![
            Span::styled("Rules: ", Style::default().fg(Color::Yellow)),
            Span::raw(app.rules.len().to_string()),
        ]),
    ];

    let info = Paragraph::new(info_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(" Proxy Info "),
    );

    f.render_widget(info, area);
}

pub(super) fn draw_ca_cert(f: &mut Frame, app: &App, area: Rect) {
    let status = if app.ca_cert_path.is_some() {
        if app.ca_cert_installed {
            Span::styled(
                " Trusted ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                " Not Trusted ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        }
    } else {
        Span::styled(" Not Generated ", Style::default().fg(Color::DarkGray))
    };

    let path_text = app
        .ca_cert_path
        .as_deref()
        .unwrap_or("Start proxy to generate");

    let lines = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),
            status,
        ]),
        Line::from(vec![
            Span::styled("Path:   ", Style::default().fg(Color::Yellow)),
            Span::styled(path_text, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(Span::styled(
            "  i: Install   U: Uninstall",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray))
        .title(" CA Certificate ");

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

pub(super) fn draw_remote_device_cert(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray))
        .title(" Remote Device Certificate ");

    if !app.connected {
        let lines = vec![Line::from(Span::styled(
            "Start proxy to see remote device setup info",
            Style::default().fg(Color::DarkGray),
        ))];
        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let primary_ip = app
        .local_ips
        .first()
        .cloned()
        .unwrap_or_else(|| "127.0.0.1".to_string());

    let qr_url = format!("http://{}:{}/ssl", primary_ip, app.port);

    // 내부 영역 계산 (border 제외)
    let inner = block.inner(area);

    // 좌측: QR 코드, 우측: 정보 텍스트
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(qr_widget_width(&qr_url)),
            Constraint::Min(30),
        ])
        .split(inner);

    f.render_widget(block, area);

    // QR 코드 렌더링
    render_qr_code(f, &qr_url, h_chunks[0]);

    // 우측 정보 텍스트
    let mut info_lines = vec![
        Line::from(vec![
            Span::styled("Proxy:    ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{}:{}", primary_ip, app.port),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Cert URL: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "http://cheolsu.proxy/ssl",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Direct:   ", Style::default().fg(Color::Yellow)),
            Span::styled(
                &qr_url,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    if app.local_ips.len() > 1 {
        let extra_ips: Vec<String> = app.local_ips[1..]
            .iter()
            .map(|ip| format!("{}:{}", ip, app.port))
            .collect();
        info_lines.push(Line::from(vec![
            Span::styled("Also:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(extra_ips.join(", "), Style::default().fg(Color::DarkGray)),
        ]));
    }

    info_lines.push(Line::from(""));
    info_lines.push(Line::from(Span::styled(
        "1) Set Wi-Fi proxy on device",
        Style::default().fg(Color::DarkGray),
    )));
    info_lines.push(Line::from(Span::styled(
        "2) Scan QR or open URL in mobile browser",
        Style::default().fg(Color::DarkGray),
    )));
    info_lines.push(Line::from(Span::styled(
        "3) Install & trust certificate",
        Style::default().fg(Color::DarkGray),
    )));
    info_lines.push(Line::from(""));
    info_lines.push(Line::from(vec![
        Span::styled("Downloads: ", Style::default().fg(Color::Yellow)),
        Span::styled("/ssl/pem", Style::default().fg(Color::Cyan)),
        Span::styled(" (iOS)  ", Style::default().fg(Color::DarkGray)),
        Span::styled("/ssl/der", Style::default().fg(Color::Cyan)),
        Span::styled(" (Android)  ", Style::default().fg(Color::DarkGray)),
        Span::styled("/ssl/ca.crt", Style::default().fg(Color::Cyan)),
    ]));
    info_lines.push(Line::from(Span::styled(
        "iOS: .pem / Android: .der — auto-detected by device",
        Style::default().fg(Color::DarkGray),
    )));

    let info_paragraph = Paragraph::new(info_lines);
    f.render_widget(info_paragraph, h_chunks[1]);
}

/// QR 코드의 모듈 수를 반환합니다.
fn qr_modules(data: &str) -> Option<usize> {
    QrCode::with_error_correction_level(data, qrcode::EcLevel::H)
        .ok()
        .map(|code| code.width())
}

/// QR 코드 위젯의 필요 너비를 계산합니다 (반블록 문자 사용 시 width = modules + 4 quiet zone + 1 spacing).
fn qr_widget_width(data: &str) -> u16 {
    qr_modules(data).map_or(0, |w| (w + 4 + 1) as u16)
}

/// QR 코드 위젯의 필요 높이를 계산합니다 (반블록: 2행->1줄, +2 border).
pub(super) fn qr_widget_height(data: &str) -> u16 {
    qr_modules(data).map_or(5, |w| {
        let total = w + 4; // quiet zone 포함
        let lines = (total + 1) / 2; // 반블록이므로 2행->1줄
        (lines + 2) as u16 // +2 for border
    })
}

/// QR 코드를 유니코드 반블록 문자로 렌더링합니다.
fn render_qr_code(f: &mut Frame, data: &str, area: Rect) {
    let Ok(code) = QrCode::with_error_correction_level(data, qrcode::EcLevel::H) else {
        return;
    };

    let width = code.width();
    // quiet zone 포함한 전체 크기
    let total = width + 4; // 2 quiet zone on each side

    // QR 모듈 데이터를 2D bool 배열로 변환 (quiet zone 포함)
    let mut modules = vec![vec![false; total]; total];
    for y in 0..width {
        for x in 0..width {
            use qrcode::Color as QrColor;
            if code[(x, y)] == QrColor::Dark {
                modules[y + 2][x + 2] = true;
            }
        }
    }

    // 반블록 문자로 렌더링: 2행을 1터미널 행으로
    let dark_color = Color::White;
    let light_color = Color::Black;

    let mut lines: Vec<Line> = Vec::new();
    let mut y = 0;
    while y < total {
        let mut spans: Vec<Span> = Vec::new();
        for x in 0..total {
            let top = modules.get(y).map_or(false, |row| row[x]);
            let bottom = modules.get(y + 1).map_or(false, |row| row[x]);

            match (top, bottom) {
                (true, true) => {
                    spans.push(Span::styled("█", Style::default().fg(dark_color)));
                }
                (true, false) => {
                    spans.push(Span::styled(
                        "▀",
                        Style::default().fg(dark_color).bg(light_color),
                    ));
                }
                (false, true) => {
                    spans.push(Span::styled(
                        "▄",
                        Style::default().fg(dark_color).bg(light_color),
                    ));
                }
                (false, false) => {
                    spans.push(Span::styled(" ", Style::default().bg(light_color)));
                }
            }
        }
        lines.push(Line::from(spans));
        y += 2;
    }

    let qr_paragraph = Paragraph::new(lines);
    f.render_widget(qr_paragraph, area);
}
