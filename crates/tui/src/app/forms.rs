use proxy_daemon::ThrottlePreset;
use proxy_daemon::{
    HostMapping, InterceptAction, InterceptRule, SslProxyingEntry, ThrottleConfig,
    UpstreamProxyAuth, UpstreamProxyConfig,
};

/// 스크립트 로그 엔트리 (TUI 표시용)
#[derive(Debug, Clone)]
#[allow(dead_code)]
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
            username: self.username.clone(),
            password: self.password.clone(),
        }
    }
}

/// Settings 탭 섹션 선택
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    UpstreamProxy,
    ProxyAuth,
    Throttle,
    HostMapping,
    QuickSettings,
    SslProxying,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 6] = [
        Self::UpstreamProxy,
        Self::ProxyAuth,
        Self::Throttle,
        Self::HostMapping,
        Self::QuickSettings,
        Self::SslProxying,
    ];
}

cycle_enum!(SettingsSection);

/// Quick Settings 폼
#[derive(Debug, Clone)]
pub struct QuickSettingsForm {
    pub no_caching: bool,
    pub block_cookies: bool,
    pub no_gzip: bool,
    pub field: QuickSettingsField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickSettingsField {
    NoCaching,
    BlockCookies,
    NoGzip,
}

impl QuickSettingsField {
    pub const ALL: [QuickSettingsField; 3] = [Self::NoCaching, Self::BlockCookies, Self::NoGzip];

    pub fn label(&self) -> &str {
        match self {
            Self::NoCaching => "No Caching",
            Self::BlockCookies => "Block Cookies",
            Self::NoGzip => "No Gzip",
        }
    }
}

cycle_enum!(QuickSettingsField);

impl QuickSettingsForm {
    pub fn new() -> Self {
        Self {
            no_caching: false,
            block_cookies: false,
            no_gzip: false,
            field: QuickSettingsField::NoCaching,
        }
    }
}

/// Host Mapping 폼
#[derive(Debug, Clone)]
pub struct HostMappingForm {
    pub field: HostMappingField,
    pub source_host: String,
    pub source_port: String,
    pub target_host: String,
    pub target_port: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostMappingField {
    SourceHost,
    SourcePort,
    TargetHost,
    TargetPort,
}

impl HostMappingField {
    pub const ALL: [HostMappingField; 4] = [
        Self::SourceHost,
        Self::SourcePort,
        Self::TargetHost,
        Self::TargetPort,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::SourceHost => "Source Host",
            Self::SourcePort => "Source Port",
            Self::TargetHost => "Target Host",
            Self::TargetPort => "Target Port",
        }
    }
}

cycle_enum!(HostMappingField);

impl HostMappingForm {
    pub fn new() -> Self {
        Self {
            field: HostMappingField::SourceHost,
            source_host: String::new(),
            source_port: String::new(),
            target_host: String::new(),
            target_port: String::new(),
        }
    }

    pub fn to_mapping(&self) -> Option<HostMapping> {
        if self.source_host.is_empty() || self.target_host.is_empty() {
            return None;
        }
        Some(HostMapping {
            id: format!("hm_{}", uuid::Uuid::new_v4()),
            source_host: self.source_host.clone(),
            source_port: self.source_port.parse().ok(),
            target_host: self.target_host.clone(),
            target_port: self.target_port.parse().ok(),
            enabled: true,
        })
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.source_host.clear();
        self.source_port.clear();
        self.target_host.clear();
        self.target_port.clear();
        self.field = HostMappingField::SourceHost;
    }
}

/// 스로틀링 프리셋
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottlePresetChoice {
    None,
    Gprs,
    Slow3G,
    Fast3G,
    Lte,
    Wifi,
    Custom,
}

impl ThrottlePresetChoice {
    pub const ALL: [ThrottlePresetChoice; 7] = [
        Self::None,
        Self::Gprs,
        Self::Slow3G,
        Self::Fast3G,
        Self::Lte,
        Self::Wifi,
        Self::Custom,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Gprs => "GPRS (50 KB/s)",
            Self::Slow3G => "Slow 3G (500 KB/s)",
            Self::Fast3G => "Fast 3G (1.6 MB/s)",
            Self::Lte => "4G/LTE (4 MB/s)",
            Self::Wifi => "WiFi (30 MB/s)",
            Self::Custom => "Custom",
        }
    }
}

