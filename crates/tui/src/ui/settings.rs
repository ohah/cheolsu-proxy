use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(area);

    // Proxy info
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

    // Keybindings help
    let help_lines = vec![
        Line::from(Span::styled(
            "Keybindings",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Tab / Shift+Tab    Switch tabs"),
        Line::from("  Alt+1~4            Jump to tab"),
        Line::from("  j / k / ↑ / ↓      Navigate list"),
        Line::from("  Enter              Toggle detail (Network)"),
        Line::from("  Space              Pause/Resume (Network)"),
        Line::from("  y                  Copy URL (Network)"),
        Line::from("  Y                  Copy full detail (Network)"),
        Line::from("  c                  Clear list"),
        Line::from("  a                  Add rule (Rules)"),
        Line::from("  t                  Toggle rule (Rules)"),
        Line::from("  d / Delete         Delete rule (Rules)"),
        Line::from("  C                  Clear all rules (Rules)"),
        Line::from("  g / Home           Jump to top"),
        Line::from("  G / End            Jump to bottom"),
        Line::from("  q / Ctrl+C         Quit"),
    ];

    let help = Paragraph::new(help_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(" Keybindings "),
    );

    f.render_widget(help, chunks[1]);
}
