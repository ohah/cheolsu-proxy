use proxy_daemon::ThrottleConfig;
use proxy_daemon::ThrottlePreset;

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
