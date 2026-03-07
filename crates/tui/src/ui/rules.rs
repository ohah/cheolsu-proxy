use proxy_daemon::InterceptAction;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    draw_rules_list(f, app, chunks[0]);
    draw_rules_help(f, chunks[1]);
}

fn draw_rules_list(f: &mut Frame, app: &App, area: Rect) {
    if app.rules.is_empty() {
        let empty = Paragraph::new("  규칙이 없습니다")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Intercept Rules "),
            );
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("Name"),
        Cell::from("Pattern"),
        Cell::from("Method"),
        Cell::from("Action"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .rules
        .iter()
        .enumerate()
        .map(|(i, rule)| {
            let enabled = if rule.enabled {
                Cell::from("✓").style(Style::default().fg(Color::Green))
            } else {
                Cell::from("✗").style(Style::default().fg(Color::Red))
            };

            let action = match &rule.action {
                InterceptAction::Block { status_code, .. } => {
                    format!("Block({})", status_code)
                }
                InterceptAction::ModifyRequest { .. } => "ModifyReq".to_string(),
                InterceptAction::ModifyResponse { set_status, .. } => {
                    if let Some(s) = set_status {
                        format!("ModifyRes({})", s)
                    } else {
                        "ModifyRes".to_string()
                    }
                }
                InterceptAction::MapLocal { file_path, .. } => {
                    format!("MapLocal({})", truncate(file_path, 20))
                }
                InterceptAction::MapRemote { target_url, .. } => {
                    format!("MapRemote({})", truncate(target_url, 20))
                }
            };

            let method = rule.method.as_deref().unwrap_or("*");

            let selected = app.selected_rule == Some(i);
            let style = if selected {
                Style::default().bg(Color::Rgb(40, 40, 60))
            } else {
                Style::default()
            };

            Row::new(vec![
                enabled,
                Cell::from(rule.name.clone()),
                Cell::from(truncate(&rule.pattern, 30)),
                Cell::from(method.to_string()),
                Cell::from(action),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(20),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(22),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!(" Intercept Rules ({}) ", app.rules.len())),
    );

    f.render_widget(table, area);
}

fn draw_rules_help(f: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(vec![
        Span::styled("t", Style::default().fg(Color::Yellow)),
        Span::raw(": 토글  "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(": 삭제  "),
        Span::styled("C", Style::default().fg(Color::Yellow)),
        Span::raw(": 전체삭제  "),
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(": 이동"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(help, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
