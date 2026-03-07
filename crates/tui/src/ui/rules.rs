use proxy_daemon::InterceptAction;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, RuleFormField};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    draw_rules_list(f, app, area);
}

fn draw_rules_list(f: &mut Frame, app: &mut App, area: Rect) {
    if app.rules.is_empty() {
        let empty = Paragraph::new("  No rules defined. Press 'a' to add.")
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
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
        .map(|(_i, rule)| {
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

            Row::new(vec![
                enabled,
                Cell::from(rule.name.clone()),
                Cell::from(truncate(&rule.pattern, 30)),
                Cell::from(method.to_string()),
                Cell::from(action),
            ])
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
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(50, 60, 140))
            .fg(Color::White),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(format!(" Intercept Rules ({}) ", app.rules.len()))
            .title_bottom(
                Line::from(" j/k: navigate | a: add | t: toggle | d: delete | C: clear all ")
                    .style(Style::default().fg(Color::DarkGray)),
            ),
    );

    f.render_stateful_widget(table, area, &mut app.rules_table_state);
}

pub fn draw_rule_form(f: &mut Frame, app: &App, area: Rect) {
    let form = match &app.rule_form {
        Some(form) => form,
        None => return,
    };

    // Center the popup
    let popup_area = centered_rect(60, 70, area);

    // Clear background
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Add Rule (Enter: save, Esc: cancel) ");

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Pattern
            Constraint::Length(3), // Method
            Constraint::Length(3), // Action Type
            Constraint::Length(3), // Dynamic field 1
            Constraint::Length(3), // Dynamic field 2
            Constraint::Min(0),    // Spacer
        ])
        .split(inner);

    // Name
    draw_input_field(
        f,
        "Name",
        &form.name,
        form.field == RuleFormField::Name,
        chunks[0],
    );

    // Pattern
    draw_input_field(
        f,
        "Pattern *",
        &form.pattern,
        form.field == RuleFormField::Pattern,
        chunks[1],
    );

    // Method
    let method_display = form.method.as_deref().unwrap_or("ALL");
    draw_select_field(
        f,
        "Method",
        method_display,
        form.field == RuleFormField::Method,
        chunks[2],
    );

    // Action Type
    draw_select_field(
        f,
        "Action",
        form.action_type.label(),
        form.field == RuleFormField::ActionType,
        chunks[3],
    );

    // Dynamic fields based on action type
    match form.action_type {
        crate::app::ActionType::Block => {
            draw_input_field(
                f,
                "Status Code",
                &form.status_code,
                form.field == RuleFormField::StatusCode,
                chunks[4],
            );
            draw_input_field(
                f,
                "Body",
                &form.body,
                form.field == RuleFormField::Body,
                chunks[5],
            );
        }
        crate::app::ActionType::ModifyRequest | crate::app::ActionType::ModifyResponse => {
            draw_input_field(
                f,
                "Body",
                &form.body,
                form.field == RuleFormField::Body,
                chunks[4],
            );
            // Empty
            f.render_widget(Paragraph::new(""), chunks[5]);
        }
        crate::app::ActionType::MapLocal => {
            draw_input_field(
                f,
                "File Path",
                &form.file_path,
                form.field == RuleFormField::FilePath,
                chunks[4],
            );
            draw_input_field(
                f,
                "Status Code",
                &form.status_code,
                form.field == RuleFormField::StatusCode,
                chunks[5],
            );
        }
        crate::app::ActionType::MapRemote => {
            draw_input_field(
                f,
                "Target URL",
                &form.target_url,
                form.field == RuleFormField::TargetUrl,
                chunks[4],
            );
            // Empty
            f.render_widget(Paragraph::new(""), chunks[5]);
        }
    }
}

fn draw_input_field(f: &mut Frame, label: &str, value: &str, active: bool, area: Rect) {
    let border_style = if active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let display = if active {
        format!("{}▎", value)
    } else {
        value.to_string()
    };

    let input = Paragraph::new(display).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", label)),
    );

    f.render_widget(input, area);
}

fn draw_select_field(f: &mut Frame, label: &str, value: &str, active: bool, area: Rect) {
    let border_style = if active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let display = if active {
        format!("◄ {} ►", value)
    } else {
        value.to_string()
    };

    let input = Paragraph::new(display).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", label)),
    );

    f.render_widget(input, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
