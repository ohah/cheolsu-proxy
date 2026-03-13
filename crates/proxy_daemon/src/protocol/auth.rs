use serde::{Deserialize, Serialize};

/// 프록시 인증 방식
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    #[default]
    Basic,
    Bearer,
    ApiKey,
}

/// 프록시 서버 자체의 인증 설정
/// 활성화 시, 클라이언트가 설정된 인증 방식에 따라 인증을 해야만 프록시를 사용할 수 있음
#[derive(Serialize, Deserialize, Clone)]
pub struct ProxyAuthConfig {
    pub enabled: bool,
    #[serde(default)]
    pub method: AuthMethod,
    pub username: String,
    pub password: String,
    /// Bearer/ApiKey용 토큰
    #[serde(default)]
    pub token: Option<String>,
    /// ApiKey용 커스텀 헤더명 (기본값: "X-Api-Key")
    #[serde(default)]
    pub header_name: Option<String>,
}

impl std::fmt::Debug for ProxyAuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyAuthConfig")
            .field("enabled", &self.enabled)
            .field("method", &self.method)
            .field("username", &self.username)
            .field("password", &"****")
            .field("token", &self.token.as_ref().map(|_| "****"))
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

    /// ApiKey용 헤더명을 반환합니다. 기본값은 "X-Api-Key"입니다.
    pub fn api_key_header_name(&self) -> &str {
        self.header_name.as_deref().unwrap_or("x-api-key")
    }

    /// 인증 방식에 따라 요청 헤더를 검증합니다.
    pub fn validate_proxy_auth(&self, auth_header: Option<&str>) -> bool {
        if !self.enabled {
            return true;
        }

        match self.method {
            AuthMethod::Basic => {
                if self.username.is_empty() {
                    return true;
                }
                match auth_header {
                    Some(header) => header == self.expected_basic_header(),
                    None => false,
                }
            }
            AuthMethod::Bearer => {
                let Some(token) = &self.token else {
                    return true;
                };
                if token.is_empty() {
                    return true;
                }
                match auth_header {
                    Some(header) => header == format!("Bearer {}", token),
                    None => false,
                }
            }
            AuthMethod::ApiKey => {
                // ApiKey 검증은 validate_api_key로 별도 처리
                // auth_header에는 커스텀 헤더 값이 전달됨
                let Some(token) = &self.token else {
                    return true;
                };
                if token.is_empty() {
                    return true;
                }
                match auth_header {
                    Some(header) => header == token.as_str(),
                    None => false,
                }
            }
        }
    }
}
