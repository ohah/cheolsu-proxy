use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::collections::{BTreeMap, BTreeSet};

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

/// Compare two header maps and return a list of diffs.
/// Supports duplicate header keys by grouping values into Vec<String>.
pub fn diff_headers(
    old_headers: &[(String, String)],
    new_headers: &[(String, String)],
) -> Vec<HeaderDiff> {
    let mut old_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (k, v) in old_headers {
        old_map.entry(k.to_lowercase()).or_default().push(v.clone());
    }
    let mut new_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (k, v) in new_headers {
        new_map.entry(k.to_lowercase()).or_default().push(v.clone());
    }

    let all_keys: BTreeSet<&String> = old_map.keys().chain(new_map.keys()).collect();

    let mut diffs = Vec::new();
    for key in all_keys {
        match (old_map.get(key), new_map.get(key)) {
            (Some(old_vals), Some(new_vals)) => {
                if old_vals != new_vals {
                    diffs.push(HeaderDiff::Modified {
                        key: key.clone(),
                        old_value: old_vals.join(", "),
                        new_value: new_vals.join(", "),
                    });
                }
            }
            (Some(old_vals), None) => {
                diffs.push(HeaderDiff::Removed {
                    key: key.clone(),
                    value: old_vals.join(", "),
                });
            }
            (None, Some(new_vals)) => {
                diffs.push(HeaderDiff::Added {
                    key: key.clone(),
                    value: new_vals.join(", "),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    diffs
}

/// Line-based text diff using the `similar` crate.
pub fn diff_text(old_text: &str, new_text: &str) -> BodyDiff {
    let text_diff = TextDiff::from_lines(old_text, new_text);

    let mut additions = Vec::new();
    let mut deletions = Vec::new();
    let mut unchanged = 0usize;

    for change in text_diff.iter_all_changes() {
        let line_number = change
            .old_index()
            .or_else(|| change.new_index())
            .map(|i| i + 1)
            .unwrap_or(0);
        let content = change.value().trim_end_matches('\n').to_string();

        match change.tag() {
            ChangeTag::Delete => {
                deletions.push(DiffLine {
                    line_number,
                    content,
                });
            }
            ChangeTag::Insert => {
                additions.push(DiffLine {
                    line_number,
                    content,
                });
            }
            ChangeTag::Equal => {
                unchanged += 1;
            }
        }
    }

    BodyDiff::Text {
        additions,
        deletions,
        unchanged,
    }
}

/// Structural JSON diff that recursively compares two serde_json::Value trees.
pub fn diff_json(old: &serde_json::Value, new: &serde_json::Value) -> BodyDiff {
    let mut changes = Vec::new();
    compare_json_values(old, new, "$", &mut changes);
    BodyDiff::Json { changes }
}

fn compare_json_values(
    old: &serde_json::Value,
    new: &serde_json::Value,
    path: &str,
    changes: &mut Vec<JsonDiffEntry>,
) {
    use serde_json::Value;

    if old == new {
        return;
    }

    match (old, new) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            let all_keys: BTreeSet<&String> = old_map.keys().chain(new_map.keys()).collect();

            for key in all_keys {
                let child_path = format!("{}.{}", path, key);
                match (old_map.get(key), new_map.get(key)) {
                    (Some(old_val), Some(new_val)) => {
                        compare_json_values(old_val, new_val, &child_path, changes);
                    }
                    (None, Some(new_val)) => {
                        changes.push(JsonDiffEntry {
                            path: child_path,
                            change_type: "added".to_string(),
                            old_value: None,
                            new_value: Some(format_json_value(new_val)),
                        });
                    }
                    (Some(old_val), None) => {
                        changes.push(JsonDiffEntry {
                            path: child_path,
                            change_type: "removed".to_string(),
                            old_value: Some(format_json_value(old_val)),
                            new_value: None,
                        });
                    }
                    (None, None) => unreachable!(),
                }
            }
        }
        (Value::Array(old_arr), Value::Array(new_arr)) => {
            let max_len = old_arr.len().max(new_arr.len());
            for i in 0..max_len {
                let child_path = format!("{}[{}]", path, i);
                match (old_arr.get(i), new_arr.get(i)) {
                    (Some(old_val), Some(new_val)) => {
                        compare_json_values(old_val, new_val, &child_path, changes);
                    }
                    (None, Some(new_val)) => {
                        changes.push(JsonDiffEntry {
                            path: child_path,
                            change_type: "added".to_string(),
                            old_value: None,
                            new_value: Some(format_json_value(new_val)),
                        });
                    }
                    (Some(old_val), None) => {
                        changes.push(JsonDiffEntry {
                            path: child_path,
                            change_type: "removed".to_string(),
                            old_value: Some(format_json_value(old_val)),
                            new_value: None,
                        });
                    }
                    (None, None) => unreachable!(),
                }
            }
        }
        _ => {
            changes.push(JsonDiffEntry {
                path: path.to_string(),
                change_type: "modified".to_string(),
                old_value: Some(format_json_value(old)),
                new_value: Some(format_json_value(new)),
            });
        }
    }
}

fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("\"{}\"", s),
        other => other.to_string(),
    }
}

/// Format a TrafficDiff as human-readable text.
pub fn format_diff_text(diff: &TrafficDiff) -> String {
    let mut out = String::new();

    if let Some(req_diff) = &diff.request_diff {
        out.push_str("## Request Diff\n\n");
        format_part_diff(&mut out, req_diff);
    }

    if let Some(res_diff) = &diff.response_diff {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("## Response Diff\n\n");
        format_part_diff(&mut out, res_diff);
    }

    if out.is_empty() {
        out.push_str("No differences found.");
    }

    out
}

fn format_part_diff(out: &mut String, part: &TransactionPartDiff) {
    if let Some((old, new)) = &part.method_diff {
        out.push_str(&format!("Method: {} → {}\n", old, new));
    }
    if let Some((old, new)) = &part.url_diff {
        out.push_str(&format!("URL: {} → {}\n", old, new));
    }
    if let Some((old, new)) = &part.status_diff {
        out.push_str(&format!("Status: {} → {}\n", old, new));
    }

    if !part.header_diffs.is_empty() {
        out.push_str("\n### Headers\n");
        for hd in &part.header_diffs {
            match hd {
                HeaderDiff::Added { key, value } => {
                    out.push_str(&format!("+ {}: {}\n", key, value));
                }
                HeaderDiff::Removed { key, value } => {
                    out.push_str(&format!("- {}: {}\n", key, value));
                }
                HeaderDiff::Modified {
                    key,
                    old_value,
                    new_value,
                } => {
                    out.push_str(&format!("~ {}: {} → {}\n", key, old_value, new_value));
                }
            }
        }
    }

    if let Some(body_diff) = &part.body_diff {
        out.push_str("\n### Body\n");
        match body_diff {
            BodyDiff::Text {
                additions,
                deletions,
                unchanged,
            } => {
                out.push_str(&format!(
                    "{} unchanged, {} added, {} deleted\n",
                    unchanged,
                    additions.len(),
                    deletions.len()
                ));
                for d in deletions {
                    out.push_str(&format!("- L{}: {}\n", d.line_number, d.content));
                }
                for a in additions {
                    out.push_str(&format!("+ L{}: {}\n", a.line_number, a.content));
                }
            }
            BodyDiff::Json { changes } => {
                out.push_str(&format!("{} JSON changes:\n", changes.len()));
                for c in changes {
                    match c.change_type.as_str() {
                        "added" => {
                            out.push_str(&format!(
                                "+ {} = {}\n",
                                c.path,
                                c.new_value.as_deref().unwrap_or("")
                            ));
                        }
                        "removed" => {
                            out.push_str(&format!(
                                "- {} = {}\n",
                                c.path,
                                c.old_value.as_deref().unwrap_or("")
                            ));
                        }
                        "modified" => {
                            out.push_str(&format!(
                                "~ {} : {} → {}\n",
                                c.path,
                                c.old_value.as_deref().unwrap_or(""),
                                c.new_value.as_deref().unwrap_or("")
                            ));
                        }
                        _ => {}
                    }
                }
            }
            BodyDiff::Binary { old_size, new_size } => {
                out.push_str(&format!(
                    "Binary content: {} bytes → {} bytes\n",
                    old_size, new_size
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_diff_headers_no_changes() {
        let old = vec![("content-type".to_string(), "text/html".to_string())];
        let new = vec![("content-type".to_string(), "text/html".to_string())];
        let diffs = diff_headers(&old, &new);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_diff_headers_added() {
        let old = vec![];
        let new = vec![("x-new".to_string(), "value".to_string())];
        let diffs = diff_headers(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(&diffs[0], HeaderDiff::Added { key, .. } if key == "x-new"));
    }

    #[test]
    fn test_diff_headers_removed() {
        let old = vec![("x-old".to_string(), "value".to_string())];
        let new = vec![];
        let diffs = diff_headers(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(&diffs[0], HeaderDiff::Removed { key, .. } if key == "x-old"));
    }

    #[test]
    fn test_diff_headers_modified() {
        let old = vec![("content-type".to_string(), "text/html".to_string())];
        let new = vec![("content-type".to_string(), "application/json".to_string())];
        let diffs = diff_headers(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert!(
            matches!(&diffs[0], HeaderDiff::Modified { key, old_value, new_value } if key == "content-type" && old_value == "text/html" && new_value == "application/json")
        );
    }

    #[test]
    fn test_diff_text_identical() {
        let body = diff_text("hello\nworld\n", "hello\nworld\n");
        match body {
            BodyDiff::Text {
                additions,
                deletions,
                unchanged,
            } => {
                assert!(additions.is_empty());
                assert!(deletions.is_empty());
                assert_eq!(unchanged, 2);
            }
            _ => panic!("Expected Text diff"),
        }
    }

    #[test]
    fn test_diff_text_with_changes() {
        let body = diff_text("line1\nline2\nline3\n", "line1\nmodified\nline3\n");
        match body {
            BodyDiff::Text {
                additions,
                deletions,
                unchanged,
            } => {
                assert_eq!(unchanged, 2);
                assert_eq!(deletions.len(), 1);
                assert_eq!(deletions[0].content, "line2");
                assert_eq!(additions.len(), 1);
                assert_eq!(additions[0].content, "modified");
            }
            _ => panic!("Expected Text diff"),
        }
    }

    #[test]
    fn test_diff_json_no_changes() {
        let old = json!({"key": "value"});
        let new = json!({"key": "value"});
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => assert!(changes.is_empty()),
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_diff_json_added_key() {
        let old = json!({"a": 1});
        let new = json!({"a": 1, "b": 2});
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].path, "$.b");
                assert_eq!(changes[0].change_type, "added");
            }
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_diff_json_removed_key() {
        let old = json!({"a": 1, "b": 2});
        let new = json!({"a": 1});
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].path, "$.b");
                assert_eq!(changes[0].change_type, "removed");
            }
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_diff_json_modified_value() {
        let old = json!({"a": 1});
        let new = json!({"a": 2});
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].path, "$.a");
                assert_eq!(changes[0].change_type, "modified");
                assert_eq!(changes[0].old_value.as_deref(), Some("1"));
                assert_eq!(changes[0].new_value.as_deref(), Some("2"));
            }
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_diff_json_nested() {
        let old = json!({"data": {"users": [{"name": "Alice"}]}});
        let new = json!({"data": {"users": [{"name": "Bob"}]}});
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].path, "$.data.users[0].name");
                assert_eq!(changes[0].change_type, "modified");
            }
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_diff_json_array_length_change() {
        let old = json!([1, 2, 3]);
        let new = json!([1, 2]);
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].path, "$[2]");
                assert_eq!(changes[0].change_type, "removed");
            }
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_diff_json_type_change() {
        let old = json!({"a": "string"});
        let new = json!({"a": 42});
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].change_type, "modified");
                assert_eq!(changes[0].old_value.as_deref(), Some("\"string\""));
                assert_eq!(changes[0].new_value.as_deref(), Some("42"));
            }
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_format_diff_text_no_diff() {
        let diff = TrafficDiff {
            request_diff: None,
            response_diff: None,
        };
        assert_eq!(format_diff_text(&diff), "No differences found.");
    }

    #[test]
    fn test_format_diff_text_with_header_diff() {
        let diff = TrafficDiff {
            request_diff: Some(TransactionPartDiff {
                method_diff: Some(("GET".to_string(), "POST".to_string())),
                url_diff: None,
                status_diff: None,
                header_diffs: vec![HeaderDiff::Added {
                    key: "x-new".to_string(),
                    value: "val".to_string(),
                }],
                body_diff: None,
            }),
            response_diff: None,
        };
        let text = format_diff_text(&diff);
        assert!(text.contains("Method: GET → POST"));
        assert!(text.contains("+ x-new: val"));
    }

    // ─── 대소문자 혼합 헤더 키 비교 ─────────────────────────

    #[test]
    fn test_diff_headers_case_insensitive() {
        let old = vec![("Content-Type".to_string(), "text/html".to_string())];
        let new = vec![("content-type".to_string(), "text/html".to_string())];
        let diffs = diff_headers(&old, &new);
        assert!(diffs.is_empty(), "동일 헤더의 대소문자 차이는 무시해야 함");
    }

    #[test]
    fn test_diff_headers_case_insensitive_modified() {
        let old = vec![("Content-Type".to_string(), "text/html".to_string())];
        let new = vec![("CONTENT-TYPE".to_string(), "application/json".to_string())];
        let diffs = diff_headers(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(&diffs[0], HeaderDiff::Modified { key, .. } if key == "content-type"));
    }

    // ─── 동일 키 중복 헤더 처리 ──────────────────────────────

    #[test]
    fn test_diff_headers_duplicate_keys() {
        let old = vec![
            ("set-cookie".to_string(), "a=1".to_string()),
            ("set-cookie".to_string(), "b=2".to_string()),
        ];
        let new = vec![
            ("set-cookie".to_string(), "a=1".to_string()),
            ("set-cookie".to_string(), "b=2".to_string()),
        ];
        let diffs = diff_headers(&old, &new);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_diff_headers_duplicate_keys_different() {
        let old = vec![
            ("set-cookie".to_string(), "a=1".to_string()),
            ("set-cookie".to_string(), "b=2".to_string()),
        ];
        let new = vec![
            ("set-cookie".to_string(), "a=1".to_string()),
            ("set-cookie".to_string(), "c=3".to_string()),
        ];
        let diffs = diff_headers(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert!(
            matches!(&diffs[0], HeaderDiff::Modified { key, old_value, new_value }
            if key == "set-cookie" && old_value == "a=1, b=2" && new_value == "a=1, c=3")
        );
    }

    // ─── 빈 문자열 텍스트 diff ───────────────────────────────

    #[test]
    fn test_diff_text_empty_strings() {
        let body = diff_text("", "");
        match body {
            BodyDiff::Text {
                additions,
                deletions,
                unchanged,
            } => {
                assert!(additions.is_empty());
                assert!(deletions.is_empty());
                assert_eq!(unchanged, 0);
            }
            _ => panic!("Expected Text diff"),
        }
    }

    #[test]
    fn test_diff_text_one_side_empty() {
        let body = diff_text("", "hello\nworld\n");
        match body {
            BodyDiff::Text {
                additions,
                deletions,
                unchanged,
            } => {
                assert_eq!(additions.len(), 2);
                assert!(deletions.is_empty());
                assert_eq!(unchanged, 0);
            }
            _ => panic!("Expected Text diff"),
        }

        let body2 = diff_text("hello\nworld\n", "");
        match body2 {
            BodyDiff::Text {
                additions,
                deletions,
                unchanged,
            } => {
                assert!(additions.is_empty());
                assert_eq!(deletions.len(), 2);
                assert_eq!(unchanged, 0);
            }
            _ => panic!("Expected Text diff"),
        }
    }

    // ─── JSON diff 추가 케이스 ───────────────────────────────

    #[test]
    fn test_diff_json_empty_objects() {
        let old = json!({});
        let new = json!({});
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => assert!(changes.is_empty()),
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_diff_json_null_values() {
        let old = json!({"a": null});
        let new = json!({"a": null});
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => assert!(changes.is_empty()),
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_diff_json_null_to_value() {
        let old = json!({"a": null});
        let new = json!({"a": 42});
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].path, "$.a");
                assert_eq!(changes[0].change_type, "modified");
                assert_eq!(changes[0].old_value.as_deref(), Some("null"));
                assert_eq!(changes[0].new_value.as_deref(), Some("42"));
            }
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_diff_json_deeply_nested() {
        let old = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": "old_value"
                    }
                }
            }
        });
        let new = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": "new_value"
                    }
                }
            }
        });
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].path, "$.level1.level2.level3.level4");
                assert_eq!(changes[0].change_type, "modified");
            }
            _ => panic!("Expected Json diff"),
        }
    }

    #[test]
    fn test_diff_json_array_of_objects() {
        let old = json!([
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]);
        let new = json!([
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Charlie"}
        ]);
        match diff_json(&old, &new) {
            BodyDiff::Json { changes } => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].path, "$[1].name");
                assert_eq!(changes[0].change_type, "modified");
                assert_eq!(changes[0].old_value.as_deref(), Some("\"Bob\""));
                assert_eq!(changes[0].new_value.as_deref(), Some("\"Charlie\""));
            }
            _ => panic!("Expected Json diff"),
        }
    }

    // ─── format_diff_text 추가 케이스 ────────────────────────

    #[test]
    fn test_format_diff_text_json_body() {
        let diff = TrafficDiff {
            request_diff: None,
            response_diff: Some(TransactionPartDiff {
                method_diff: None,
                url_diff: None,
                status_diff: Some((200, 201)),
                header_diffs: vec![],
                body_diff: Some(BodyDiff::Json {
                    changes: vec![
                        JsonDiffEntry {
                            path: "$.name".to_string(),
                            change_type: "modified".to_string(),
                            old_value: Some("\"old\"".to_string()),
                            new_value: Some("\"new\"".to_string()),
                        },
                        JsonDiffEntry {
                            path: "$.age".to_string(),
                            change_type: "added".to_string(),
                            old_value: None,
                            new_value: Some("30".to_string()),
                        },
                        JsonDiffEntry {
                            path: "$.removed_field".to_string(),
                            change_type: "removed".to_string(),
                            old_value: Some("\"val\"".to_string()),
                            new_value: None,
                        },
                    ],
                }),
            }),
        };
        let text = format_diff_text(&diff);
        assert!(text.contains("Status: 200 → 201"));
        assert!(text.contains("3 JSON changes:"));
        assert!(text.contains("~ $.name : \"old\" → \"new\""));
        assert!(text.contains("+ $.age = 30"));
        assert!(text.contains("- $.removed_field = \"val\""));
    }

    #[test]
    fn test_format_diff_text_binary_body() {
        let diff = TrafficDiff {
            request_diff: Some(TransactionPartDiff {
                method_diff: None,
                url_diff: None,
                status_diff: None,
                header_diffs: vec![],
                body_diff: Some(BodyDiff::Binary {
                    old_size: 1024,
                    new_size: 2048,
                }),
            }),
            response_diff: None,
        };
        let text = format_diff_text(&diff);
        assert!(text.contains("Binary content: 1024 bytes → 2048 bytes"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let diff = TrafficDiff {
            request_diff: Some(TransactionPartDiff {
                method_diff: Some(("GET".to_string(), "POST".to_string())),
                url_diff: None,
                status_diff: None,
                header_diffs: vec![],
                body_diff: Some(BodyDiff::Binary {
                    old_size: 100,
                    new_size: 200,
                }),
            }),
            response_diff: None,
        };
        let json = serde_json::to_string(&diff).unwrap();
        let parsed: TrafficDiff = serde_json::from_str(&json).unwrap();
        assert!(parsed.request_diff.is_some());
        assert!(parsed.response_diff.is_none());
    }
}
