use ratatui::prelude::*;
use ratatui::widgets::*;

use super::{format_size, format_time};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    if app.session_save_editing {
        draw_session_input(f, app, area, true);
        return;
    }

    if app.session_load_editing {
        draw_session_input(f, app, area, false);
        return;
    }

    if app.show_diff && app.diff_result.is_some() {
        draw_diff_view(f, app, area);
    } else if app.show_detail && app.selected_transaction.is_some() {
        // Full-screen detail view
        draw_transaction_detail(f, app, area);
    } else {
        draw_transaction_list(f, app, area);
    }
}

fn draw_transaction_list(f: &mut Frame, app: &mut App, area: Rect) {
    let header = Row::new(vec![
        Cell::from(" "),
        Cell::from("Time"),
        Cell::from("Method"),
        Cell::from("URL"),
        Cell::from("Status"),
        Cell::from("Size"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

    let rows: Vec<Row> = app
        .transactions
        .iter()
        .enumerate()
        .map(|(i, info)| {
            let (method, uri, time, status, size) = extract_transaction_info(info);
            let status_style = status_color(status);

            let mark = if app.diff_mark == Some(i) {
                Cell::from("D").style(
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Cell::from(" ")
            };

            Row::new(vec![
                mark,
                Cell::from(format_time(time)),
                Cell::from(method).style(Style::default().fg(Color::Cyan)),
                Cell::from(uri),
                Cell::from(status.to_string()).style(status_style),
                Cell::from(format_size(size)),
            ])
        })
        .collect();

    let diff_hint = if app.diff_mark.is_some() {
        " [D: diff target marked] "
    } else {
        ""
    };
    let title = format!(
        " Network ({}) {}{}",
        app.transactions.len(),
        if app.paused { "[PAUSED] " } else { "" },
        diff_hint,
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(1),
            Constraint::Length(8),
            Constraint::Length(7),
            Constraint::Min(30),
            Constraint::Length(6),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(title)
            .title_bottom(Line::from(
                " j/k: nav | Enter: detail | D: diff | y: URL | c: cURL | r: replay | e: export | S: save | L: load | Space: pause | x: clear "
            ).style(Style::default().fg(Color::Cyan))),
    )
    .row_highlight_style(Style::default().bg(Color::Rgb(50, 60, 140)).fg(Color::White));

    f.render_stateful_widget(table, area, &mut app.network_table_state);
}

fn draw_diff_view(f: &mut Frame, app: &mut App, area: Rect) {
    let text = app.diff_result.as_deref().unwrap_or("No diff result");
    let lines: Vec<Line> = text
        .lines()
        .map(|line| {
            let style = if line.starts_with("+ ") {
                Style::default().fg(Color::Green)
            } else if line.starts_with("- ") {
                Style::default().fg(Color::Red)
            } else if line.starts_with("~ ") {
                Style::default().fg(Color::Yellow)
            } else if line.starts_with("## ") || line.starts_with("### ") {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if line.starts_with("Method:")
                || line.starts_with("URL:")
                || line.starts_with("Status:")
            {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(line.to_string(), style))
        })
        .collect();

    let visible_height = area.height.saturating_sub(2) as u16;
    let total_lines = lines.len() as u16;
    let max_scroll = total_lines.saturating_sub(visible_height);
    app.diff_scroll = app.diff_scroll.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta))
                .title(" Diff Result ")
                .title_bottom(
                    Line::from(" j/k: scroll | g: top | Esc: close ")
                        .style(Style::default().fg(Color::Cyan)),
                ),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.diff_scroll, 0));

    f.render_widget(paragraph, area);
}

fn draw_transaction_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(idx) = app.selected_transaction else {
        return;
    };
    let Some(info) = app.transactions.get(idx) else {
        return;
    };

    let mut lines = Vec::new();

    // Request info
    if let Some(req) = &info.0 {
        lines.push(Line::from(Span::styled(
            "── Request ──",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::styled(
                req.method().to_string(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(req.uri().to_string(), Style::default().fg(Color::White)),
        ]));
        lines.push(key_value_line("Version", &format!("{:?}", req.version())));
        lines.push(key_value_line("Type", &format!("{:?}", req.data_type())));
        lines.push(key_value_line("Size", &format_size(req.body_size())));
        lines.push(Line::from(""));

        // Headers
        lines.push(Line::from(Span::styled(
            "Headers:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        for (name, value) in req.headers().iter() {
            lines.push(header_line(
                name.as_str(),
                value.to_str().unwrap_or("<binary>"),
            ));
        }
        lines.push(Line::from(""));
    }

    // Response info
    if let Some(res) = &info.1 {
        lines.push(Line::from(Span::styled(
            "── Response ──",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        let status = res.status().as_u16();
        let status_color = match status {
            200..=299 => Color::Green,
            300..=399 => Color::Yellow,
            400..=499 => Color::Red,
            500..=599 => Color::Magenta,
            _ => Color::Gray,
        };
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                status.to_string(),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(key_value_line("Type", &format!("{:?}", res.data_type())));
        lines.push(key_value_line("Size", &format_size(res.body_size())));
        lines.push(Line::from(""));

        // Headers
        lines.push(Line::from(Span::styled(
            "Headers:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        for (name, value) in res.headers().iter() {
            lines.push(header_line(
                name.as_str(),
                value.to_str().unwrap_or("<binary>"),
            ));
        }
    }

    // 표시 영역 높이 (border 2줄 제외)
    let visible_height = area.height.saturating_sub(2) as u16;
    let total_lines = lines.len() as u16;
    let max_scroll = total_lines.saturating_sub(visible_height);
    app.detail_scroll = app.detail_scroll.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
                .title(" Detail ")
                .title_bottom(
                    Line::from(
                        " j/k: scroll | g: top | y: URL | c: cURL | r: replay | Esc/Enter: back ",
                    )
                    .style(Style::default().fg(Color::Cyan)),
                ),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));

    f.render_widget(paragraph, area);
}

fn extract_transaction_info(
    info: &proxy_v2_models::RequestInfo,
) -> (String, String, i64, u16, usize) {
    let method = info
        .0
        .as_ref()
        .map(|r| r.method().to_string())
        .unwrap_or_default();
    let uri = info
        .0
        .as_ref()
        .map(|r| r.uri().to_string())
        .unwrap_or_default();
    let time = info.0.as_ref().map(|r| r.time()).unwrap_or(0);
    let status = info.1.as_ref().map(|r| r.status().as_u16()).unwrap_or(0);
    let size = info.1.as_ref().map(|r| r.body_size()).unwrap_or(0);

    (method, uri, time, status, size)
}

fn status_color(status: u16) -> Style {
    match status {
        0 => Style::default().fg(Color::Gray),
        200..=299 => Style::default().fg(Color::Green),
        300..=399 => Style::default().fg(Color::Yellow),
        400..=499 => Style::default().fg(Color::Red),
        500..=599 => Style::default().fg(Color::Magenta),
        _ => Style::default(),
    }
}

fn key_value_line(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{}: ", key), Style::default().fg(Color::Cyan)),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

fn header_line(name: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{}: ", name), Style::default().fg(Color::Blue)),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

fn draw_session_input(f: &mut Frame, app: &App, area: Rect, is_save: bool) {
    let (title, path) = if is_save {
        (" Save Session ", &app.session_save_path_input)
    } else {
        (" Load Session ", &app.session_load_path_input)
    };

    let hint = if is_save {
        "Enter file path (.cheolsu / .cheolsu.gz)"
    } else {
        "Enter file path to load"
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            hint,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}_", path),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            if is_save {
                format!("({} transactions will be saved)", app.transactions.len())
            } else {
                "Transactions will replace current data".to_string()
            },
            Style::default().fg(Color::Gray),
        )),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(title)
            .title_bottom(
                Line::from(" Enter: confirm | Esc: cancel ")
                    .style(Style::default().fg(Color::Cyan)),
            ),
    );

    f.render_widget(paragraph, area);
}
