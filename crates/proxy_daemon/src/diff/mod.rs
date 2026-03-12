//! 트래픽 비교(diff) 모듈: 헤더, 텍스트, JSON 등 다양한 형식의 diff를 생성하고 포맷합니다.

mod body;
mod format;
mod headers;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrafficDiff {
    pub request_diff: Option<TransactionPartDiff>,
    pub response_diff: Option<TransactionPartDiff>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionPartDiff {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_diff: Option<(String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_diff: Option<(String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_diff: Option<(u16, u16)>,
    pub header_diffs: Vec<HeaderDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_diff: Option<BodyDiff>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum HeaderDiff {
    #[serde(rename = "added")]
    Added { key: String, value: String },
    #[serde(rename = "removed")]
    Removed { key: String, value: String },
    #[serde(rename = "modified")]
    Modified {
        key: String,
        old_value: String,
        new_value: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum BodyDiff {
    #[serde(rename = "text")]
    Text {
        additions: Vec<DiffLine>,
        deletions: Vec<DiffLine>,
        unchanged: usize,
    },
    #[serde(rename = "json")]
    Json { changes: Vec<JsonDiffEntry> },
    #[serde(rename = "binary")]
    Binary { old_size: usize, new_size: usize },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiffLine {
    pub line_number: usize,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonDiffEntry {
    pub path: String,
    pub change_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_value: Option<String>,
}

pub use body::{diff_json, diff_text, is_text_data_type};
pub use format::format_diff_text;
pub use headers::diff_headers;
