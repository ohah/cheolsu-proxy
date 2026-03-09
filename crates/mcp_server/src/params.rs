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
