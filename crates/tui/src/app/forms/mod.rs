mod certificate;
mod host_mapping;
mod quick_settings;
mod rule;
mod throttle;
mod upstream_proxy;

pub use certificate::*;
pub use host_mapping::*;
pub use quick_settings::*;
pub use rule::*;
pub use throttle::*;
pub use upstream_proxy::*;

/// 스크립트 로그 엔트리 (TUI 표시용)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ScriptLogEntry {
    pub level: String,
    pub message: String,
    pub time: std::time::Instant,
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
    ClientCertificate,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 7] = [
        Self::UpstreamProxy,
        Self::ProxyAuth,
        Self::Throttle,
        Self::HostMapping,
        Self::QuickSettings,
        Self::SslProxying,
        Self::ClientCertificate,
    ];
}

cycle_enum!(SettingsSection);

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WsConnection {
    pub connection_id: String,
    pub uri: String,
    pub time: i64,
    pub active: bool,
}

#[cfg(test)]
mod tests;
