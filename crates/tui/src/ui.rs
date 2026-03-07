mod network;
mod rules;
mod settings;
mod websocket;

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::tabs::Tab;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 탭 바
            Constraint::Min(0),    // 콘텐츠
            Constraint::Length(1), // 상태 바
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    draw_content(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| {
            let style = if *t == app.tab {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(t.title(), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Cheolsu Proxy "),
        )
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(Tab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0))
        .divider("│");

    f.render_widget(tabs, area);
}

fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    match app.tab {
        Tab::Network => network::draw(f, app, area),
        Tab::WebSocket => websocket::draw(f, app, area),
        Tab::InterceptRules => rules::draw(f, app, area),
        Tab::Settings => settings::draw(f, app, area),
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
        Style::default().fg(Color::DarkGray),
    );

    let help = Span::styled(
        "  Tab: 탭전환 | q: 종료",
        Style::default().fg(Color::DarkGray),
    );

    let paused = if app.paused {
        Span::styled("  ⏸ 일시정지", Style::default().fg(Color::Yellow))
    } else {
        Span::raw("")
    };

    let bar = Line::from(vec![status_indicator, port_info, paused, help]);
    let paragraph = Paragraph::new(bar).style(Style::default().bg(Color::Rgb(30, 30, 30)));

    f.render_widget(paragraph, area);
}

/// 바이트 크기를 읽기 좋은 형식으로 변환
pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// 나노초 타임스탬프를 시:분:초로 변환
pub fn format_time(nanos: i64) -> String {
    let secs = nanos / 1_000_000_000;
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
