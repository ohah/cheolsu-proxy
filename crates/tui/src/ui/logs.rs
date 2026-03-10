use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::ui::format_size;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30), // File list
            Constraint::Min(0),     // Log content
        ])
        .split(area);

    draw_file_list(f, app, chunks[0]);
    draw_log_content(f, app, chunks[1]);
}

fn draw_file_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<Row> = app
        .log_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let style = if app.selected_log_file == Some(i) {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let marker = if app.selected_log_file == Some(i) {
                "▶ "
            } else {
                "  "
            };

            Row::new(vec![
                Cell::from(format!("{}{}", marker, file.name)),
                Cell::from(format_size(file.size as usize)).style(Style::default().fg(Color::Gray)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(items, [Constraint::Min(0), Constraint::Length(8)])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
                .title(format!(" Log Files ({}) ", app.log_files.len())),
        )
        .row_highlight_style(Style::default().fg(Color::Cyan));

    f.render_widget(table, area);
}

fn draw_log_content(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Filter + keybindings
        ])
        .split(area);

    let inner_height = chunks[0].height.saturating_sub(2) as usize;

    // 필터 적용
    let filtered_lines: Vec<&str> = if app.log_filter.is_empty() {
        app.log_content_lines.iter().map(|s| s.as_str()).collect()
    } else {
        let lower = app.log_filter.to_lowercase();
        app.log_content_lines
            .iter()
            .filter(|line| line.to_lowercase().contains(&lower))
            .map(|s| s.as_str())
            .collect()
    };

    let total = filtered_lines.len();
    let scroll_offset = if app.log_filter.is_empty() {
        app.log_scroll.min(total.saturating_sub(inner_height))
    } else {
        // 필터링 시 항상 끝에서 보여줌
        total.saturating_sub(inner_height)
    };

    let log_lines: Vec<Line> = filtered_lines
        .iter()
        .skip(scroll_offset)
        .take(inner_height)
        .map(|line| {
            let style = if line.contains("ERROR") {
                Style::default().fg(Color::Red)
            } else if line.contains("WARN") {
                Style::default().fg(Color::Yellow)
            } else if line.contains("DEBUG") || line.contains("TRACE") {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            Line::from(Span::styled(*line, style))
        })
        .collect();

    let title = if let Some(idx) = app.selected_log_file {
        let name = app
            .log_files
            .get(idx)
            .map(|f| f.name.as_str())
            .unwrap_or("?");
        if app.log_filter.is_empty() {
            format!(" {} ({} lines) ", name, total)
        } else {
            format!(
                " {} ({}/{} lines, filter: \"{}\") ",
                name,
                total,
                app.log_content_lines.len(),
                app.log_filter
            )
        }
    } else {
        " No file selected ".to_string()
    };

    let content = Paragraph::new(log_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(title),
    );

    f.render_widget(content, chunks[0]);

    // Keybindings / filter bar
    let help_line = if app.log_filter_editing {
        Line::from(vec![
            Span::styled("  Filter: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{}█", app.log_filter),
                Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 50)),
            ),
            Span::styled(
                "  (Enter: apply, Esc: cancel)",
                Style::default().fg(Color::Gray),
            ),
        ])
    } else {
        Line::from(vec![Span::styled(
            "  j/k: scroll | g/G: top/bottom | h/l: select file | r: refresh | /: filter | c: clear filter | C: clear file",
            Style::default().fg(Color::Gray),
        )])
    };

    let help = Paragraph::new(help_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(" Keybindings "),
    );

    f.render_widget(help, chunks[1]);
}
