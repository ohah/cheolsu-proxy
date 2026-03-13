use proxy_daemon::{UpstreamProxyAuth, UpstreamProxyConfig};

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

cycle_enum!(UpstreamProxyField);

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

/// Proxy Authentication 폼
#[derive(Debug, Clone)]
pub struct ProxyAuthForm {
    pub enabled: bool,
    pub field: ProxyAuthField,
    pub editing: bool,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyAuthField {
    Enabled,
    Username,
    Password,
}

impl ProxyAuthField {
    pub const ALL: [ProxyAuthField; 3] = [Self::Enabled, Self::Username, Self::Password];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Enabled => "Enabled",
            Self::Username => "Username",
            Self::Password => "Password",
        }
    }
}

cycle_enum!(ProxyAuthField);

impl ProxyAuthForm {
    pub fn new() -> Self {
        Self {
            enabled: false,
            field: ProxyAuthField::Enabled,
            editing: false,
            username: String::new(),
            password: String::new(),
        }
    }

    pub fn to_config(&self) -> proxy_daemon::ProxyAuthConfig {
        proxy_daemon::ProxyAuthConfig {
            enabled: self.enabled,
            method: proxy_daemon::AuthMethod::default(),
            username: self.username.clone(),
            password: self.password.clone(),
            token: None,
            header_name: None,
        }
    }
}
