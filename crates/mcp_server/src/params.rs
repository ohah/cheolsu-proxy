use std::collections::HashMap;

use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchTrafficParams {
    /// Filter by hostname or URL substring
    pub host: Option<String>,
    /// Filter by HTTP method (GET, POST, etc.)
    pub method: Option<String>,
    /// Filter by response status code
    pub status: Option<u16>,
    /// Filter by URL path substring
    pub path: Option<String>,
    /// Maximum results to return (default: 50)
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTransactionParams {
    /// Transaction ID (from search_traffic results)
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetWsMessagesParams {
    /// Filter by connection URI substring
    pub connection_id: Option<String>,
    /// Maximum results (default: 100)
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReplayRequestParams {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// Full URL to send the request to
    pub url: String,
    /// Request headers as key-value pairs
    pub headers: Option<HashMap<String, String>>,
    /// Request body content
    pub body: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddRuleParams {
    /// Rule display name
    pub name: String,
    /// URL pattern with wildcards (e.g., *.example.com/api/*)
    pub pattern: String,
    /// HTTP method filter (optional)
    pub method: Option<String>,
    /// Action: "block", "modify_request", "modify_response", "map_local", "map_remote"
    pub action_type: String,
    /// Status code (for block: default 403, for modify_response)
    pub status_code: Option<u16>,
    /// Body content (for block/modify_request/modify_response)
    pub response_body: Option<String>,
    /// Headers to add
    pub add_headers: Option<HashMap<String, String>>,
    /// Header names to remove
    pub remove_headers: Option<Vec<String>>,
    /// Local file path (required for map_local)
    pub file_path: Option<String>,
    /// Target URL (required for map_remote)
    pub target_url: Option<String>,
    /// Preserve original path for map_remote (default: true)
    pub preserve_path: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RemoveRuleParams {
    /// Rule ID to remove
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LoadScriptParams {
    /// File path to a JavaScript/TypeScript script
    pub path: Option<String>,
    /// Inline JavaScript/TypeScript code
    pub code: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UnloadScriptParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DiffTransactionsParams {
    /// First transaction ID (from search_traffic results)
    pub transaction_id_a: String,
    /// Second transaction ID (from search_traffic results)
    pub transaction_id_b: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddBreakpointParams {
    /// URL pattern with wildcards (e.g., *.example.com/api/*)
    pub pattern: String,
    /// Break on request (default: true)
    pub break_on_request: Option<bool>,
    /// Break on response (default: false)
    pub break_on_response: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RemoveBreakpointParams {
    /// Breakpoint rule ID to remove
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResolveBreakpointParams {
    /// Breakpoint ID to resolve (from list_pending_breakpoints)
    pub id: String,
    /// Action: "forward", "modify_and_forward", "drop", "abort"
    pub action: String,
    /// Headers to set (for modify_and_forward)
    pub headers: Option<HashMap<String, String>>,
    /// Body to set (for modify_and_forward)
    pub body: Option<String>,
    /// Status code to set (for modify_and_forward, response only)
    pub status: Option<u16>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SaveSessionParams {
    /// File path to save the session (extension .cheolsu will be added if missing, use .cheolsu.gz for gzip compression)
    pub path: String,
    /// Optional filter: only save transactions matching this URL substring
    pub filter: Option<String>,
    /// Optional session name
    pub name: Option<String>,
    /// Optional session description
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LoadSessionParams {
    /// File path to load the session from (.cheolsu or .cheolsu.gz or .har)
    pub path: String,
    /// If true, append to existing traffic instead of replacing
    #[serde(default)]
    pub append: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddHostMappingParams {
    /// Source host pattern, supports wildcards (e.g., "*.api.example.com")
    pub source_host: String,
    /// Source port filter (optional, None = any port)
    pub source_port: Option<u16>,
    /// Target host (IP address or domain name)
    pub target_host: String,
    /// Target port (optional, None = keep original port)
    pub target_port: Option<u16>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RemoveHostMappingParams {
    /// Host mapping ID to remove
    pub id: String,
}
