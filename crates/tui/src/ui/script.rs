use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Max(7), // Script status (최대 7줄, 작은 화면에서 축소)
            Constraint::Min(3), // Console logs (최소 3줄 보장)
            Constraint::Max(5), // Keybindings (최대 5줄, 작은 화면에서 축소)
        ])
        .split(area);

    draw_script_status(f, app, chunks[0]);
    draw_console_logs(f, app, chunks[1]);
    draw_keybindings(f, app, chunks[2]);
}

fn draw_script_status(f: &mut Frame, app: &App, area: Rect) {
    let (status_text, status_color) = if app.script_active {
        ("ACTIVE", Color::Green)
    } else {
        ("INACTIVE", Color::DarkGray)
    };

    let path_display = if app.script_editing {
        Line::from(vec![
            Span::styled("  Path:   ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{}█", app.script_path_input),
                Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 50)),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("  Path:   ", Style::default().fg(Color::Yellow)),
            Span::raw(app.script_path.as_deref().unwrap_or("(no script loaded)")),
        ])
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("  Status: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                status_text,
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        path_display,
        Line::from(""),
        Line::from(vec![
            Span::styled("  Logs:   ", Style::default().fg(Color::Yellow)),
            Span::raw(app.script_logs.len().to_string()),
        ]),
    ];

    let border_color = if app.script_active {
        Color::Green
    } else {
        Color::Gray
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" Script [{}] ", status_text));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_console_logs(f: &mut Frame, app: &App, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;

    let log_lines: Vec<Line> = app
        .script_logs
        .iter()
        .skip(app.script_log_scroll)
        .take(inner_height)
        .map(|entry| {
            let level_style = match entry.level.as_str() {
                "error" => Style::default().fg(Color::Red),
                "warn" => Style::default().fg(Color::Yellow),
                "debug" => Style::default().fg(Color::DarkGray),
                _ => Style::default().fg(Color::Cyan),
            };

            let level_tag = match entry.level.as_str() {
                "error" => "[ERR]",
                "warn" => "[WRN]",
                "debug" => "[DBG]",
                _ => "[LOG]",
            };

            Line::from(vec![
                Span::styled(format!("{} ", level_tag), level_style),
                Span::raw(&entry.message),
            ])
        })
        .collect();

    let title = if app.script_logs.is_empty() {
        " Console ".to_string()
    } else {
        format!(
            " Console ({}/{}) ",
            app.script_log_scroll + 1,
            app.script_logs.len()
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray))
        .title(title);

    let paragraph = Paragraph::new(log_lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_keybindings(f: &mut Frame, app: &App, area: Rect) {
    let help_lines = if app.script_editing {
        vec![
            Line::from("  Enter          Load script"),
            Line::from("  Esc            Cancel"),
        ]
    } else {
        vec![
            Line::from("  l: load script | u: unload | r: reload | c: clear logs"),
            Line::from("  j/k: scroll logs | g/G: top/bottom"),
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
