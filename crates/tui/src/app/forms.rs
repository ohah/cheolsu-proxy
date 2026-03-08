use proxy_daemon::{InterceptAction, InterceptRule, UpstreamProxyAuth, UpstreamProxyConfig};

/// 스크립트 로그 엔트리 (TUI 표시용)
#[derive(Debug, Clone)]
pub struct ScriptLogEntry {
    pub level: String,
    pub message: String,
    pub time: std::time::Instant,
}

/// Upstream proxy settings form
#[derive(Debug, Clone)]
pub struct UpstreamProxyForm {
    pub enabled: bool,
    pub field: UpstreamProxyField,
    pub editing: bool,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub bypass: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamProxyField {
    Enabled,
    Host,
    Port,
    Username,
    Password,
    Bypass,
}

impl UpstreamProxyField {
    pub const ALL: [UpstreamProxyField; 6] = [
        Self::Enabled,
        Self::Host,
        Self::Port,
        Self::Username,
        Self::Password,
        Self::Bypass,
    ];

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|f| f == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(&self) -> Self {
        let idx = Self::ALL.iter().position(|f| f == self).unwrap_or(0);
        if idx == 0 {
            Self::ALL[Self::ALL.len() - 1]
        } else {
            Self::ALL[idx - 1]
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Enabled => "Enabled",
            Self::Host => "Host",
            Self::Port => "Port",
            Self::Username => "Username",
            Self::Password => "Password",
            Self::Bypass => "Bypass",
        }
    }
}

impl UpstreamProxyForm {
    pub fn new() -> Self {
        Self {
            enabled: false,
            field: UpstreamProxyField::Enabled,
            editing: false,
            host: String::new(),
            port: "8080".to_string(),
            username: String::new(),
            password: String::new(),
            bypass: "localhost".to_string(),
        }
    }

