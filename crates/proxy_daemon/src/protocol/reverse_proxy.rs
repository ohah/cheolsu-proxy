use serde::{Deserialize, Serialize};

/// 리버스 프록시 규칙.
/// 기존 프록시 포트에서 relative URI 요청을 받아 Host 헤더 기반으로
/// 백엔드 서버로 전달하는 규칙을 정의합니다.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReverseProxyRule {
    pub id: String,
    /// Host 헤더 매칭 패턴 (예: "api.myapp.local", "*.local")
    pub match_host: String,
    /// 백엔드 스킴 ("http" 또는 "https")
    #[serde(default = "default_scheme")]
    pub backend_scheme: String,
    /// 백엔드 호스트
    pub backend_host: String,
    /// 백엔드 포트
    pub backend_port: u16,
    /// Host 헤더를 백엔드 주소로 재작성 여부 (기본: true)
    #[serde(default = "super::default_true")]
    pub rewrite_host: bool,
    pub enabled: bool,
}

fn default_scheme() -> String {
    "http".to_string()
}

impl std::fmt::Display for ReverseProxyRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.enabled { "enabled" } else { "disabled" };
        write!(
            f,
            "[{}] {} -> {}://{}:{} (rewrite_host={}) [{}]",
            self.id,
            self.match_host,
            self.backend_scheme,
            self.backend_host,
            self.backend_port,
            self.rewrite_host,
            status
        )
    }
}