cycle_enum!(ThrottlePresetChoice);

/// 스로틀링 폼
#[derive(Debug, Clone)]
pub struct ThrottleForm {
    pub enabled: bool,
    pub preset: ThrottlePresetChoice,
    pub field: ThrottleField,
    pub editing: bool,
    pub download: String, // KB/s
    pub upload: String,   // KB/s
    pub latency: String,  // ms
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleField {
    Enabled,
    Preset,
    Download,
    Upload,
    Latency,
}

impl ThrottleField {
    pub const ALL: [ThrottleField; 5] = [
        Self::Enabled,
        Self::Preset,
        Self::Download,
        Self::Upload,
        Self::Latency,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Enabled => "Enabled",
            Self::Preset => "Profile",
            Self::Download => "Download (KB/s)",
            Self::Upload => "Upload (KB/s)",
            Self::Latency => "Latency (ms)",
        }
    }
}

cycle_enum!(ThrottleField);

impl ThrottleForm {
    pub fn new() -> Self {
        Self {
            enabled: false,
            preset: ThrottlePresetChoice::None,
            field: ThrottleField::Enabled,
            editing: false,
            download: String::new(),
            upload: String::new(),
            latency: "0".to_string(),
        }
    }

    pub fn to_config(&self) -> Option<ThrottleConfig> {
        if !self.enabled {
            return None;
        }

        match self.preset {
            ThrottlePresetChoice::None => None,
            ThrottlePresetChoice::Gprs => Some(ThrottlePreset::Gprs.to_config()),
            ThrottlePresetChoice::Slow3G => Some(ThrottlePreset::Slow3G.to_config()),
            ThrottlePresetChoice::Fast3G => Some(ThrottlePreset::Fast3G.to_config()),
            ThrottlePresetChoice::Lte => Some(ThrottlePreset::Lte.to_config()),
            ThrottlePresetChoice::Wifi => Some(ThrottlePreset::Wifi.to_config()),
            ThrottlePresetChoice::Custom => {
                let dl: u64 = self.download.parse().unwrap_or(0);
                let ul: u64 = self.upload.parse().unwrap_or(0);
                Some(ThrottleConfig {
                    enabled: true,
                    download_rate: if dl > 0 { Some(dl * 1024) } else { None },
                    upload_rate: if ul > 0 { Some(ul * 1024) } else { None },
                    latency_ms: self.latency.parse().unwrap_or(0),
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
}

cycle_enum!(ActionType);

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

/// SSL Proxying 화이트리스트 추가 폼
#[derive(Debug, Clone)]
pub struct SslProxyingAddForm {
    pub pattern: String,
}

impl SslProxyingAddForm {
    pub fn new() -> Self {
        Self {
            pattern: String::new(),
        }
    }

    pub fn to_entry(&self) -> Option<SslProxyingEntry> {
        if self.pattern.is_empty() {
            return None;
        }
        Some(SslProxyingEntry {
            pattern: self.pattern.clone(),
            enabled: true,
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

    // -- SettingsSection --

    #[test]
    fn settings_section_next_prev_cycle() {
        let mut section = SettingsSection::UpstreamProxy;
        section = section.next();
        assert_eq!(section, SettingsSection::ProxyAuth);
        section = section.next();
        assert_eq!(section, SettingsSection::Throttle);
        section = section.next();
        assert_eq!(section, SettingsSection::HostMapping);
        section = section.next();
        assert_eq!(section, SettingsSection::QuickSettings);
        section = section.next();
        assert_eq!(section, SettingsSection::SslProxying);
        section = section.next();
        assert_eq!(section, SettingsSection::UpstreamProxy);
        section = section.prev();
        assert_eq!(section, SettingsSection::SslProxying);
    }

    // -- ProxyAuthField --

    #[test]
    fn proxy_auth_field_next_prev_cycle() {
        let mut field = ProxyAuthField::Enabled;
        for _ in 0..ProxyAuthField::ALL.len() {
            field = field.next();
        }
        assert_eq!(field, ProxyAuthField::Enabled);

        for _ in 0..ProxyAuthField::ALL.len() {
            field = field.prev();
        }
        assert_eq!(field, ProxyAuthField::Enabled);
    }

    #[test]
    fn proxy_auth_field_labels_not_empty() {
        for field in ProxyAuthField::ALL {
            assert!(!field.label().is_empty());
        }
    }

    // -- ProxyAuthForm --

    #[test]
    fn proxy_auth_form_new_defaults() {
        let form = ProxyAuthForm::new();
        assert!(!form.enabled);
        assert_eq!(form.field, ProxyAuthField::Enabled);
        assert!(!form.editing);
        assert!(form.username.is_empty());
        assert!(form.password.is_empty());
    }

    #[test]
    fn proxy_auth_form_to_config() {
        let mut form = ProxyAuthForm::new();
        form.enabled = true;
        form.username = "admin".to_string();
        form.password = "secret".to_string();
        let config = form.to_config();
        assert!(config.enabled);
        assert_eq!(config.username, "admin");
        assert_eq!(config.password, "secret");
    }

    #[test]
    fn proxy_auth_form_to_config_disabled() {
        let form = ProxyAuthForm::new();
        let config = form.to_config();
        assert!(!config.enabled);
        assert!(config.username.is_empty());
        assert!(config.password.is_empty());
    }

    // -- ThrottlePresetChoice --

    #[test]
    fn throttle_preset_choice_labels_not_empty() {
        for preset in ThrottlePresetChoice::ALL {
            assert!(!preset.label().is_empty());
        }
    }

    #[test]
    fn throttle_preset_choice_full_cycle() {
        let mut preset = ThrottlePresetChoice::None;
        for _ in 0..ThrottlePresetChoice::ALL.len() {
            preset = preset.next();
        }
        assert_eq!(preset, ThrottlePresetChoice::None);
    }

    // -- ThrottleField --

    #[test]
    fn throttle_field_labels_not_empty() {
        for field in ThrottleField::ALL {
            assert!(!field.label().is_empty());
        }
    }

    #[test]
    fn throttle_field_next_prev_cycle() {
        let mut field = ThrottleField::Enabled;
        for _ in 0..ThrottleField::ALL.len() {
            field = field.next();
        }
        assert_eq!(field, ThrottleField::Enabled);

        for _ in 0..ThrottleField::ALL.len() {
            field = field.prev();
        }
        assert_eq!(field, ThrottleField::Enabled);
    }

    // -- ThrottleForm --

    #[test]
    fn throttle_form_new_defaults() {
        let form = ThrottleForm::new();
        assert!(!form.enabled);
        assert_eq!(form.preset, ThrottlePresetChoice::None);
        assert_eq!(form.field, ThrottleField::Enabled);
        assert!(!form.editing);
        assert!(form.download.is_empty());
        assert!(form.upload.is_empty());
        assert_eq!(form.latency, "0");
    }

    #[test]
    fn throttle_form_to_config_disabled_returns_none() {
        let form = ThrottleForm::new();
        assert!(form.to_config().is_none());
    }

    #[test]
    fn throttle_form_to_config_enabled_none_preset_returns_none() {
        let mut form = ThrottleForm::new();
        form.enabled = true;
        form.preset = ThrottlePresetChoice::None;
        assert!(form.to_config().is_none());
    }

    #[test]
    fn throttle_form_to_config_gprs_matches_core_preset() {
        let mut form = ThrottleForm::new();
        form.enabled = true;
        form.preset = ThrottlePresetChoice::Gprs;
        let config = form.to_config().unwrap();
        let core_config = proxy_daemon::ThrottlePreset::Gprs.to_config();
        assert_eq!(config.download_rate, core_config.download_rate);
        assert_eq!(config.upload_rate, core_config.upload_rate);
        assert_eq!(config.latency_ms, core_config.latency_ms);
    }

    #[test]
    fn throttle_form_to_config_all_presets_match_core() {
        let pairs: Vec<(ThrottlePresetChoice, proxy_daemon::ThrottlePreset)> = vec![
            (
                ThrottlePresetChoice::Gprs,
                proxy_daemon::ThrottlePreset::Gprs,
            ),
            (
                ThrottlePresetChoice::Slow3G,
                proxy_daemon::ThrottlePreset::Slow3G,
            ),
            (
                ThrottlePresetChoice::Fast3G,
                proxy_daemon::ThrottlePreset::Fast3G,
            ),
            (ThrottlePresetChoice::Lte, proxy_daemon::ThrottlePreset::Lte),
            (
                ThrottlePresetChoice::Wifi,
                proxy_daemon::ThrottlePreset::Wifi,
            ),
        ];
        for (tui_preset, core_preset) in pairs {
            let mut form = ThrottleForm::new();
            form.enabled = true;
            form.preset = tui_preset;
            let config = form.to_config().unwrap();
            let core_config = core_preset.to_config();
            assert_eq!(
                config.download_rate, core_config.download_rate,
                "download mismatch for {:?}",
                tui_preset
            );
            assert_eq!(
                config.upload_rate, core_config.upload_rate,
                "upload mismatch for {:?}",
                tui_preset
            );
            assert_eq!(
                config.latency_ms, core_config.latency_ms,
                "latency mismatch for {:?}",
                tui_preset
            );
        }
    }

    #[test]
    fn throttle_form_to_config_custom() {
        let mut form = ThrottleForm::new();
        form.enabled = true;
        form.preset = ThrottlePresetChoice::Custom;
        form.download = "1024".to_string();
        form.upload = "512".to_string();
        form.latency = "100".to_string();

        let config = form.to_config().unwrap();
        assert!(config.enabled);
        assert_eq!(config.download_rate, Some(1024 * 1024));
        assert_eq!(config.upload_rate, Some(512 * 1024));
        assert_eq!(config.latency_ms, 100);
    }

    #[test]
    fn throttle_form_to_config_custom_zero_rate_is_none() {
        let mut form = ThrottleForm::new();
        form.enabled = true;
        form.preset = ThrottlePresetChoice::Custom;
        form.download = "0".to_string();
        form.upload = "0".to_string();
        form.latency = "50".to_string();

        let config = form.to_config().unwrap();
        assert!(config.download_rate.is_none());
        assert!(config.upload_rate.is_none());
        assert_eq!(config.latency_ms, 50);
    }

    #[test]
    fn throttle_form_to_config_custom_empty_strings() {
        let mut form = ThrottleForm::new();
        form.enabled = true;
        form.preset = ThrottlePresetChoice::Custom;
        form.download = "".to_string();
        form.upload = "".to_string();
        form.latency = "".to_string();

        let config = form.to_config().unwrap();
        assert!(config.download_rate.is_none());
        assert!(config.upload_rate.is_none());
        assert_eq!(config.latency_ms, 0);
    }

    #[test]
    fn throttle_form_to_config_custom_invalid_input() {
        let mut form = ThrottleForm::new();
        form.enabled = true;
        form.preset = ThrottlePresetChoice::Custom;
        form.download = "abc".to_string();
        form.upload = "xyz".to_string();
        form.latency = "not_a_number".to_string();

        let config = form.to_config().unwrap();
        assert!(config.download_rate.is_none());
        assert!(config.upload_rate.is_none());
        assert_eq!(config.latency_ms, 0);
    }
}
