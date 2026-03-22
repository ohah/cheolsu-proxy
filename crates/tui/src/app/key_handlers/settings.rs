/// Settings 탭 키 핸들러
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::forms::{
    ClientCertField, HostMappingField, ProxyAuthField, QuickSettingsField, SettingsSection,
    SslProxyingAddForm, ThrottleField, UpstreamProxyField,
};
use crate::app::App;

impl App {
    pub(in crate::app) async fn handle_settings_key(&mut self, key: KeyEvent) {
        // SSL Proxying add form 열려있으면 해당 핸들러로
        if self.ssl_proxying_add_form.is_some() {
            self.handle_ssl_proxying_add_form_key(key).await;
            return;
        }

        // Host Mapping form 열려있으면 해당 핸들러로
        if self.host_mapping_form.is_some() {
            self.handle_host_mapping_form_key(key).await;
            return;
        }

        // 텍스트 편집 모드 (upstream, throttle, client cert 등)
        let is_editing = match self.settings_section {
            SettingsSection::UpstreamProxy => self.upstream_form.editing,
            SettingsSection::ProxyAuth => self.proxy_auth_form.editing,
            SettingsSection::Throttle => self.throttle_form.editing,
            SettingsSection::ClientCertificate => self.client_cert_form.editing,
            SettingsSection::HostMapping
            | SettingsSection::QuickSettings
            | SettingsSection::SslProxying => false,
        };

        if is_editing {
            match self.settings_section {
                SettingsSection::UpstreamProxy => match key.code {
                    KeyCode::Esc => {
                        self.upstream_form.editing = false;
                    }
                    KeyCode::Enter => {
                        self.upstream_form.editing = false;
                        self.send_upstream_update().await;
                    }
                    KeyCode::Char(c) => {
                        let field = match self.upstream_form.field {
                            UpstreamProxyField::Host => &mut self.upstream_form.host,
                            UpstreamProxyField::Port => &mut self.upstream_form.port,
                            UpstreamProxyField::Username => &mut self.upstream_form.username,
                            UpstreamProxyField::Password => &mut self.upstream_form.password,
                            UpstreamProxyField::Bypass => &mut self.upstream_form.bypass,
                            _ => return,
                        };
                        field.push(c);
                    }
                    KeyCode::Backspace => {
                        let field = match self.upstream_form.field {
                            UpstreamProxyField::Host => &mut self.upstream_form.host,
                            UpstreamProxyField::Port => &mut self.upstream_form.port,
                            UpstreamProxyField::Username => &mut self.upstream_form.username,
                            UpstreamProxyField::Password => &mut self.upstream_form.password,
                            UpstreamProxyField::Bypass => &mut self.upstream_form.bypass,
                            _ => return,
                        };
                        field.pop();
                    }
                    _ => {}
                },
                SettingsSection::Throttle => {
                    match key.code {
                        KeyCode::Esc => {
                            self.throttle_form.editing = false;
                        }
                        KeyCode::Enter => {
                            self.throttle_form.editing = false;
                            self.send_throttle_update().await;
                        }
                        KeyCode::Char(c) => {
                            let field = match self.throttle_form.field {
                                ThrottleField::Download => &mut self.throttle_form.download,
                                ThrottleField::Upload => &mut self.throttle_form.upload,
                                ThrottleField::Latency => &mut self.throttle_form.latency,
                                _ => return,
                            };
                            // 숫자만 허용
                            if c.is_ascii_digit() {
                                field.push(c);
                            }
                        }
                        KeyCode::Backspace => {
                            let field = match self.throttle_form.field {
                                ThrottleField::Download => &mut self.throttle_form.download,
                                ThrottleField::Upload => &mut self.throttle_form.upload,
                                ThrottleField::Latency => &mut self.throttle_form.latency,
                                _ => return,
                            };
                            field.pop();
                        }
                        _ => {}
                    }
                }
                SettingsSection::ProxyAuth => match key.code {
                    KeyCode::Esc => {
                        self.proxy_auth_form.editing = false;
                    }
                    KeyCode::Enter => {
                        self.proxy_auth_form.editing = false;
                        self.send_proxy_auth_update().await;
                    }
                    KeyCode::Char(c) => {
                        let field = match self.proxy_auth_form.field {
                            ProxyAuthField::Username => &mut self.proxy_auth_form.username,
                            ProxyAuthField::Password => &mut self.proxy_auth_form.password,
                            _ => return,
                        };
                        field.push(c);
                    }
                    KeyCode::Backspace => {
                        let field = match self.proxy_auth_form.field {
                            ProxyAuthField::Username => &mut self.proxy_auth_form.username,
                            ProxyAuthField::Password => &mut self.proxy_auth_form.password,
                            _ => return,
                        };
                        field.pop();
                    }
                    _ => {}
                },
                SettingsSection::ClientCertificate => match key.code {
                    KeyCode::Esc => {
                        self.client_cert_form.editing = false;
                    }
                    KeyCode::Enter => {
                        self.client_cert_form.editing = false;
                        self.send_client_cert_update().await;
                    }
                    KeyCode::Char(c) => {
                        let field = match self.client_cert_form.field {
                            ClientCertField::CertPath => &mut self.client_cert_form.cert_path,
                            ClientCertField::KeyPath => &mut self.client_cert_form.key_path,
                            _ => return,
                        };
                        field.push(c);
                    }
                    KeyCode::Backspace => {
                        let field = match self.client_cert_form.field {
                            ClientCertField::CertPath => &mut self.client_cert_form.cert_path,
                            ClientCertField::KeyPath => &mut self.client_cert_form.key_path,
                            _ => return,
                        };
                        field.pop();
                    }
                    _ => {}
                },
                SettingsSection::HostMapping
                | SettingsSection::QuickSettings
                | SettingsSection::SslProxying => {
                    // HostMapping, QuickSettings, SslProxying은 editing 모드가 없음
                }
            }
            return;
        }

        // 네비게이션 모드
        match key.code {
            // 섹션 전환: H/L 또는 Left/Right
            KeyCode::Char('H') | KeyCode::Char('h') if !is_editing => {
                self.settings_section = self.settings_section.prev();
            }
            KeyCode::Char('L') | KeyCode::Char('l') if !is_editing => {
                self.settings_section = self.settings_section.next();
            }

            KeyCode::Up | KeyCode::Char('k') => match self.settings_section {
                SettingsSection::UpstreamProxy => {
                    self.upstream_form.field = self.upstream_form.field.prev();
                }
                SettingsSection::ProxyAuth => {
                    self.proxy_auth_form.field = self.proxy_auth_form.field.prev();
                }
                SettingsSection::Throttle => {
                    self.throttle_form.field = self.throttle_form.field.prev();
                }
                SettingsSection::HostMapping => {
                    let len = self.host_mappings.len();
                    if len > 0 {
                        if let Some(ref mut idx) = self.selected_host_mapping {
                            *idx = idx.saturating_sub(1);
                        } else {
                            self.selected_host_mapping = Some(len.saturating_sub(1));
                        }
                    }
                }
                SettingsSection::QuickSettings => {
                    self.quick_settings_form.field = self.quick_settings_form.field.prev();
                }
                SettingsSection::SslProxying => {
                    let len = self.ssl_proxying_entries.len();
                    if len > 0 {
                        if let Some(ref mut idx) = self.selected_ssl_proxying {
                            *idx = idx.saturating_sub(1);
                        } else {
                            self.selected_ssl_proxying = Some(len.saturating_sub(1));
                        }
                    }
                }
                SettingsSection::ClientCertificate => {
                    self.client_cert_form.field = self.client_cert_form.field.prev();
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match self.settings_section {
                SettingsSection::UpstreamProxy => {
                    self.upstream_form.field = self.upstream_form.field.next();
                }
                SettingsSection::ProxyAuth => {
                    self.proxy_auth_form.field = self.proxy_auth_form.field.next();
                }
                SettingsSection::Throttle => {
                    self.throttle_form.field = self.throttle_form.field.next();
                }
                SettingsSection::HostMapping => {
                    let len = self.host_mappings.len();
                    if len > 0 {
                        if let Some(ref mut idx) = self.selected_host_mapping {
                            if *idx + 1 < len {
                                *idx += 1;
                            }
                        } else {
                            self.selected_host_mapping = Some(0);
                        }
                    }
                }
                SettingsSection::QuickSettings => {
                    self.quick_settings_form.field = self.quick_settings_form.field.next();
                }
                SettingsSection::SslProxying => {
                    let len = self.ssl_proxying_entries.len();
                    if len > 0 {
                        if let Some(ref mut idx) = self.selected_ssl_proxying {
                            if *idx + 1 < len {
                                *idx += 1;
                            }
                        } else {
                            self.selected_ssl_proxying = Some(0);
                        }
                    }
                }
                SettingsSection::ClientCertificate => {
                    self.client_cert_form.field = self.client_cert_form.field.next();
                }
            },
            KeyCode::Enter | KeyCode::Char(' ') => match self.settings_section {
                SettingsSection::UpstreamProxy => {
                    if self.upstream_form.field == UpstreamProxyField::Enabled {
                        self.upstream_form.enabled = !self.upstream_form.enabled;
                        self.send_upstream_update().await;
                    } else {
                        self.upstream_form.editing = true;
                    }
                }
                SettingsSection::ProxyAuth => {
                    if self.proxy_auth_form.field == ProxyAuthField::Enabled {
                        self.proxy_auth_form.enabled = !self.proxy_auth_form.enabled;
                        self.send_proxy_auth_update().await;
                    } else {
                        self.proxy_auth_form.editing = true;
                    }
                }
                SettingsSection::Throttle => match self.throttle_form.field {
                    ThrottleField::Enabled => {
                        self.throttle_form.enabled = !self.throttle_form.enabled;
                        self.send_throttle_update().await;
                    }
                    ThrottleField::Preset => {
                        self.throttle_form.preset = self.throttle_form.preset.next();
                        self.send_throttle_update().await;
                    }
                    ThrottleField::Download | ThrottleField::Upload | ThrottleField::Latency => {
                        self.throttle_form.editing = true;
                    }
                },
                SettingsSection::HostMapping => {}
                SettingsSection::SslProxying => {}
                SettingsSection::ClientCertificate => {
                    if self.client_cert_form.field == ClientCertField::Enabled {
                        self.client_cert_form.enabled = !self.client_cert_form.enabled;
                        self.send_client_cert_update().await;
                    } else {
                        self.client_cert_form.editing = true;
                    }
                }
                SettingsSection::QuickSettings => match self.quick_settings_form.field {
                    QuickSettingsField::NoCaching => {
                        self.quick_settings_form.no_caching = !self.quick_settings_form.no_caching;
                        self.send_quick_settings_update().await;
                    }
                    QuickSettingsField::BlockCookies => {
                        self.quick_settings_form.block_cookies =
                            !self.quick_settings_form.block_cookies;
                        self.send_quick_settings_update().await;
                    }
                    QuickSettingsField::NoGzip => {
                        self.quick_settings_form.no_gzip = !self.quick_settings_form.no_gzip;
                        self.send_quick_settings_update().await;
                    }
                    QuickSettingsField::BlockQuic => {
                        self.quick_settings_form.block_quic = !self.quick_settings_form.block_quic;
                        self.send_quick_settings_update().await;
                    }
                },
            },
            // Host Mapping: a=add, d=delete, t=toggle
            KeyCode::Char('a') if self.settings_section == SettingsSection::HostMapping => {
                self.host_mapping_form = Some(crate::app::forms::HostMappingForm::new());
            }
            KeyCode::Char('d') | KeyCode::Delete
                if self.settings_section == SettingsSection::HostMapping =>
            {
                if let Some(idx) = self.selected_host_mapping {
                    if idx < self.host_mappings.len() {
                        self.host_mappings.remove(idx);
                        if self.host_mappings.is_empty() {
                            self.selected_host_mapping = None;
                        } else if idx >= self.host_mappings.len() {
                            self.selected_host_mapping = Some(self.host_mappings.len() - 1);
                        }
                        self.send_host_mappings_update().await;
                    }
                }
            }
            KeyCode::Char('t') if self.settings_section == SettingsSection::HostMapping => {
                if let Some(idx) = self.selected_host_mapping {
                    if idx < self.host_mappings.len() {
                        self.host_mappings[idx].enabled = !self.host_mappings[idx].enabled;
                        self.send_host_mappings_update().await;
                    }
                }
            }
            // SSL Proxying: a=add, d=delete, t=toggle
            KeyCode::Char('a') if self.settings_section == SettingsSection::SslProxying => {
                self.ssl_proxying_add_form = Some(SslProxyingAddForm::new());
            }
            KeyCode::Char('d') | KeyCode::Delete
                if self.settings_section == SettingsSection::SslProxying =>
            {
                if let Some(idx) = self.selected_ssl_proxying {
                    if idx < self.ssl_proxying_entries.len() {
                        self.ssl_proxying_entries.remove(idx);
                        if self.ssl_proxying_entries.is_empty() {
                            self.selected_ssl_proxying = None;
                        } else if idx >= self.ssl_proxying_entries.len() {
                            self.selected_ssl_proxying = Some(self.ssl_proxying_entries.len() - 1);
                        }
                        self.send_ssl_proxying_update().await;
                    }
                }
            }
            KeyCode::Char('t') if self.settings_section == SettingsSection::SslProxying => {
                if let Some(idx) = self.selected_ssl_proxying {
                    if idx < self.ssl_proxying_entries.len() {
                        self.ssl_proxying_entries[idx].enabled =
                            !self.ssl_proxying_entries[idx].enabled;
                        self.send_ssl_proxying_update().await;
                    }
                }
            }
            // SSL Proxying: m=toggle mode (blacklist/whitelist)
            KeyCode::Char('m') if self.settings_section == SettingsSection::SslProxying => {
                self.ssl_proxying_mode = match self.ssl_proxying_mode {
                    proxy_daemon::SslProxyingMode::Blacklist => {
                        proxy_daemon::SslProxyingMode::Whitelist
                    }
                    proxy_daemon::SslProxyingMode::Whitelist => {
                        proxy_daemon::SslProxyingMode::Blacklist
                    }
                };
                self.send_ssl_proxying_update().await;
            }
            KeyCode::Left => {
                if self.settings_section == SettingsSection::Throttle
                    && self.throttle_form.field == ThrottleField::Preset
                {
                    self.throttle_form.preset = self.throttle_form.preset.prev();
                    self.send_throttle_update().await;
                } else {
                    self.settings_section = self.settings_section.prev();
                }
            }
            KeyCode::Right => {
                if self.settings_section == SettingsSection::Throttle
                    && self.throttle_form.field == ThrottleField::Preset
                {
                    self.throttle_form.preset = self.throttle_form.preset.next();
                    self.send_throttle_update().await;
                } else {
                    self.settings_section = self.settings_section.next();
                }
            }
            KeyCode::Char('i') => {
                self.install_ca_cert();
            }
            KeyCode::Char('U') => {
                self.uninstall_ca_cert();
            }
            _ => {}
        }
    }

    /// Host Mapping 폼 키 핸들러
    async fn handle_host_mapping_form_key(&mut self, key: KeyEvent) {
        let Some(form) = self.host_mapping_form.as_mut() else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                self.host_mapping_form = None;
            }
            KeyCode::Tab => {
                form.field = form.field.next();
            }
            KeyCode::BackTab => {
                form.field = form.field.prev();
            }
            KeyCode::Enter => {
                if let Some(mapping) = form.to_mapping() {
                    self.host_mappings.push(mapping);
                    self.send_host_mappings_update().await;
                    self.set_status("Host mapping added");
                    self.host_mapping_form = None;
                } else {
                    self.set_status("Source host and target host are required");
                }
            }
            KeyCode::Char(c) => {
                let field = match form.field {
                    HostMappingField::SourceHost => &mut form.source_host,
                    HostMappingField::SourcePort => &mut form.source_port,
                    HostMappingField::TargetHost => &mut form.target_host,
                    HostMappingField::TargetPort => &mut form.target_port,
                };
                // 포트 필드는 숫자만 허용
                match form.field {
                    HostMappingField::SourcePort | HostMappingField::TargetPort => {
                        if c.is_ascii_digit() {
                            field.push(c);
                        }
                    }
                    _ => {
                        field.push(c);
                    }
                }
            }
            KeyCode::Backspace => {
                let field = match form.field {
                    HostMappingField::SourceHost => &mut form.source_host,
                    HostMappingField::SourcePort => &mut form.source_port,
                    HostMappingField::TargetHost => &mut form.target_host,
                    HostMappingField::TargetPort => &mut form.target_port,
                };
                field.pop();
            }
            _ => {}
        }
    }

    /// SSL Proxying 추가 폼 키 핸들러
    async fn handle_ssl_proxying_add_form_key(&mut self, key: KeyEvent) {
        let Some(form) = self.ssl_proxying_add_form.as_mut() else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                self.ssl_proxying_add_form = None;
            }
            KeyCode::Enter => {
                if let Some(entry) = form.to_entry() {
                    self.ssl_proxying_entries.push(entry);
                    self.send_ssl_proxying_update().await;
                    self.set_status("SSL Proxying entry added");
                    self.ssl_proxying_add_form = None;
                } else {
                    self.set_status("Pattern is required");
                }
            }
            KeyCode::Char(c) => {
                form.pattern.push(c);
            }
            KeyCode::Backspace => {
                form.pattern.pop();
            }
            _ => {}
        }
    }
}
