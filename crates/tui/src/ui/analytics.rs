use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let result = match &app.analytics_result {
        Some(r) => r.clone(),
        None => {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  분석 데이터가 없습니다. [r] 키를 눌러 분석을 실행하세요.",
                    Style::default().fg(Color::Gray),
                )),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
                    .title(" Analytics ")
                    .title_bottom(
                        Line::from(" r: 새로고침 ").style(Style::default().fg(Color::Cyan)),
                    ),
            );
            f.render_widget(empty, area);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    // -- 요약 섹션 --
    lines.push(Line::from(Span::styled(
        "── 요약 ──────────────────────────────────────────────────",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    let error_rate_color = if result.error_rate > 10.0 {
        Color::Red
    } else if result.error_rate > 5.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    lines.push(Line::from(vec![
        Span::styled("  총 요청: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_number(result.total_requests),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  에러율: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:.1}%", result.error_rate),
            Style::default()
                .fg(error_rate_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  평균: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}ms", result.avg_duration_ms as u64),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  P95: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}ms", result.p95_duration_ms),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // -- 느린 요청 Top 10 --
    lines.push(Line::from(Span::styled(
        "── 느린 요청 Top 10 ──────────────────────────────────────",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    if result.slow_requests.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (데이터 없음)",
            Style::default().fg(Color::Gray),
        )));
    } else {
        // 헤더
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<3}", "#"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<8}", "Method"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<40}", "URL"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>10}", "Duration"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>8}", "Status"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        for (i, req) in result.slow_requests.iter().enumerate() {
            let url_display = truncate_str(&req.url, 38);
            let status_color = match req.status {
                200..=299 => Color::Green,
                300..=399 => Color::Yellow,
                400..=499 => Color::Red,
                500..=599 => Color::Magenta,
                _ => Color::Gray,
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {:<3}", i + 1), Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{:<8}", req.method),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{:<40}", url_display),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:>8}ms", format_number_u64(req.duration_ms)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:>8}", req.status),
                    Style::default().fg(status_color),
                ),
            ]));
        }
    }
    lines.push(Line::from(""));

    // -- 에러 분포 --
    lines.push(Line::from(Span::styled(
        "── 에러 분포 ─────────────────────────────────────────────",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    if result.error_distribution.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (에러 없음)",
            Style::default().fg(Color::Green),
        )));
    } else {
        let max_count = result
            .error_distribution
            .iter()
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(1);
        let max_bar_width = 30usize;

        for (status, count) in &result.error_distribution {
            let bar_width = if max_count > 0 {
                ((*count as f64 / max_count as f64) * max_bar_width as f64).ceil() as usize
            } else {
                0
            };
            let bar: String = "\u{2588}".repeat(bar_width);

            let status_color = match *status {
                400..=499 => Color::Red,
                500..=599 => Color::Magenta,
                _ => Color::Yellow,
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}: ", status),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(bar, Style::default().fg(status_color)),
                Span::styled(format!(" {}", count), Style::default().fg(Color::White)),
            ]));
        }
    }
    lines.push(Line::from(""));

    // -- 중복 요청 --
    lines.push(Line::from(Span::styled(
        "── 중복 요청 ─────────────────────────────────────────────",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    if result.duplicate_requests.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (중복 요청 없음)",
            Style::default().fg(Color::Green),
        )));
    } else {
        for dup in &result.duplicate_requests {
            let url_display = truncate_str(&dup.url, 50);
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("{} ", dup.method), Style::default().fg(Color::Cyan)),
                Span::styled(url_display, Style::default().fg(Color::White)),
                Span::styled(
                    format!(" (x{}, {}초 내)", dup.count, dup.window_secs),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }
    lines.push(Line::from(""));

    // -- N+1 패턴 --
    lines.push(Line::from(Span::styled(
        "── N+1 패턴 ──────────────────────────────────────────────",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    if result.n_plus_one_patterns.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (N+1 패턴 미탐지)",
            Style::default().fg(Color::Green),
        )));
    } else {
        for pattern in &result.n_plus_one_patterns {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    &pattern.pattern,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" - {}회 연속 호출", pattern.count),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("→ {}", pattern.suggestion),
                    Style::default().fg(Color::Green),
                ),
            ]));
        }
    }

    // 스크롤 처리
    let visible_height = area.height.saturating_sub(2) as u16;
    let total_lines = lines.len() as u16;
    let max_scroll = total_lines.saturating_sub(visible_height);
    app.analytics_scroll = app.analytics_scroll.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
                .title(format!(" Analytics ({} requests) ", result.total_requests))
                .title_bottom(
                    Line::from(" r: 새로고침 | j/k: 스크롤 ")
                        .style(Style::default().fg(Color::Cyan)),
                ),
        )
        .scroll((app.analytics_scroll, 0));

    f.render_widget(paragraph, area);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_number_u64(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
