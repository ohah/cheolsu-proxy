/// Breakpoint 탭 키 핸들러
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use proxy_daemon::BreakpointAction;

use crate::app::{
    App, BreakpointAddForm, BreakpointEditField, BreakpointEditForm, BreakpointFocus,
    BreakpointFormField,
};

impl App {
    pub(in crate::app) async fn handle_breakpoint_key(&mut self, key: KeyEvent) {
        match key.code {
            // 규칙/대기 패널 전환
            KeyCode::Left | KeyCode::Right => {
                self.bp_focus = match self.bp_focus {
                    BreakpointFocus::Rules => BreakpointFocus::Pending,
                    BreakpointFocus::Pending => BreakpointFocus::Rules,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => match self.bp_focus {
                BreakpointFocus::Rules => {
                    let len = self.breakpoint_rules.len();
                    if len > 0 {
                        if let Some(ref mut idx) = self.selected_bp_rule {
                            *idx = idx.saturating_sub(1);
                        } else {
                            self.selected_bp_rule = Some(len.saturating_sub(1));
                        }
                    }
                }
                BreakpointFocus::Pending => {
                    let len = self.pending_breakpoints.len();
                    if len > 0 {
                        if let Some(ref mut idx) = self.selected_pending_bp {
                            *idx = idx.saturating_sub(1);
                        } else {
                            self.selected_pending_bp = Some(len.saturating_sub(1));
                        }
                    }
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match self.bp_focus {
                BreakpointFocus::Rules => {
                    let len = self.breakpoint_rules.len();
                    if len > 0 {
                        if let Some(ref mut idx) = self.selected_bp_rule {
                            if *idx + 1 < len {
                                *idx += 1;
                            }
                        } else {
                            self.selected_bp_rule = Some(0);
                        }
                    }
                }
                BreakpointFocus::Pending => {
                    let len = self.pending_breakpoints.len();
                    if len > 0 {
                        if let Some(ref mut idx) = self.selected_pending_bp {
                            if *idx + 1 < len {
                                *idx += 1;
                            }
                        } else {
                            self.selected_pending_bp = Some(0);
                        }
                    }
                }
            },
            KeyCode::Char('a') if self.bp_focus == BreakpointFocus::Rules => {
                self.bp_add_form = Some(BreakpointAddForm::new());
            }
            KeyCode::Char('t') if self.bp_focus == BreakpointFocus::Rules => {
                if let Some(idx) = self.selected_bp_rule {
                    if idx < self.breakpoint_rules.len() {
                        self.breakpoint_rules[idx].enabled = !self.breakpoint_rules[idx].enabled;
                        self.send_breakpoint_rules_update().await;
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Delete if self.bp_focus == BreakpointFocus::Rules => {
                if let Some(idx) = self.selected_bp_rule {
                    if idx < self.breakpoint_rules.len() {
                        self.breakpoint_rules.remove(idx);
                        if self.breakpoint_rules.is_empty() {
                            self.selected_bp_rule = None;
                        } else if idx >= self.breakpoint_rules.len() {
                            self.selected_bp_rule = Some(self.breakpoint_rules.len() - 1);
                        }
                        self.send_breakpoint_rules_update().await;
                    }
                }
            }
            // 대기 중인 breakpoint 액션
            KeyCode::Char('f') if self.bp_focus == BreakpointFocus::Pending => {
                self.send_breakpoint_resolve(BreakpointAction::Forward)
                    .await;
            }
            KeyCode::Char('x') if self.bp_focus == BreakpointFocus::Pending => {
                self.send_breakpoint_resolve(BreakpointAction::Drop).await;
            }
            KeyCode::Char('b') if self.bp_focus == BreakpointFocus::Pending => {
                self.send_breakpoint_resolve(BreakpointAction::Abort).await;
            }
            KeyCode::Char('e') if self.bp_focus == BreakpointFocus::Pending => {
                if let Some(idx) = self.selected_pending_bp {
                    if let Some(bp) = self.pending_breakpoints.get(idx) {
                        self.bp_edit_form = Some(BreakpointEditForm::from_pending(bp));
                    }
                }
            }
            _ => {}
        }
    }

    pub(in crate::app) async fn handle_bp_add_form_key(&mut self, key: KeyEvent) {
        let Some(form) = self.bp_add_form.as_mut() else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                self.bp_add_form = None;
            }
            KeyCode::Tab => {
                form.field = match form.field {
                    BreakpointFormField::Pattern => BreakpointFormField::BreakOnRequest,
                    BreakpointFormField::BreakOnRequest => BreakpointFormField::BreakOnResponse,
                    BreakpointFormField::BreakOnResponse => BreakpointFormField::Pattern,
                };
            }
            KeyCode::Enter => {
                if form.pattern.is_empty() {
                    self.set_status("Pattern is required");
                } else {
                    let rule = proxy_daemon::BreakpointRule {
                        id: uuid::Uuid::new_v4().to_string(),
                        pattern: form.pattern.clone(),
                        break_on_request: form.break_on_request,
                        break_on_response: form.break_on_response,
                        enabled: true,
                    };
                    self.breakpoint_rules.push(rule);
                    self.send_breakpoint_rules_update().await;
                    self.set_status("Breakpoint rule added");
                    self.bp_add_form = None;
                }
            }
            _ => match form.field {
                BreakpointFormField::Pattern => match key.code {
                    KeyCode::Char(c) => form.pattern.push(c),
                    KeyCode::Backspace => {
                        form.pattern.pop();
                    }
                    _ => {}
                },
                BreakpointFormField::BreakOnRequest => {
                    if matches!(
                        key.code,
                        KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right
                    ) {
                        form.break_on_request = !form.break_on_request;
                    }
                }
                BreakpointFormField::BreakOnResponse => {
                    if matches!(
                        key.code,
                        KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right
                    ) {
                        form.break_on_response = !form.break_on_response;
                    }
                }
            },
        }
    }

    pub(in crate::app) async fn handle_bp_edit_form_key(&mut self, key: KeyEvent) {
        let Some(form) = self.bp_edit_form.as_mut() else {
            return;
        };

        // Shift+Enter: 현재 필드에 개행 삽입
        if key.code == KeyCode::Enter && key.modifiers.contains(KeyModifiers::SHIFT) {
            match form.field {
                BreakpointEditField::Headers => form.headers_text.push('\n'),
                BreakpointEditField::Body => form.body.push('\n'),
                BreakpointEditField::Status => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.bp_edit_form = None;
            }
            KeyCode::Tab => {
                form.field = match form.field {
                    BreakpointEditField::Headers => BreakpointEditField::Body,
                    BreakpointEditField::Body => {
                        if form.phase == proxy_daemon::BreakpointPhase::Response {
                            BreakpointEditField::Status
                        } else {
                            BreakpointEditField::Headers
                        }
                    }
                    BreakpointEditField::Status => BreakpointEditField::Headers,
                };
            }
            KeyCode::Enter => {
                let headers = form.parse_headers();
                let body = if form.body.is_empty() {
                    None
                } else {
                    Some(form.body.clone())
                };
                let status = if form.status.is_empty() {
                    None
                } else {
                    match form.status.parse::<u16>() {
                        Ok(s) if (100..=599).contains(&s) => Some(s),
                        _ => {
                            self.set_status("Invalid status code (100-599)");
                            return;
                        }
                    }
                };

                let action = BreakpointAction::ModifyAndForward {
                    headers: if headers.is_empty() {
                        None
                    } else {
                        Some(headers)
                    },
                    body,
                    status,
                };

                // bp_edit_form에서 bp_id를 꺼내서 직접 resolve
                let bp_id = form.bp_id.clone();
                self.bp_edit_form = None;

                if let Some(pos) = self.pending_breakpoints.iter().position(|b| b.id == bp_id) {
                    if let Some(conn) = &self.conn {
                        let cmd =
                            proxy_daemon::ClientCommand::ResolveBreakpoint { id: bp_id, action };
                        let _ = conn.send_command(&cmd).await;
                    }
                    self.pending_breakpoints.remove(pos);
                    if self.pending_breakpoints.is_empty() {
                        self.selected_pending_bp = None;
                    } else if pos >= self.pending_breakpoints.len() {
                        self.selected_pending_bp = Some(self.pending_breakpoints.len() - 1);
                    }
                    self.set_status("Breakpoint modified & forwarded");
                } else {
                    self.set_status("Breakpoint is no longer pending");
                }
            }
            _ => match form.field {
                BreakpointEditField::Headers => match key.code {
                    KeyCode::Char(c) => form.headers_text.push(c),
                    KeyCode::Backspace => {
                        form.headers_text.pop();
                    }
                    _ => {}
                },
                BreakpointEditField::Body => match key.code {
                    KeyCode::Char(c) => form.body.push(c),
                    KeyCode::Backspace => {
                        form.body.pop();
                    }
                    _ => {}
                },
                BreakpointEditField::Status => match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() => form.status.push(c),
                    KeyCode::Backspace => {
                        form.status.pop();
                    }
                    _ => {}
                },
            },
        }
    }
}
