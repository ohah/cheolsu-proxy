use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{tool_error, tool_ok};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Export captured HTTP traffic as a HAR (HTTP Archive) 1.2 JSON file. Optionally filter by host or path. Saves to the specified file path."
    )]
    pub(crate) async fn export_har(
        &self,
        Parameters(p): Parameters<ExportHarParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();

        let filtered: Vec<&proxy_v2_models::RequestInfo> = txns
            .iter()
            .filter(|info| {
                let Some(req) = &info.0 else { return false };
                let uri = req.uri().to_string();

                if let Some(ref host) = p.host {
                    if !uri.to_lowercase().contains(&host.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref path) = p.path {
                    if !uri.to_lowercase().contains(&path.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .collect();

        if filtered.is_empty() {
            return tool_ok("No matching transactions found.");
        }

        let mut entries = Vec::new();
        for info in &filtered {
            let mut entry = serde_json::Map::new();

            if let Some(req) = &info.0 {
                let mut har_req = serde_json::Map::new();
                har_req.insert(
                    "method".into(),
                    serde_json::Value::String(req.method().to_string()),
                );
                har_req.insert(
                    "url".into(),
                    serde_json::Value::String(req.uri().to_string()),
                );
                har_req.insert(
                    "httpVersion".into(),
                    serde_json::Value::String(format!("{:?}", req.version())),
                );

                let headers: Vec<serde_json::Value> = req
                    .headers()
                    .iter()
                    .map(|(k, v)| {
                        serde_json::json!({
                            "name": k.to_string(),
                            "value": v.to_str().unwrap_or("<binary>"),
                        })
                    })
                    .collect();
                har_req.insert("headers".into(), serde_json::Value::Array(headers));
                har_req.insert("headersSize".into(), serde_json::Value::Number((-1).into()));
                har_req.insert(
                    "bodySize".into(),
                    serde_json::Value::Number((req.body_size() as i64).into()),
                );

                entry.insert("request".into(), serde_json::Value::Object(har_req));
            }

            if let Some(res) = &info.1 {
                let mut har_res = serde_json::Map::new();
                har_res.insert(
                    "status".into(),
                    serde_json::Value::Number((res.status().as_u16() as i64).into()),
                );
                har_res.insert(
                    "statusText".into(),
                    serde_json::Value::String(
                        res.status().canonical_reason().unwrap_or("").to_string(),
                    ),
                );
                har_res.insert(
                    "httpVersion".into(),
                    serde_json::Value::String(format!("{:?}", res.version())),
                );

                let headers: Vec<serde_json::Value> = res
                    .headers()
                    .iter()
                    .map(|(k, v)| {
                        serde_json::json!({
                            "name": k.to_string(),
                            "value": v.to_str().unwrap_or("<binary>"),
                        })
                    })
                    .collect();
                har_res.insert("headers".into(), serde_json::Value::Array(headers));

                let mut content = serde_json::Map::new();
                content.insert(
                    "size".into(),
                    serde_json::Value::Number((res.body_size() as i64).into()),
                );
                content.insert(
                    "mimeType".into(),
                    serde_json::Value::String(format!("{:?}", res.data_type())),
                );
                har_res.insert("content".into(), serde_json::Value::Object(content));

                har_res.insert("headersSize".into(), serde_json::Value::Number((-1).into()));
                har_res.insert(
                    "bodySize".into(),
                    serde_json::Value::Number((res.body_size() as i64).into()),
                );

                entry.insert("response".into(), serde_json::Value::Object(har_res));
            }

            entry.insert(
                "startedDateTime".into(),
                serde_json::Value::String(String::new()),
            );
            entry.insert(
                "time".into(),
                serde_json::Value::Number(serde_json::Number::from(0)),
            );
            entry.insert(
                "timings".into(),
                serde_json::json!({"send": 0, "wait": 0, "receive": 0}),
            );
            entry.insert("cache".into(), serde_json::json!({}));

            entries.push(serde_json::Value::Object(entry));
        }

        let har = serde_json::json!({
            "log": {
                "version": "1.2",
                "creator": {
                    "name": "Cheolsu Proxy",
                    "version": "1.0"
                },
                "entries": entries
            }
        });

        let path = &p.output_path;
        let file_path = std::path::Path::new(path);
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return tool_error(format!("Failed to create directory: {}", e));
                }
            }
        }

        let json_str = serde_json::to_string_pretty(&har)
            .unwrap_or_else(|e| format!("Serialization error: {}", e));

        match std::fs::write(file_path, &json_str) {
            Ok(()) => tool_ok(format!(
                "HAR file saved to '{}' ({} entries).",
                path,
                filtered.len()
            )),
            Err(e) => tool_error(format!("Failed to write HAR file: {}", e)),
        }
    }
}
