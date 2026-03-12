use super::{BodyDiff, HeaderDiff, TrafficDiff, TransactionPartDiff};

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
