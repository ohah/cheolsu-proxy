pub(super) const CERT_DOWNLOAD_HOST: &str = "cheolsu.proxy";
pub(super) const CERT_DOWNLOAD_HOST_COLON: &str = "cheolsu.proxy:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Platform {
    Ios,
    Android,
    Unknown,
}

pub(super) fn detect_platform(user_agent: &str) -> Platform {
    let ua = user_agent.to_lowercase();
    if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ipod") {
        Platform::Ios
    } else if ua.contains("android") {
        Platform::Android
    } else {
        Platform::Unknown
    }
}
