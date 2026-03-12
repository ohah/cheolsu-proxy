/// Breakpoint 탭 키 핸들러
use crossterm::event::{KeyCode, KeyEvent};

use proxy_daemon::BreakpointAction;

use crate::app::{App, BreakpointAddForm, BreakpointFocus, BreakpointFormField};

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
}
