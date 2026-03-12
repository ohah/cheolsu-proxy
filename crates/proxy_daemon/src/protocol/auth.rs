use serde::{Deserialize, Serialize};

/// 프록시 서버 자체의 인증 설정
/// 활성화 시, 클라이언트가 Proxy-Authorization 헤더로 Basic 인증을 해야만 프록시를 사용할 수 있음
#[derive(Serialize, Deserialize, Clone)]
pub struct ProxyAuthConfig {
    pub enabled: bool,
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for ProxyAuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyAuthConfig")
            .field("enabled", &self.enabled)
            .field("username", &self.username)
            .field("password", &"****")
            .finish()
    }
}

impl ProxyAuthConfig {
    /// Basic 인증 헤더 값을 생성합니다.
    pub fn expected_basic_header(&self) -> String {
        use base64::Engine;
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    /// Proxy-Authorization 헤더 값을 검증합니다.
    pub fn validate_proxy_auth(&self, auth_header: Option<&str>) -> bool {
        if !self.enabled {
            return true;
        }
        if self.username.is_empty() {
            return true;
        }
        match auth_header {
            Some(header) => header == self.expected_basic_header(),
            None => false,
        }
    }
}
