use proxy_v2_models::{WsDirection, WsMessageType};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::format_time;
use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_connections(f, app, chunks[0]);
    draw_messages(f, app, chunks[1]);
}

fn draw_connections(f: &mut Frame, app: &mut App, area: Rect) {
    let rows: Vec<Row> = app
        .ws_connections
        .iter()
        .map(|conn| {
            let status = if conn.active {
                Span::styled("●", Style::default().fg(Color::Green))
            } else {
                Span::styled("●", Style::default().fg(Color::Gray))
            };

            Row::new(vec![
                Cell::from(status),
                Cell::from(truncate(&conn.uri, 40)),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(2), Constraint::Min(10)])
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(50, 60, 140))
                .fg(Color::White),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
                .title(format!(" Connections ({}) ", app.ws_connections.len()))
                .title_bottom(
                    Line::from(" j/k: navigate | y: copy URI | c: clear ")
                        .style(Style::default().fg(Color::DarkGray)),
                ),
        );

    f.render_stateful_widget(table, area, &mut app.ws_conn_table_state);
}

fn draw_messages(f: &mut Frame, app: &App, area: Rect) {
    let selected_conn_id = app
        .selected_ws_conn
        .and_then(|i| app.ws_connections.get(i))
        .map(|c| c.connection_id.clone());

    let filtered: Vec<&proxy_v2_models::WsMessageInfo> = if let Some(ref conn_id) = selected_conn_id
    {
        app.ws_messages
            .iter()
            .filter(|m| m.connection_id == *conn_id)
            .collect()
    } else {
        app.ws_messages.iter().collect()
    };

    let header = Row::new(vec![
        Cell::from("Dir"),
        Cell::from("Type"),
        Cell::from("Time"),
        Cell::from("Size"),
        Cell::from("Payload"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = filtered
        .iter()
        .map(|msg| {
            let dir = match msg.direction {
                WsDirection::ClientToServer => {
                    Cell::from("→").style(Style::default().fg(Color::Cyan))
                }
                WsDirection::ServerToClient => {
                    Cell::from("←").style(Style::default().fg(Color::Green))
                }
            };

            let msg_type = match msg.message_type {
                WsMessageType::Text => "TXT",
                WsMessageType::Binary => "BIN",
                WsMessageType::Ping => "PNG",
                WsMessageType::Pong => "PON",
                WsMessageType::Close => "CLS",
            };

            let payload_preview = truncate(&msg.payload, 60);

            Row::new(vec![
                dir,
                Cell::from(msg_type),
                Cell::from(format_time(msg.time)),
                Cell::from(format!("{}B", msg.size)),
                Cell::from(payload_preview),
            ])
        })
        .collect();

    let title = format!(" Messages ({}) ", filtered.len());

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(title)
            .title_bottom(
                Line::from(" Y: copy messages ").style(Style::default().fg(Color::DarkGray)),
            ),
    );

    f.render_widget(table, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