    pub fn to_config(&self) -> Option<UpstreamProxyConfig> {
        if !self.enabled || self.host.is_empty() {
            return None;
        }
        let auth = if !self.username.is_empty() {
            Some(UpstreamProxyAuth {
                username: self.username.clone(),
                password: self.password.clone(),
            })
        } else {
            None
        };
        let bypass = self
            .bypass
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Some(UpstreamProxyConfig {
            host: self.host.clone(),
            port: self.port.parse().unwrap_or(8080),
            auth,
            bypass,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WsConnection {
    pub connection_id: String,
    pub uri: String,
    pub time: i64,
    pub active: bool,
}

/// Rule creation form
#[derive(Debug, Clone)]
pub struct RuleForm {
    pub field: RuleFormField,
    pub name: String,
    pub pattern: String,
    pub method: Option<String>,
    pub action_type: ActionType,
    pub status_code: String,
    pub body: String,
    pub target_url: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleFormField {
    Name,
    Pattern,
    Method,
    ActionType,
    StatusCode,
    Body,
    TargetUrl,
    FilePath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    Block,
    ModifyRequest,
    ModifyResponse,
    MapLocal,
    MapRemote,
}

impl ActionType {
    pub const ALL: [ActionType; 5] = [
        ActionType::Block,
        ActionType::ModifyRequest,
        ActionType::ModifyResponse,
        ActionType::MapLocal,
        ActionType::MapRemote,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ActionType::Block => "Block",
            ActionType::ModifyRequest => "Modify Request",
            ActionType::ModifyResponse => "Modify Response",
            ActionType::MapLocal => "Map Local",
            ActionType::MapRemote => "Map Remote",
        }
    }

    pub fn next(&self) -> ActionType {
        let idx = Self::ALL.iter().position(|a| a == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(&self) -> ActionType {
        let idx = Self::ALL.iter().position(|a| a == self).unwrap_or(0);
        if idx == 0 {
            Self::ALL[Self::ALL.len() - 1]
        } else {
            Self::ALL[idx - 1]
        }
    }
}

impl RuleFormField {
    pub(crate) fn next(&self, action_type: ActionType) -> RuleFormField {
        match self {
            RuleFormField::Name => RuleFormField::Pattern,
            RuleFormField::Pattern => RuleFormField::Method,
            RuleFormField::Method => RuleFormField::ActionType,
            RuleFormField::ActionType => match action_type {
                ActionType::Block => RuleFormField::StatusCode,
                ActionType::ModifyRequest | ActionType::ModifyResponse => RuleFormField::Body,
                ActionType::MapLocal => RuleFormField::FilePath,
                ActionType::MapRemote => RuleFormField::TargetUrl,
            },
            RuleFormField::StatusCode => RuleFormField::Body,
            RuleFormField::Body => RuleFormField::Name,
            RuleFormField::TargetUrl => RuleFormField::Name,
            RuleFormField::FilePath => RuleFormField::StatusCode,
        }
    }

    pub(crate) fn prev(&self, action_type: ActionType) -> RuleFormField {
        match self {
            RuleFormField::Name => match action_type {
                ActionType::Block | ActionType::ModifyResponse => RuleFormField::Body,
                ActionType::ModifyRequest => RuleFormField::Body,
                ActionType::MapLocal => RuleFormField::StatusCode,
                ActionType::MapRemote => RuleFormField::TargetUrl,
            },
            RuleFormField::Pattern => RuleFormField::Name,
            RuleFormField::Method => RuleFormField::Pattern,
            RuleFormField::ActionType => RuleFormField::Method,
            RuleFormField::StatusCode => match action_type {
                ActionType::MapLocal => RuleFormField::FilePath,
                _ => RuleFormField::ActionType,
            },
            RuleFormField::Body => match action_type {
                ActionType::Block => RuleFormField::StatusCode,
                _ => RuleFormField::ActionType,
            },
            RuleFormField::TargetUrl => RuleFormField::ActionType,
            RuleFormField::FilePath => RuleFormField::ActionType,
        }
    }
}

impl RuleForm {
    pub(crate) fn new() -> Self {
        Self {
            field: RuleFormField::Name,
            name: String::new(),
            pattern: String::new(),
            method: None,
            action_type: ActionType::Block,
            status_code: "403".to_string(),
            body: String::new(),
            target_url: String::new(),
            file_path: String::new(),
        }
    }

    pub(crate) fn to_rule(&self) -> Option<InterceptRule> {
        if self.pattern.is_empty() {
            return None;
        }

        let action = match self.action_type {
            ActionType::Block => InterceptAction::Block {
                status_code: self.status_code.parse().unwrap_or(403),
                body: self.body.clone(),
            },
            ActionType::ModifyRequest => InterceptAction::ModifyRequest {
                add_headers: std::collections::HashMap::new(),
                remove_headers: Vec::new(),
                set_body: if self.body.is_empty() {
                    None
                } else {
                    Some(self.body.clone())
                },
            },
            ActionType::ModifyResponse => InterceptAction::ModifyResponse {
                set_status: self.status_code.parse().ok(),
                add_headers: std::collections::HashMap::new(),
                remove_headers: Vec::new(),
                set_body: if self.body.is_empty() {
                    None
                } else {
                    Some(self.body.clone())
                },
            },
            ActionType::MapLocal => InterceptAction::MapLocal {
                file_path: self.file_path.clone(),
                status_code: self.status_code.parse().unwrap_or(200),
                headers: std::collections::HashMap::new(),
            },
            ActionType::MapRemote => InterceptAction::MapRemote {
                target_url: self.target_url.clone(),
                preserve_path: true,
            },
        };

        Some(InterceptRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: if self.name.is_empty() {
                self.pattern.clone()
            } else {
                self.name.clone()
            },
            enabled: true,
            pattern: self.pattern.clone(),
            method: self.method.clone(),
            action,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- UpstreamProxyField --

    #[test]
    fn field_next_wraps_around() {
        assert_eq!(UpstreamProxyField::Enabled.next(), UpstreamProxyField::Host);
        assert_eq!(
            UpstreamProxyField::Bypass.next(),
            UpstreamProxyField::Enabled
        );
    }

    #[test]
    fn field_prev_wraps_around() {
        assert_eq!(
            UpstreamProxyField::Enabled.prev(),
            UpstreamProxyField::Bypass
        );
        assert_eq!(UpstreamProxyField::Host.prev(), UpstreamProxyField::Enabled);
    }

    #[test]
    fn field_next_prev_full_cycle() {
        let mut field = UpstreamProxyField::Enabled;
        for _ in 0..UpstreamProxyField::ALL.len() {
            field = field.next();
        }
        assert_eq!(field, UpstreamProxyField::Enabled);

        for _ in 0..UpstreamProxyField::ALL.len() {
            field = field.prev();
        }
        assert_eq!(field, UpstreamProxyField::Enabled);
    }

    #[test]
    fn field_labels_not_empty() {
        for field in UpstreamProxyField::ALL {
            assert!(!field.label().is_empty());
        }
    }

    // -- UpstreamProxyForm defaults --

    #[test]
    fn form_new_defaults() {
        let form = UpstreamProxyForm::new();
        assert!(!form.enabled);
        assert_eq!(form.field, UpstreamProxyField::Enabled);
        assert!(!form.editing);
        assert!(form.host.is_empty());
        assert_eq!(form.port, "8080");
        assert!(form.username.is_empty());
        assert!(form.password.is_empty());
        assert_eq!(form.bypass, "localhost");
    }

    // -- to_config --

    #[test]
    fn to_config_returns_none_when_disabled() {
        let mut form = UpstreamProxyForm::new();
        form.host = "proxy.example.com".to_string();
        assert!(form.to_config().is_none());
    }

    #[test]
    fn to_config_returns_none_when_host_empty() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        assert!(form.to_config().is_none());
    }

    #[test]
    fn to_config_basic() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.port = "3128".to_string();

        let config = form.to_config().unwrap();
        assert_eq!(config.host, "proxy.example.com");
        assert_eq!(config.port, 3128);
        assert!(config.auth.is_none());
        assert_eq!(config.bypass, vec!["localhost"]);
    }

    #[test]
    fn to_config_with_auth() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.username = "user".to_string();
        form.password = "pass".to_string();

        let config = form.to_config().unwrap();
        let auth = config.auth.unwrap();
        assert_eq!(auth.username, "user");
        assert_eq!(auth.password, "pass");
    }

    #[test]
    fn to_config_no_auth_when_username_empty() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.password = "pass".to_string();

        let config = form.to_config().unwrap();
        assert!(config.auth.is_none());
    }

    #[test]
    fn to_config_bypass_parsing() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.bypass = "localhost, *.internal.com, 10.0.0.1".to_string();

        let config = form.to_config().unwrap();
        assert_eq!(
            config.bypass,
            vec!["localhost", "*.internal.com", "10.0.0.1"]
        );
    }

    #[test]
    fn to_config_bypass_empty_string() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.bypass = "".to_string();

        let config = form.to_config().unwrap();
        assert!(config.bypass.is_empty());
    }

    #[test]
    fn to_config_bypass_trims_whitespace() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.bypass = "  localhost ,  *.test.com  ".to_string();

        let config = form.to_config().unwrap();
        assert_eq!(config.bypass, vec!["localhost", "*.test.com"]);
    }

    #[test]
    fn to_config_invalid_port_defaults_to_8080() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.port = "not_a_number".to_string();

        let config = form.to_config().unwrap();
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn to_config_bypass_filters_empty_entries() {
        let mut form = UpstreamProxyForm::new();
        form.enabled = true;
        form.host = "proxy.example.com".to_string();
        form.bypass = "localhost,,, *.test.com, ,".to_string();

        let config = form.to_config().unwrap();
        assert_eq!(config.bypass, vec!["localhost", "*.test.com"]);
    }
}
