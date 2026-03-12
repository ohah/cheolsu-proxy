use similar::{ChangeTag, TextDiff};
use std::collections::{BTreeMap, BTreeSet};

use super::types::{BodyDiff, DiffLine, HeaderDiff, JsonDiffEntry};

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

/// Check if a data type string represents text-based content.
pub fn is_text_data_type(dt: &str) -> bool {
    matches!(
        dt,
        "Json" | "GraphQL" | "Html" | "Css" | "JavaScript" | "Xml" | "Text" | "FormUrlEncoded"
    )
}

fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("\"{}\"", s),
        other => other.to_string(),
    }
}
