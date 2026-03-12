/// Settings 탭 - 키바인딩 도움말 렌더링
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, SettingsSection};

pub(super) fn draw_keybindings(f: &mut Frame, app: &App, area: Rect) {
    let editing = app.upstream_form.editing
        || app.throttle_form.editing
        || app.proxy_auth_form.editing
        || app.client_cert_form.editing;
    let in_host_mapping_form = app.host_mapping_form.is_some();
    let in_ssl_proxying_form = app.ssl_proxying_add_form.is_some();

    let help_lines = if in_ssl_proxying_form {
        vec![
            Line::from(Span::styled(
                "Add SSL Proxying Pattern",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  Enter              Save entry"),
            Line::from("  Esc                Cancel"),
            Line::from("  Type               Input pattern (e.g. *.example.com)"),
        ]
    } else if in_host_mapping_form {
        vec![
            Line::from(Span::styled(
                "Add Host Mapping",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  Tab / Shift+Tab    Next / Previous field"),
            Line::from("  Enter              Save mapping"),
            Line::from("  Esc                Cancel"),
            Line::from("  Type               Input text"),
        ]
    } else if app.settings_section == SettingsSection::SslProxying && !editing {
        vec![
            Line::from(Span::styled(
                "SSL Proxying",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  j / k / ↑ / ↓      Navigate entries"),
            Line::from("  a                  Add pattern"),
            Line::from("  d / Delete         Delete pattern"),
            Line::from("  t                  Toggle enabled/disabled"),
            Line::from("  m                  Toggle mode (Blacklist/Whitelist)"),
            Line::from("  h / l              Switch section"),
        ]
    } else if app.settings_section == SettingsSection::HostMapping && !editing {
        vec![
            Line::from(Span::styled(
                "Host Mapping",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  j / k / ↑ / ↓      Navigate mappings"),
            Line::from("  a                  Add mapping"),
            Line::from("  d / Delete         Delete mapping"),
            Line::from("  t                  Toggle enabled/disabled"),
            Line::from("  h / l              Switch section"),
        ]
    } else if editing {
        vec![
            Line::from(Span::styled(
                "Editing",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  Enter          Apply & exit edit"),
            Line::from("  Esc            Cancel edit"),
            Line::from("  Type           Input text"),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "Keybindings",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  j / k / ↑ / ↓      Navigate fields"),
            Line::from("  Enter / Space      Toggle (Enabled) / Edit (text fields)"),
            Line::from("  i                  Install CA certificate"),
            Line::from("  U                  Uninstall CA certificate"),
            Line::from("  Tab / Shift+Tab    Switch tabs"),
            Line::from("  q / Ctrl+C         Quit"),
        ]
    };

    let help = Paragraph::new(help_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(" Keybindings "),
    );

    f.render_widget(help, area);
}
