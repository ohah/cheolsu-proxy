use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{format_size, tool_error, tool_ok};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Send an HTTP request directly (bypassing the proxy). Useful for testing and replaying captured requests."
    )]
    pub(crate) async fn replay_request(
        &self,
        Parameters(p): Parameters<ReplayRequestParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = match reqwest::Client::builder()
            .no_proxy()
            .danger_accept_invalid_certs(true)
            .build()
        {
            Ok(c) => c,
            Err(e) => return tool_error(format!("Failed to create HTTP client: {}", e)),
        };

        let method: reqwest::Method = match p.method.parse() {
            Ok(m) => m,
            Err(e) => return tool_error(format!("Invalid HTTP method: {}", e)),
        };

        let mut builder = client.request(method, &p.url);
        if let Some(headers) = p.headers {
            for (k, v) in headers {
                builder = builder.header(k, v);
            }
        }
        if let Some(body) = p.body {
            builder = builder.body(body);
        }

        let start = std::time::Instant::now();
        let response = match builder.send().await {
            Ok(r) => r,
            Err(e) => return tool_error(format!("Request failed: {}", e)),
        };
        let elapsed = start.elapsed();

        let status = response.status().as_u16();
        let headers: Vec<String> = response
            .headers()
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v.to_str().unwrap_or("<binary>")))
            .collect();
        let body_bytes = match response.bytes().await {
            Ok(b) => b,
            Err(e) => return tool_error(format!("Failed to read response body: {}", e)),
        };

        let body_text = String::from_utf8(body_bytes.to_vec())
            .unwrap_or_else(|_| format!("<binary, {} bytes>", body_bytes.len()));
        let body_display = if body_text.len() > 10000 {
            format!(
                "{}...\n(truncated, {} total)",
                &body_text[..10000],
                format_size(body_text.len())
            )
        } else {
            body_text
        };

        tool_ok(format!(
            "## Response\nStatus: {}\nTime: {:.0?}\nSize: {}\n\n### Headers\n```\n{}\n```\n\n### Body\n```\n{}\n```",
            status,
            elapsed,
            format_size(body_bytes.len()),
            headers.join("\n"),
            body_display,
        ))
    }
}
