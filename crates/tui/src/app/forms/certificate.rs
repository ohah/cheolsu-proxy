use proxy_daemon::{ClientCertConfig, SslProxyingEntry};

/// Client Certificate (mTLS) 폼
#[derive(Debug, Clone)]
pub struct ClientCertForm {
    pub enabled: bool,
    pub cert_path: String,
    pub key_path: String,
    pub field: ClientCertField,
    pub editing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientCertField {
    Enabled,
    CertPath,
    KeyPath,
}

impl ClientCertField {
    pub const ALL: [ClientCertField; 3] = [Self::Enabled, Self::CertPath, Self::KeyPath];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Enabled => "Enabled",
            Self::CertPath => "Cert File",
            Self::KeyPath => "Key File",
        }
    }
}

cycle_enum!(ClientCertField);

impl ClientCertForm {
    pub fn new() -> Self {
        Self {
            enabled: false,
            cert_path: String::new(),
            key_path: String::new(),
            field: ClientCertField::Enabled,
            editing: false,
        }
    }

    pub fn to_config(&self) -> Option<ClientCertConfig> {
        if !self.enabled || self.cert_path.is_empty() || self.key_path.is_empty() {
            return None;
        }
        Some(ClientCertConfig {
            cert_path: self.cert_path.clone(),
            key_path: self.key_path.clone(),
            enabled: true,
            domain_certs: vec![],
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
