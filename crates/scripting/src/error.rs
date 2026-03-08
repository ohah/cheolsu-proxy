use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("런타임 초기화 실패: {0}")]
    RuntimeInit(String),

    #[error("스크립트 파일 읽기 실패: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("허용되지 않는 파일 확장자: {path}. js, ts, tsx, mjs만 허용됩니다.")]
    InvalidExtension { path: String },

    #[error("스크립트 경로를 확인할 수 없습니다: {path} ({reason})")]
    PathResolution { path: String, reason: String },

    #[error("TypeScript 트랜스파일 실패: {0}")]
    Transpile(String),

    #[error("스크립트 실행 실패: {0}")]
    Execution(String),

    #[error("직렬화 실패: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("훅 반환값 파싱 실패: {message} (raw: {raw})")]
    HookParse { message: String, raw: String },

    #[error("JS 결과를 문자열로 변환 실패")]
    ValueConversion,

    #[error("스크립트 엔진 스레드 종료됨")]
    EngineShutdown,

    #[error("스크립트 실행 타임아웃: {0}")]
    Timeout(String),

    #[error("스크립트 엔진 응답 없음")]
    NoResponse,
}

impl From<ScriptError> for String {
    fn from(e: ScriptError) -> Self {
        e.to_string()
    }
}
