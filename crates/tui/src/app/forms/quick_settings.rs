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
