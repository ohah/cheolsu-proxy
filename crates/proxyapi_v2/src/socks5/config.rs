use crate::throttle::ThrottleConfig;
use crate::upstream_proxy::UpstreamProxyConfig;
use std::sync::Arc;
use tokio::sync::watch;

/// SOCKS5 인증 설정
#[derive(Debug, Clone)]
pub enum Socks5Auth {
    /// 인증 없음
    NoAuth,
    /// 사용자명/비밀번호 인증 (RFC 1929)
    UsernamePassword { username: String, password: String },
}

/// SOCKS5 프록시 서버 설정
#[derive(Debug, Clone)]
pub struct Socks5Config {
    pub auth: Socks5Auth,
    pub upstream_proxy: Option<UpstreamProxyConfig>,
    pub throttle_rx: Option<Arc<watch::Receiver<Option<ThrottleConfig>>>>,
}

impl Default for Socks5Config {
    fn default() -> Self {
        Self {
            auth: Socks5Auth::NoAuth,
            upstream_proxy: None,
            throttle_rx: None,
        }
    }
}
