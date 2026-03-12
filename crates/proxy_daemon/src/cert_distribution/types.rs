pub(crate) const CERT_DOWNLOAD_HOST: &str = "cheolsu.proxy";
pub(crate) const CERT_DOWNLOAD_HOST_COLON: &str = "cheolsu.proxy:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Platform {
    Ios,
    Android,
    Unknown,
}

pub(crate) fn detect_platform(user_agent: &str) -> Platform {
    let ua = user_agent.to_lowercase();
    if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ipod") {
        Platform::Ios
    } else if ua.contains("android") {
        Platform::Android
    } else {
        Platform::Unknown
    }
}
