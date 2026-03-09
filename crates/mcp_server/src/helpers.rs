use std::sync::atomic::{AtomicU32, Ordering};

use rmcp::model::*;
use rmcp::ErrorData as McpError;

pub fn tool_error(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(msg.into())]))
}

pub fn tool_ok(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(msg.into())]))
}

pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn next_rule_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!("mcp_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn next_breakpoint_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!("mcp_bp_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn read_body_text(file_path: &Option<String>, data_type: &proxy_v2_models::DataType) -> String {
    let Some(path) = file_path else {
        return "(body not available)".to_string();
    };
    let Ok(bytes) = std::fs::read(path) else {
        return "(file read error)".to_string();
    };
    if !data_type.is_text_based() {
        return format!("(binary, {:?})", data_type);
    }
    match String::from_utf8(bytes) {
        Ok(text) => {
            if text.len() > 10000 {
                format!(
                    "{}...\n(truncated, {} total)",
                    &text[..10000],
                    format_size(text.len())
                )
            } else {
                text
            }
        }
        Err(_) => "(binary data)".to_string(),
    }
}
