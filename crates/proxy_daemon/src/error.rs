use thiserror::Error;

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("IO 오류: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 오류: {0}")]
    Json(#[from] serde_json::Error),

    #[error("데이터 디렉토리를 찾을 수 없습니다")]
    DataDirNotFound,

    #[error("프록시 오류: {0}")]
    Proxy(String),

    #[error("데몬 오류: {0}")]
    Daemon(String),

    #[error("UDS 통신 오류: {0}")]
    Uds(String),
}

impl From<String> for DaemonError {
    fn from(s: String) -> Self {
        DaemonError::Daemon(s)
    }
}

impl From<DaemonError> for String {
    fn from(e: DaemonError) -> Self {
        e.to_string()
    }
}
