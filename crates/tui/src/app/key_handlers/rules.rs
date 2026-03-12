/// Intercept Rules 탭 키 핸들러
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::forms::RuleForm;
use crate::app::App;

impl App {
    pub(in crate::app) async fn handle_rules_key(&mut self, key: KeyEvent) {
        let len = self.rules.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                if let Some(ref mut idx) = self.selected_rule {
                    *idx = idx.saturating_sub(1);
                } else {
                    self.selected_rule = Some(len.saturating_sub(1));
                }
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                if let Some(ref mut idx) = self.selected_rule {
                    if *idx + 1 < len {
                        *idx += 1;
                    }
                } else {
                    self.selected_rule = Some(0);
                }
            }
            KeyCode::Char('a') => {
                // Open add rule form
                self.rule_form = Some(RuleForm::new());
            }
            KeyCode::Char('t') => {
                // Toggle enabled/disabled
                if let Some(idx) = self.selected_rule {
                    if idx < self.rules.len() {
                        self.rules[idx].enabled = !self.rules[idx].enabled;
                        self.send_rules_update().await;
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                // Delete rule
                if let Some(idx) = self.selected_rule {
                    if idx < self.rules.len() {
                        self.rules.remove(idx);
                        if self.rules.is_empty() {
                            self.selected_rule = None;
                        } else if idx >= self.rules.len() {
                            self.selected_rule = Some(self.rules.len() - 1);
                        }
                        self.send_rules_update().await;
                    }
                }
            }
            KeyCode::Char('C') => {
                // Clear all rules
                self.rules.clear();
                self.selected_rule = None;
                self.send_rules_update().await;
            }
            _ => {}
        }
    }

    pub(in crate::app) async fn handle_rule_form_key(&mut self, key: KeyEvent) {
        let Some(form) = self.rule_form.as_mut() else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                self.rule_form = None;
            }
            KeyCode::Tab => {
                form.field = form.field.next(form.action_type);
            }
            KeyCode::BackTab => {
                form.field = form.field.prev(form.action_type);
            }
            KeyCode::Enter => {
                // Submit form
                if let Some(rule) = form.to_rule() {
                    self.rules.push(rule);
                    self.send_rules_update().await;
                    self.set_status("Rule added");
                    self.rule_form = None;
                } else {
                    self.set_status("Pattern is required");
                }
            }
            _ => {
                // Handle text input for current field
                match form.field {
                    super::super::forms::RuleFormField::ActionType => match key.code {
                        KeyCode::Left => form.action_type = form.action_type.prev(),
                        KeyCode::Right => form.action_type = form.action_type.next(),
                        _ => {}
                    },
                    super::super::forms::RuleFormField::Method => match key.code {
                        KeyCode::Left | KeyCode::Right => {
                            let methods = [
                                None,
                                Some("GET"),
                                Some("POST"),
                                Some("PUT"),
                                Some("DELETE"),
                                Some("PATCH"),
                            ];
                            let cur = methods
                                .iter()
                                .position(|m| *m == form.method.as_deref())
                                .unwrap_or(0);
                            let next = if key.code == KeyCode::Right {
                                (cur + 1) % methods.len()
                            } else if cur == 0 {
                                methods.len() - 1
                            } else {
                                cur - 1
                            };
                            form.method = methods[next].map(|s| s.to_string());
                        }
                        _ => {}
                    },
                    _ => {
                        let field = match form.field {
                            super::super::forms::RuleFormField::Name => &mut form.name,
                            super::super::forms::RuleFormField::Pattern => &mut form.pattern,
                            super::super::forms::RuleFormField::StatusCode => &mut form.status_code,
                            super::super::forms::RuleFormField::Body => &mut form.body,
                            super::super::forms::RuleFormField::TargetUrl => &mut form.target_url,
                            super::super::forms::RuleFormField::FilePath => &mut form.file_path,
                            _ => return,
                        };
                        match key.code {
                            KeyCode::Char(c) => field.push(c),
                            KeyCode::Backspace => {
                                field.pop();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
