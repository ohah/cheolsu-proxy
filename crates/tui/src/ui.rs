mod analytics;
mod breakpoint;
mod logs;
mod network;
mod rules;
mod script;
mod settings;
mod websocket;

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::tabs::Tab;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    draw_content(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);

    // Draw rule form overlay if open
    if app.rule_form.is_some() {
        rules::draw_rule_form(f, app, f.area());
    }

    // Draw breakpoint add form overlay if open
    if app.bp_add_form.is_some() {
        breakpoint::draw_bp_add_form(f, app, f.area());
    }

    // Draw breakpoint edit form overlay if open
    if app.bp_edit_form.is_some() {
        breakpoint::draw_bp_edit_form(f, app, f.area());
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| {
            let style = if *t == app.tab {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(t.title(), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
                .title(Span::styled(
                    " Cheolsu Proxy ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(Style::default().fg(Color::Cyan))
        .select(Tab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0))
        .divider("│");

    f.render_widget(tabs, area);
}

fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    match app.tab {
        Tab::Network => network::draw(f, app, area),
        Tab::WebSocket => websocket::draw(f, app, area),
        Tab::InterceptRules => rules::draw(f, app, area),
        Tab::Script => script::draw(f, app, area),
        Tab::Breakpoint => breakpoint::draw(f, app, area),
        Tab::Analytics => analytics::draw(f, app, area),
        Tab::Settings => settings::draw(f, app, area),
        Tab::Logs => logs::draw(f, app, area),
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_indicator = if app.connected {
        Span::styled("● Connected", Style::default().fg(Color::Green))
    } else {
        Span::styled("● Disconnected", Style::default().fg(Color::Red))
    };

    let port_info = Span::styled(
        format!("  {}:{}", app.host, app.port),
        Style::default().fg(Color::Gray),
    );

    let tab_help = match app.tab {
        Tab::Network => "  j/k: nav | Enter: detail | y/Y: copy | Space: pause | c: clear",
        Tab::WebSocket => "  j/k: nav | y: copy URI | Y: copy msgs | c: clear",
        Tab::InterceptRules => "  j/k: nav | a: add | t: toggle | d: delete | C: clear all",
        Tab::Script => "  l: load | u: unload | r: reload | c: clear | j/k: scroll",
        Tab::Breakpoint => "  a: add | t: toggle | d: del | e: edit | f: fwd | x: drop | b: abort",
        Tab::Analytics => "  r: 새로고침 | j/k: 스크롤",
        Tab::Settings => "  j/k: nav | Enter/Space: toggle/edit | Esc: cancel",
        Tab::Logs => "  j/k: scroll | h/l: file | /: filter | r: refresh | C: clear",
    };
    let help = Span::styled(
        format!("  Tab: switch | q: quit{}", tab_help),
        Style::default().fg(Color::Gray),
    );

    let paused = if app.paused {
        Span::styled("  ⏸ Paused", Style::default().fg(Color::Yellow))
    } else {
        Span::raw("")
    };

    let status_msg = if let Some((msg, _)) = &app.status_message {
        Span::styled(format!("  {}", msg), Style::default().fg(Color::Green))
    } else {
        Span::raw("")
    };

    let bar = Line::from(vec![status_indicator, port_info, paused, help, status_msg]);
    let paragraph = Paragraph::new(bar).style(Style::default().bg(Color::Rgb(30, 30, 30)));

    f.render_widget(paragraph, area);
}

/// Format byte size to human-readable form
pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Convert nanosecond timestamp to HH:MM:SS
pub fn format_time(nanos: i64) -> String {
    let secs = nanos / 1_000_000_000;
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- format_size --

    #[test]
    fn format_size_zero() {
        assert_eq!(format_size(0), "0B");
    }

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(500), "500B");
        assert_eq!(format_size(1023), "1023B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0KB");
        assert_eq!(format_size(1536), "1.5KB");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(1048576), "1.0MB");
        assert_eq!(format_size(2621440), "2.5MB");
    }

    #[test]
    fn format_size_boundary() {
        // 1KB 미만은 B, 1KB 이상은 KB
        assert_eq!(format_size(1023), "1023B");
        assert_eq!(format_size(1024), "1.0KB");
        // 1MB 미만은 KB, 1MB 이상은 MB
        assert_eq!(format_size(1048575), "1024.0KB");
        assert_eq!(format_size(1048576), "1.0MB");
    }

    // -- format_time --

    #[test]
    fn format_time_zero() {
        assert_eq!(format_time(0), "00:00:00");
    }

    #[test]
    fn format_time_seconds() {
        assert_eq!(format_time(5_000_000_000), "00:00:05");
    }

    #[test]
    fn format_time_minutes() {
        assert_eq!(format_time(90_000_000_000), "00:01:30");
    }

    #[test]
    fn format_time_hours() {
        assert_eq!(format_time(3_661_000_000_000), "01:01:01");
    }

    #[test]
    fn format_time_wraps_at_24h() {
        // 25시간 = 1시간으로 표시 (% 24)
        let nanos_25h = 25 * 3600 * 1_000_000_000_i64;
        assert_eq!(format_time(nanos_25h), "01:00:00");
    }

    #[test]
    fn format_time_padding() {
        // 한 자리 숫자도 두 자리로 패딩
        assert_eq!(format_time(1_000_000_000), "00:00:01");
    }
}
