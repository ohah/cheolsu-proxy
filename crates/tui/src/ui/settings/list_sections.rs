/// Settings 탭 - 리스트 기반 섹션 렌더링 (HostMapping, SSL Proxying)
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, HostMappingField, SslProxyingAddForm};

pub(super) fn draw_host_mapping(f: &mut Frame, app: &App, area: Rect) {
    // If form is open, draw form instead
    if let Some(form) = &app.host_mapping_form {
        draw_host_mapping_form(f, form, area);
        return;
    }

    let mappings = &app.host_mappings;

    if mappings.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No host mappings configured",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press 'a' to add a new mapping",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(format!(" Host Mapping [{} entries] ", mappings.len()));

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let rows: Vec<Row> = mappings
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let is_selected = app.selected_host_mapping == Some(i);
            let style = if !m.enabled {
                Style::default().fg(Color::DarkGray)
            } else if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let src = if let Some(port) = m.source_port {
                format!("{}:{}", m.source_host, port)
            } else {
                m.source_host.clone()
            };
            let tgt = if let Some(port) = m.target_port {
                format!("{}:{}", m.target_host, port)
            } else {
                m.target_host.clone()
            };
            let status = if m.enabled { "ON" } else { "OFF" };

            Row::new(vec![src, "->".to_string(), tgt, status.to_string()]).style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(35),
        Constraint::Length(4),
        Constraint::Percentage(35),
        Constraint::Length(5),
    ];

    let header = Row::new(vec!["Source", "  ", "Target", "State"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(0);

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .title(format!(" Host Mapping [{} entries] ", mappings.len())),
        );

    let mut table_state = app.host_mapping_table_state.clone();
    f.render_stateful_widget(table, area, &mut table_state);
}

fn draw_host_mapping_form(f: &mut Frame, form: &crate::app::HostMappingForm, area: Rect) {
    let fields: Vec<Line> = HostMappingField::ALL
        .iter()
        .map(|field| {
            let is_selected = *field == form.field;

            let label = Span::styled(
                format!("  {:<14} ", field.label()),
                if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            );

            let value_str = match field {
                HostMappingField::SourceHost => &form.source_host,
                HostMappingField::SourcePort => &form.source_port,
                HostMappingField::TargetHost => &form.target_host,
                HostMappingField::TargetPort => &form.target_port,
            };

            let value = if is_selected {
                Span::styled(
                    format!("{}█", value_str),
                    Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 50)),
                )
            } else if value_str.is_empty() {
                let placeholder = match field {
                    HostMappingField::SourceHost => "(e.g. *.api.example.com)",
                    HostMappingField::SourcePort => "(optional)",
                    HostMappingField::TargetHost => "(e.g. 192.168.1.100)",
                    HostMappingField::TargetPort => "(optional)",
                };
                Span::styled(placeholder, Style::default().fg(Color::DarkGray))
            } else {
                Span::styled(value_str.as_str(), Style::default().fg(Color::White))
            };

            let cursor = if is_selected {
                Span::styled("▶ ", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("  ")
            };

            Line::from(vec![cursor, label, value])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Add Host Mapping (Enter: save, Esc: cancel, Tab: next field) ");

    let paragraph = Paragraph::new(fields).block(block);
    f.render_widget(paragraph, area);
}

pub(super) fn draw_ssl_proxying(f: &mut Frame, app: &App, area: Rect) {
    // If add form is open, draw form instead
    if let Some(form) = &app.ssl_proxying_add_form {
        draw_ssl_proxying_add_form(f, form, area);
        return;
    }

    let entries = &app.ssl_proxying_entries;
    let mode_label = match app.ssl_proxying_mode {
        proxy_daemon::SslProxyingMode::Blacklist => "Blacklist",
        proxy_daemon::SslProxyingMode::Whitelist => "Whitelist",
    };

    if entries.is_empty() {
        let mode_desc = match app.ssl_proxying_mode {
            proxy_daemon::SslProxyingMode::Blacklist => {
                "  All HTTPS intercepted, OAuth domains auto-excluded"
            }
            proxy_daemon::SslProxyingMode::Whitelist => {
                "  All HTTPS traffic will be intercepted (no whitelist)"
            }
        };
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  Mode: {} (press 'm' to toggle)", mode_label),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(Span::styled(
                mode_desc,
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press 'a' to add a domain pattern",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  Examples: *.example.com, api.io:8443",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(format!(
                " SSL Proxying [{}] [{} entries] ",
                mode_label,
                entries.len()
            ));

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let rows: Vec<Row> = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = app.selected_ssl_proxying == Some(i);
            let style = if !entry.enabled {
                Style::default().fg(Color::DarkGray)
            } else if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let status = if entry.enabled { "ON" } else { "OFF" };
            Row::new(vec![entry.pattern.clone(), status.to_string()]).style(style)
        })
        .collect();

    let widths = [Constraint::Percentage(80), Constraint::Length(5)];

    let header = Row::new(vec!["Pattern", "State"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(0);

    let enabled_count = entries.iter().filter(|e| e.enabled).count();
    let title = if enabled_count > 0 {
        format!(
            " SSL Proxying [{}] [{}/{} active] ",
            mode_label,
            enabled_count,
            entries.len()
        )
    } else {
        format!(
            " SSL Proxying [{}] [{} entries, all disabled] ",
            mode_label,
            entries.len()
        )
    };

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .title(title),
        );

    let mut table_state = app.ssl_proxying_table_state.clone();
    f.render_stateful_widget(table, area, &mut table_state);
}

fn draw_ssl_proxying_add_form(f: &mut Frame, form: &SslProxyingAddForm, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("▶ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "  Pattern        ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}█", form.pattern),
                Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 50)),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Supported formats:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "    example.com          Exact domain (any port)",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "    *.example.com        Wildcard subdomains",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "    example.com:8443     Domain with specific port",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "    *.example.com:443    Wildcard with port",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Add SSL Proxying Pattern (Enter: save, Esc: cancel) ");

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}
