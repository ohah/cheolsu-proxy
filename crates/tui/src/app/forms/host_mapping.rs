use proxy_daemon::HostMapping;

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
