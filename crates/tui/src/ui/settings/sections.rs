/// Settings 탭 - 폼 기반 섹션 렌더링 (UpstreamProxy, ProxyAuth, Throttle, QuickSettings, ClientCertificate)
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{
    App, ClientCertField, ProxyAuthField, QuickSettingsField, SettingsSection, ThrottleField,
    ThrottlePresetChoice, UpstreamProxyField,
};

use super::render_text_field;

pub(super) fn draw_upstream_proxy(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.upstream_form;

    let fields: Vec<Line> = UpstreamProxyField::ALL
        .iter()
        .map(|field| {
            let is_selected = *field == form.field;
            let is_editing = is_selected && form.editing;

            let label = Span::styled(
                format!("  {:<10} ", field.label()),
                if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            );

            let value = match field {
                UpstreamProxyField::Enabled => {
                    let (text, color) = if form.enabled {
                        ("ON", Color::Green)
                    } else {
                        ("OFF", Color::Red)
                    };
                    Span::styled(
                        text,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )
                }
                UpstreamProxyField::Host => render_text_field(&form.host, is_editing),
                UpstreamProxyField::Port => render_text_field(&form.port, is_editing),
                UpstreamProxyField::Username => render_text_field(&form.username, is_editing),
                UpstreamProxyField::Password => {
                    if form.password.is_empty() {
                        render_text_field("", is_editing)
                    } else if is_editing {
                        render_text_field(&form.password, true)
                    } else {
                        Span::styled(
                            "*".repeat(form.password.len()),
                            Style::default().fg(Color::White),
                        )
                    }
                }
                UpstreamProxyField::Bypass => render_text_field(&form.bypass, is_editing),
            };

            let cursor = if is_selected {
                Span::styled("▶ ", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("  ")
            };

            Line::from(vec![cursor, label, value])
        })
        .collect();

    let title = if form.enabled {
        " Upstream Proxy [ON] "
    } else {
        " Upstream Proxy [OFF] "
    };

    let border_color = if form.enabled {
        Color::Green
    } else {
        Color::Gray
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let paragraph = Paragraph::new(fields).block(block);
    f.render_widget(paragraph, area);
}

pub(super) fn draw_proxy_auth(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.proxy_auth_form;

    let fields: Vec<Line> = ProxyAuthField::ALL
        .iter()
        .map(|field| {
            let is_selected = *field == form.field;
            let is_editing = is_selected && form.editing;

            let label = Span::styled(
                format!("  {:<10} ", field.label()),
                if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            );

            let value = match field {
                ProxyAuthField::Enabled => {
                    let (text, color) = if form.enabled {
                        ("ON", Color::Green)
                    } else {
                        ("OFF", Color::Red)
                    };
                    Span::styled(
                        text,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )
                }
                ProxyAuthField::Username => render_text_field(&form.username, is_editing),
                ProxyAuthField::Password => {
                    if form.password.is_empty() {
                        render_text_field("", is_editing)
                    } else if is_editing {
                        render_text_field(&form.password, true)
                    } else {
                        Span::styled(
                            "*".repeat(form.password.len()),
                            Style::default().fg(Color::White),
                        )
                    }
                }
            };

            let cursor = if is_selected {
                Span::styled("\u{25b6} ", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("  ")
            };

            Line::from(vec![cursor, label, value])
        })
        .collect();

    let title = if form.enabled {
        " Proxy Auth [ON] "
    } else {
        " Proxy Auth [OFF] "
    };

    let border_color = if form.enabled {
        Color::Yellow
    } else {
        Color::Gray
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let paragraph = Paragraph::new(fields).block(block);
    f.render_widget(paragraph, area);
}

pub(super) fn draw_throttle(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.throttle_form;

    let fields: Vec<Line> = ThrottleField::ALL
        .iter()
        .map(|field| {
            let is_selected =
                *field == form.field && app.settings_section == SettingsSection::Throttle;
            let is_editing = is_selected && form.editing;

            let label = Span::styled(
                format!("  {:<16} ", field.label()),
                if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            );

            let value = match field {
                ThrottleField::Enabled => {
                    let (text, color) = if form.enabled {
                        ("ON", Color::Green)
                    } else {
                        ("OFF", Color::Red)
                    };
                    Span::styled(
                        text,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )
                }
                ThrottleField::Preset => Span::styled(
                    format!("◀ {} ▶", form.preset.label()),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                ThrottleField::Download => render_text_field(&form.download, is_editing),
                ThrottleField::Upload => render_text_field(&form.upload, is_editing),
                ThrottleField::Latency => render_text_field(&form.latency, is_editing),
            };

            let cursor = if is_selected {
                Span::styled("▶ ", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("  ")
            };

            // Custom이 아닌 프리셋일 때 Download/Upload/Latency 필드는 흐리게
            let dim = matches!(
                field,
                ThrottleField::Download | ThrottleField::Upload | ThrottleField::Latency
            ) && form.preset != ThrottlePresetChoice::Custom;

            if dim {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("  {:<16} ", field.label()),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("—", Style::default().fg(Color::DarkGray)),
                ])
            } else {
                Line::from(vec![cursor, label, value])
            }
        })
        .collect();

    let title = if form.enabled {
        format!(" Throttle [ON: {}] ", form.preset.label())
    } else {
        " Throttle [OFF] ".to_string()
    };

    let border_color = if form.enabled {
        Color::Magenta
    } else {
        Color::Gray
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let paragraph = Paragraph::new(fields).block(block);
    f.render_widget(paragraph, area);
}

pub(super) fn draw_quick_settings(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.quick_settings_form;

    let fields: Vec<Line> = QuickSettingsField::ALL
        .iter()
        .map(|field| {
            let is_active =
                *field == form.field && app.settings_section == SettingsSection::QuickSettings;
            let cursor = if is_active {
                Span::styled("▸ ", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("  ")
            };

            let label = Span::styled(
                format!("{:<16}", field.label()),
                if is_active {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            );

            let value = match field {
                QuickSettingsField::NoCaching => {
                    let (text, color) = if form.no_caching {
                        ("[ON]", Color::Green)
                    } else {
                        ("[OFF]", Color::Red)
                    };
                    Span::styled(text, Style::default().fg(color))
                }
                QuickSettingsField::BlockCookies => {
                    let (text, color) = if form.block_cookies {
                        ("[ON]", Color::Green)
                    } else {
                        ("[OFF]", Color::Red)
                    };
                    Span::styled(text, Style::default().fg(color))
                }
                QuickSettingsField::NoGzip => {
                    let (text, color) = if form.no_gzip {
                        ("[ON]", Color::Green)
                    } else {
                        ("[OFF]", Color::Red)
                    };
                    Span::styled(text, Style::default().fg(color))
                }
            };

            Line::from(vec![cursor, label, value])
        })
        .collect();

    let title = if form.no_caching || form.block_cookies || form.no_gzip {
        let mut active = Vec::new();
        if form.no_caching {
            active.push("No Caching");
        }
        if form.block_cookies {
            active.push("Block Cookies");
        }
        if form.no_gzip {
            active.push("No Gzip");
        }
        format!(" Quick Settings [{}] ", active.join(", "))
    } else {
        " Quick Settings [OFF] ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    let paragraph = Paragraph::new(fields).block(block);
    f.render_widget(paragraph, area);
}

pub(super) fn draw_client_certificate(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.client_cert_form;

    let fields: Vec<Line> = ClientCertField::ALL
        .iter()
        .map(|field| {
            let is_selected =
                *field == form.field && app.settings_section == SettingsSection::ClientCertificate;
            let is_editing = is_selected && form.editing;

            let label = Span::styled(
                format!("  {:<12} ", field.label()),
                if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            );

            let value = match field {
                ClientCertField::Enabled => {
                    let (text, color) = if form.enabled {
                        ("ON", Color::Green)
                    } else {
                        ("OFF", Color::Red)
                    };
                    Span::styled(
                        text,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )
                }
                ClientCertField::CertPath => render_text_field(&form.cert_path, is_editing),
                ClientCertField::KeyPath => render_text_field(&form.key_path, is_editing),
            };

            let cursor = if is_selected {
                Span::styled("▶ ", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("  ")
            };

            Line::from(vec![cursor, label, value])
        })
        .collect();

    let title = if form.enabled {
        " Client Certificate (mTLS) [ON] "
    } else {
        " Client Certificate (mTLS) [OFF] "
    };

    let border_color = if form.enabled {
        Color::Green
    } else {
        Color::Gray
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let paragraph = Paragraph::new(fields).block(block);
    f.render_widget(paragraph, area);
}
