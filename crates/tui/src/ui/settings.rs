use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(area);

    // 프록시 정보
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

    f.render_widget(info, chunks[0]);

    // 키 바인딩 도움말
    let help_lines = vec![
        Line::from(Span::styled(
            "키 바인딩",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Tab / Shift+Tab    탭 전환"),
        Line::from("  Alt+1~4            탭 직접 이동"),
        Line::from("  j / k / ↑ / ↓      목록 이동"),
        Line::from("  Enter              상세보기 토글 (Network)"),
        Line::from("  Space              일시정지/재개 (Network)"),
        Line::from("  c                  목록 초기화"),
        Line::from("  t                  규칙 토글 (Rules)"),
        Line::from("  d / Delete         규칙 삭제 (Rules)"),
        Line::from("  C                  전체 규칙 삭제 (Rules)"),
        Line::from("  g / Home           맨 위로"),
        Line::from("  G / End            맨 아래로"),
        Line::from("  q / Ctrl+C         종료"),
    ];

    let help = Paragraph::new(help_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(" Keybindings "),
    );

    f.render_widget(help, chunks[1]);
}
