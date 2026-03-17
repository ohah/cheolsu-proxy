use proxy_daemon::BreakpointPhase;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, BreakpointEditField, BreakpointFocus, BreakpointFormField};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_rules_panel(f, app, chunks[0]);
    draw_pending_panel(f, app, chunks[1]);
}

fn draw_rules_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.bp_focus == BreakpointFocus::Rules;
    let border_color = if is_focused { Color::Cyan } else { Color::Gray };

    if app.breakpoint_rules.is_empty() {
        let empty = Paragraph::new("  No breakpoint rules. Press 'a' to add.")
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(" Breakpoint Rules ")
                    .title_bottom(
                        Line::from(" a: add | Left/Right: switch panel ")
                            .style(Style::default().fg(Color::Cyan)),
                    ),
            );
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("Pattern"),
        Cell::from("Req"),
        Cell::from("Res"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .breakpoint_rules
        .iter()
        .map(|rule| {
            let enabled = if rule.enabled {
                Cell::from("on").style(Style::default().fg(Color::Green))
            } else {
                Cell::from("off").style(Style::default().fg(Color::Red))
            };

            let req = if rule.break_on_request {
                Cell::from("Y").style(Style::default().fg(Color::Cyan))
            } else {
                Cell::from("-").style(Style::default().fg(Color::DarkGray))
            };

            let res = if rule.break_on_response {
                Cell::from("Y").style(Style::default().fg(Color::Magenta))
            } else {
                Cell::from("-").style(Style::default().fg(Color::DarkGray))
            };

            Row::new(vec![
                enabled,
                Cell::from(truncate(&rule.pattern, 30)),
                req,
                res,
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Min(20),
            Constraint::Length(4),
            Constraint::Length(4),
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
            .border_style(Style::default().fg(border_color))
            .title(format!(
                " Breakpoint Rules ({}) ",
                app.breakpoint_rules.len()
            ))
            .title_bottom(
                Line::from(" a: add | t: toggle | d: delete ")
                    .style(Style::default().fg(Color::Cyan)),
            ),
    );

    f.render_stateful_widget(table, area, &mut app.bp_rules_table_state);
}

fn draw_pending_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.bp_focus == BreakpointFocus::Pending;
    let border_color = if is_focused { Color::Cyan } else { Color::Gray };

    if app.pending_breakpoints.is_empty() {
        let empty = Paragraph::new("  No pending breakpoints.")
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(" Pending Breakpoints "),
            );
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Phase"),
        Cell::from("Method"),
        Cell::from("URL"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .pending_breakpoints
        .iter()
        .map(|bp| {
            let phase = match bp.phase {
                BreakpointPhase::Request => {
                    Cell::from("REQ").style(Style::default().fg(Color::Cyan))
                }
                BreakpointPhase::Response => {
                    Cell::from("RES").style(Style::default().fg(Color::Magenta))
                }
            };

            Row::new(vec![
                phase,
                Cell::from(bp.data.method.clone()),
                Cell::from(truncate(&bp.data.url, 40)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(140, 60, 50))
            .fg(Color::White),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(format!(" Pending ({}) ", app.pending_breakpoints.len()))
            .title_bottom(
                Line::from(" e: edit | f: forward | x: drop | b: abort ")
                    .style(Style::default().fg(Color::Cyan)),
            ),
    );

    f.render_stateful_widget(table, area, &mut app.bp_pending_table_state);
}

pub fn draw_bp_add_form(f: &mut Frame, app: &App, area: Rect) {
    let form = match &app.bp_add_form {
        Some(form) => form,
        None => return,
    };

    let popup_area = centered_rect(50, 40, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Add Breakpoint Rule (Enter: save, Esc: cancel) ");

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Pattern
            Constraint::Length(3), // Break on Request
            Constraint::Length(3), // Break on Response
            Constraint::Min(0),    // Spacer
        ])
        .split(inner);

    // Pattern
    let pattern_border = if form.field == BreakpointFormField::Pattern {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };
    let pattern_display = if form.field == BreakpointFormField::Pattern {
        format!("{}|", form.pattern)
    } else {
        form.pattern.clone()
    };
    let pattern = Paragraph::new(pattern_display).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(pattern_border)
            .title(" Pattern * "),
    );
    f.render_widget(pattern, chunks[0]);

    // Break on Request
    let req_border = if form.field == BreakpointFormField::BreakOnRequest {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };
    let req_val = if form.break_on_request { "Yes" } else { "No" };
    let req_display = if form.field == BreakpointFormField::BreakOnRequest {
        format!("< {} >", req_val)
    } else {
        req_val.to_string()
    };
    let req = Paragraph::new(req_display).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(req_border)
            .title(" Break on Request "),
    );
    f.render_widget(req, chunks[1]);

    // Break on Response
    let res_border = if form.field == BreakpointFormField::BreakOnResponse {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };
    let res_val = if form.break_on_response { "Yes" } else { "No" };
    let res_display = if form.field == BreakpointFormField::BreakOnResponse {
        format!("< {} >", res_val)
    } else {
        res_val.to_string()
    };
    let res = Paragraph::new(res_display).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(res_border)
            .title(" Break on Response "),
    );
    f.render_widget(res, chunks[2]);
}

pub fn draw_bp_edit_form(f: &mut Frame, app: &App, area: Rect) {
    let form = match &app.bp_edit_form {
        Some(form) => form,
        None => return,
    };

    let is_response = form.phase == BreakpointPhase::Response;
    let popup_area = centered_rect(70, 70, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Edit & Forward (Enter: send, Shift+Enter: newline, Tab: next, Esc: cancel) ");

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let mut constraints = vec![
        Constraint::Length(8), // Headers
        Constraint::Length(6), // Body
    ];
    if is_response {
        constraints.push(Constraint::Length(3)); // Status
    }
    constraints.push(Constraint::Min(0)); // Spacer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    // Headers
    let headers_border = if form.field == BreakpointEditField::Headers {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };
    let headers_display = if form.field == BreakpointEditField::Headers {
        format!("{}|", form.headers_text)
    } else {
        form.headers_text.clone()
    };
    let headers = Paragraph::new(headers_display)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(headers_border)
                .title(" Headers (key: value per line) "),
        );
    f.render_widget(headers, chunks[0]);

    // Body
    let body_border = if form.field == BreakpointEditField::Body {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };
    let body_display = if form.field == BreakpointEditField::Body {
        format!("{}|", form.body)
    } else {
        form.body.clone()
    };
    let body = Paragraph::new(body_display)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(body_border)
                .title(" Body "),
        );
    f.render_widget(body, chunks[1]);

    // Status (response only)
    if is_response {
        let status_border = if form.field == BreakpointEditField::Status {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };
        let status_display = if form.field == BreakpointEditField::Status {
            format!("{}|", form.status)
        } else {
            form.status.clone()
        };
        let status = Paragraph::new(status_display).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(status_border)
                .title(" Status Code "),
        );
        f.render_widget(status, chunks[2]);
    }
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
        format!("{}...", &s[..max - 3])
    }
}
