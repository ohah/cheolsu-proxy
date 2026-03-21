/// rmcp 비의존 범용 결과 타입.
/// MCP에서는 CallToolResult로, CLI에서는 stdout/stderr로 변환된다.
pub enum OpResult {
    Ok(String),
    Err(String),
}

impl OpResult {
    pub fn ok(msg: impl Into<String>) -> Self {
        OpResult::Ok(msg.into())
    }

    pub fn err(msg: impl Into<String>) -> Self {
        OpResult::Err(msg.into())
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, OpResult::Ok(_))
    }

    pub fn message(&self) -> &str {
        match self {
            OpResult::Ok(m) | OpResult::Err(m) => m,
        }
    }
}
