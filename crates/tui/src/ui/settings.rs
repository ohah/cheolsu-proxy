use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, SettingsSection, ThrottleField, ThrottlePresetChoice, UpstreamProxyField};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Proxy info
            Constraint::Length(5),  // CA certificate
            Constraint::Length(7),  // Remote device cert
            Constraint::Length(3),  // Section tabs
            Constraint::Length(12), // Form (upstream or throttle)
            Constraint::Min(0),     // Keybindings
        ])
        .split(area);

    draw_proxy_info(f, app, chunks[0]);
    draw_ca_cert(f, app, chunks[1]);
    draw_remote_device_cert(f, app, chunks[2]);
    draw_section_tabs(f, app, chunks[3]);
    match app.settings_section {
        SettingsSection::UpstreamProxy => draw_upstream_proxy(f, app, chunks[4]),
        SettingsSection::Throttle => draw_throttle(f, app, chunks[4]),
    }
    draw_keybindings(f, app, chunks[5]);
}

fn draw_section_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = vec![
        Line::from(vec![
            if app.settings_section == SettingsSection::UpstreamProxy {
                Span::styled(
                    " ● Upstream Proxy ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(" ○ Upstream Proxy ", Style::default().fg(Color::DarkGray))
            },
            Span::raw("  "),
            if app.settings_section == SettingsSection::Throttle {
                Span::styled(
                    " ● Throttle ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(" ○ Throttle ", Style::default().fg(Color::DarkGray))
            },
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

fn draw_ca_cert(f: &mut Frame, app: &App, area: Rect) {
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

fn draw_remote_device_cert(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![];

    if app.connected {
        let primary_ip = app
            .local_ips
            .first()
            .cloned()
            .unwrap_or_else(|| "127.0.0.1".to_string());

        lines.push(Line::from(vec![
            Span::styled("Proxy:    ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{}:{}", primary_ip, app.port),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Cert URL: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "http://cheolsu.proxy/ssl",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        if app.local_ips.len() > 1 {
            let extra_ips: Vec<String> = app.local_ips[1..]
                .iter()
                .map(|ip| format!("{}:{}", ip, app.port))
                .collect();
            lines.push(Line::from(vec![
                Span::styled("Also:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(extra_ips.join(", "), Style::default().fg(Color::DarkGray)),
            ]));
        }

        lines.push(Line::from(Span::styled(
            "  1) Set Wi-Fi proxy on device  2) Open URL  3) Install cert",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Start proxy to see remote device setup info",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray))
        .title(" Remote Device Certificate ");

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
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

fn draw_throttle(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.throttle_form;

    let fields: Vec<Line> = ThrottleField::ALL
        .iter()
        .map(|field| {
            let is_selected =
                *field == form.field && app.settings_section == SettingsSection::Throttle;
            let is_editing = is_selected && form.editing;

            let label = Span::styled(
                format!("  {:<16} ", field.label()),
                if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            );

            let value = match field {
                ThrottleField::Enabled => {
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
                ThrottleField::Preset => Span::styled(
                    format!("◀ {} ▶", form.preset.label()),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                ThrottleField::Download => render_text_field(&form.download, is_editing),
                ThrottleField::Upload => render_text_field(&form.upload, is_editing),
                ThrottleField::Latency => render_text_field(&form.latency, is_editing),
            };

            let cursor = if is_selected {
                Span::styled("▶ ", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("  ")
            };

            // Custom이 아닌 프리셋일 때 Download/Upload/Latency 필드는 흐리게
            let dim = matches!(
                field,
                ThrottleField::Download | ThrottleField::Upload | ThrottleField::Latency
            ) && form.preset != ThrottlePresetChoice::Custom;

            if dim {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("  {:<16} ", field.label()),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("—", Style::default().fg(Color::DarkGray)),
                ])
            } else {
                Line::from(vec![cursor, label, value])
            }
        })
        .collect();

    let title = if form.enabled {
        format!(" Throttle [ON: {}] ", form.preset.label())
    } else {
        " Throttle [OFF] ".to_string()
    };

    let border_color = if form.enabled {
        Color::Magenta
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
    let editing = app.upstream_form.editing || app.throttle_form.editing;

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
            Line::from("  i                  Install CA certificate"),
            Line::from("  U                  Uninstall CA certificate"),
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
