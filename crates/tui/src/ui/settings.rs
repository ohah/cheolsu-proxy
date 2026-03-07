use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, UpstreamProxyField};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Proxy info
            Constraint::Length(12), // Upstream proxy
            Constraint::Min(0),     // Keybindings
        ])
        .split(area);

    draw_proxy_info(f, app, chunks[0]);
    draw_upstream_proxy(f, app, chunks[1]);
    draw_keybindings(f, app, chunks[2]);
}

fn draw_proxy_info(f: &mut Frame, app: &App, area: Rect) {
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

fn draw_upstream_proxy(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.upstream_form;

    let fields: Vec<Line> = UpstreamProxyField::ALL
        .iter()
        .map(|field| {
            let is_selected = *field == form.field;
            let is_editing = is_selected && form.editing;

            let label = Span::styled(
                format!("  {:<10} ", field.label()),
                if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            );

            let value = match field {
                UpstreamProxyField::Enabled => {
                    let (text, color) = if form.enabled {
                        ("ON", Color::Green)
                    } else {
                        ("OFF", Color::Red)
                    };
                    Span::styled(
                        text,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )
                }
                UpstreamProxyField::Host => render_text_field(&form.host, is_editing),
                UpstreamProxyField::Port => render_text_field(&form.port, is_editing),
                UpstreamProxyField::Username => render_text_field(&form.username, is_editing),
                UpstreamProxyField::Password => {
                    if form.password.is_empty() {
                        render_text_field("", is_editing)
                    } else if is_editing {
                        render_text_field(&form.password, true)
                    } else {
                        Span::styled(
                            "*".repeat(form.password.len()),
                            Style::default().fg(Color::White),
                        )
                    }
                }
                UpstreamProxyField::Bypass => render_text_field(&form.bypass, is_editing),
            };

            let cursor = if is_selected {
                Span::styled("▶ ", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("  ")
            };

            Line::from(vec![cursor, label, value])
        })
        .collect();

    let title = if form.enabled {
        " Upstream Proxy [ON] "
    } else {
        " Upstream Proxy [OFF] "
    };

    let border_color = if form.enabled {
        Color::Green
    } else {
        Color::Gray
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let paragraph = Paragraph::new(fields).block(block);
    f.render_widget(paragraph, area);
}

fn render_text_field<'a>(value: &'a str, is_editing: bool) -> Span<'a> {
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

fn draw_keybindings(f: &mut Frame, app: &App, area: Rect) {
    let editing = app.upstream_form.editing;

    let help_lines = if editing {
        vec![
            Line::from(Span::styled(
                "Editing",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  Enter          Apply & exit edit"),
            Line::from("  Esc            Cancel edit"),
            Line::from("  Type           Input text"),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "Keybindings",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  j / k / ↑ / ↓      Navigate fields"),
            Line::from("  Enter / Space      Toggle (Enabled) / Edit (text fields)"),
            Line::from("  Tab / Shift+Tab    Switch tabs"),
            Line::from("  q / Ctrl+C         Quit"),
        ]
    };

    let help = Paragraph::new(help_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(" Keybindings "),
    );

    f.render_widget(help, area);
}
