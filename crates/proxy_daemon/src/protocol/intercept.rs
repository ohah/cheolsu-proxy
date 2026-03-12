use super::default_true;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 인터셉트 규칙 정의
/// `pattern`은 와일드카드 URL 패턴 (예: `*.example.com/api/*`)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterceptRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub pattern: String,
    #[serde(default)]
    pub method: Option<String>,
    pub action: InterceptAction,
}

/// Rewrite 대상 열거형
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RewriteTarget {
    #[serde(rename = "request_header")]
    RequestHeader,
    #[serde(rename = "response_header")]
    ResponseHeader,
    #[serde(rename = "request_body")]
    RequestBody,
    #[serde(rename = "response_body")]
    ResponseBody,
}

/// 인터셉트 동작
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum InterceptAction {
    /// 요청을 차단하고 지정된 응답을 반환
    #[serde(rename = "block")]
    Block {
        #[serde(default = "default_block_status")]
        status_code: u16,
        #[serde(default)]
        body: String,
    },
    /// 요청 헤더/바디를 수정하여 전달
    #[serde(rename = "modify_request")]
    ModifyRequest {
        #[serde(default)]
        add_headers: HashMap<String, String>,
        #[serde(default)]
        remove_headers: Vec<String>,
        #[serde(default)]
        set_body: Option<String>,
    },
    /// 응답 헤더/바디/상태코드를 수정
    #[serde(rename = "modify_response")]
    ModifyResponse {
        #[serde(default)]
        set_status: Option<u16>,
        #[serde(default)]
        add_headers: HashMap<String, String>,
        #[serde(default)]
        remove_headers: Vec<String>,
        #[serde(default)]
        set_body: Option<String>,
    },
    /// 요청을 로컬 파일의 내용으로 응답 (Map Local)
    #[serde(rename = "map_local")]
    MapLocal {
        /// 로컬 파일 경로
        file_path: String,
        /// 응답 상태 코드 (기본: 200)
        #[serde(default = "default_ok_status")]
        status_code: u16,
        /// 추가 응답 헤더
        #[serde(default)]
        headers: HashMap<String, String>,
    },
    /// 요청을 다른 URL로 리다이렉트 (Map Remote)
    #[serde(rename = "map_remote")]
    MapRemote {
        /// 대상 URL (예: http://localhost:3000)
        target_url: String,
        /// 원본 경로 유지 여부 (true면 원본 path를 target_url에 붙임)
        #[serde(default = "default_true")]
        preserve_path: bool,
    },
    /// 정규식을 사용하여 요청/응답의 헤더 또는 바디를 치환 (Rewrite)
    #[serde(rename = "rewrite")]
    Rewrite {
        /// 적용 대상
        target: RewriteTarget,
        /// 매칭할 정규식 패턴
        match_pattern: String,
        /// 치환 문자열 ($1, $2 등 캡처 그룹 지원)
        replace_with: String,
    },
}

/// 서버 리플레이 엔트리: 캡처된 응답을 저장하여 동일 요청 시 재사용
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerReplayEntry {
    pub id: String,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

pub(crate) fn default_block_status() -> u16 {
    403
}

pub(crate) fn default_ok_status() -> u16 {
    200
}

impl std::fmt::Display for InterceptRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.enabled { "활성" } else { "비활성" };
        let action_desc = match &self.action {
            InterceptAction::Block { status_code, .. } => {
                format!("Block({})", status_code)
            }
            InterceptAction::ModifyRequest { .. } => "ModifyRequest".to_string(),
            InterceptAction::ModifyResponse { set_status, .. } => {
                if let Some(status) = set_status {
                    format!("ModifyResponse(status={})", status)
                } else {
                    "ModifyResponse".to_string()
                }
            }
            InterceptAction::MapLocal {
                file_path,
                status_code,
                ..
            } => {
                format!("MapLocal({}, status={})", file_path, status_code)
            }
            InterceptAction::MapRemote {
                target_url,
                preserve_path,
            } => {
                format!("MapRemote({}, preserve_path={})", target_url, preserve_path)
            }
            InterceptAction::Rewrite {
                target,
                match_pattern,
                ..
            } => {
                format!("Rewrite({:?}, pattern={})", target, match_pattern)
            }
        };
        let method_str = self.method.as_deref().unwrap_or("*");
        write!(
            f,
            "[{}] {} ({} {}) -> {} [{}]",
            self.id, self.name, method_str, self.pattern, action_desc, status
        )
    }
}
