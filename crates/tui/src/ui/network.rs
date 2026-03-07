use ratatui::prelude::*;
use ratatui::widgets::*;

use super::{format_size, format_time};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    if app.show_detail && app.selected_transaction.is_some() {
        // Split view: list + detail
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        draw_transaction_list(f, app, chunks[0]);
        draw_transaction_detail(f, app, chunks[1]);
    } else {
        draw_transaction_list(f, app, area);
    }
}

fn draw_transaction_list(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
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

            let selected = app.selected_transaction == Some(i);
            let style = if selected {
                Style::default().bg(Color::Rgb(40, 40, 60))
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format_time(time)),
                Cell::from(method).style(Style::default().fg(Color::Cyan)),
                Cell::from(truncate_str(&uri, 60)),
                Cell::from(status.to_string()).style(status_style),
                Cell::from(format_size(size)),
            ])
            .style(style)
        })
        .collect();

    let title = format!(
        " Network ({}) {}",
        app.transactions.len(),
        if app.paused { "[PAUSED]" } else { "" }
    );

    let table = Table::new(
        rows,
        [
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
            .title(title),
    )
    .row_highlight_style(Style::default().bg(Color::Rgb(40, 40, 60)));

    f.render_widget(table, area);
}

fn draw_transaction_detail(f: &mut Frame, app: &App, area: Rect) {
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
        lines.push(Line::from(format!("{} {}", req.method(), req.uri())));
        lines.push(Line::from(format!("Version: {:?}", req.version())));
        lines.push(Line::from(format!("Type: {:?}", req.data_type())));
        lines.push(Line::from(format!(
            "Size: {}",
            format_size(req.body_size())
        )));
        lines.push(Line::from(""));

        // Headers
        lines.push(Line::from(Span::styled(
            "Headers:",
            Style::default().fg(Color::Yellow),
        )));
        for (name, value) in req.headers().iter() {
            lines.push(Line::from(format!(
                "  {}: {}",
                name,
                value.to_str().unwrap_or("<binary>")
            )));
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
        lines.push(Line::from(format!("Status: {}", res.status())));
        lines.push(Line::from(format!("Type: {:?}", res.data_type())));
        lines.push(Line::from(format!(
            "Size: {}",
            format_size(res.body_size())
        )));
        lines.push(Line::from(""));

        // Headers
        lines.push(Line::from(Span::styled(
            "Headers:",
            Style::default().fg(Color::Yellow),
        )));
        for (name, value) in res.headers().iter() {
            lines.push(Line::from(format!(
                "  {}: {}",
                name,
                value.to_str().unwrap_or("<binary>")
            )));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
                .title(" Detail "),
        )
        .wrap(Wrap { trim: false })
        .scroll((0, 0));

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

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
