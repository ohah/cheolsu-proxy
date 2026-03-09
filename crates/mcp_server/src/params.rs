use std::collections::HashMap;

use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct SearchTrafficParams {
    /// Filter by hostname or URL substring
    pub(crate) host: Option<String>,
    /// Filter by HTTP method (GET, POST, etc.)
    pub(crate) method: Option<String>,
    /// Filter by response status code
    pub(crate) status: Option<u16>,
    /// Filter by URL path substring
    pub(crate) path: Option<String>,
    /// Maximum results to return (default: 50)
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct GetTransactionParams {
    /// Transaction ID (from search_traffic results)
    pub(crate) id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct GetWsMessagesParams {
    /// Filter by connection URI substring
    pub(crate) connection_id: Option<String>,
    /// Maximum results (default: 100)
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ReplayRequestParams {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub(crate) method: String,
    /// Full URL to send the request to
    pub(crate) url: String,
    /// Request headers as key-value pairs
    pub(crate) headers: Option<HashMap<String, String>>,
    /// Request body content
    pub(crate) body: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct AddRuleParams {
    /// Rule display name
    pub(crate) name: String,
    /// URL pattern with wildcards (e.g., *.example.com/api/*)
    pub(crate) pattern: String,
    /// HTTP method filter (optional)
    pub(crate) method: Option<String>,
    /// Action: "block", "modify_request", "modify_response", "map_local", "map_remote"
    pub(crate) action_type: String,
    /// Status code (for block: default 403, for modify_response)
    pub(crate) status_code: Option<u16>,
    /// Body content (for block/modify_request/modify_response)
    pub(crate) response_body: Option<String>,
    /// Headers to add
    pub(crate) add_headers: Option<HashMap<String, String>>,
    /// Header names to remove
    pub(crate) remove_headers: Option<Vec<String>>,
    /// Local file path (required for map_local)
    pub(crate) file_path: Option<String>,
    /// Target URL (required for map_remote)
    pub(crate) target_url: Option<String>,
    /// Preserve original path for map_remote (default: true)
    pub(crate) preserve_path: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct RemoveRuleParams {
    /// Rule ID to remove
    pub(crate) id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct LoadScriptParams {
    /// File path to a JavaScript/TypeScript script
    pub(crate) path: Option<String>,
    /// Inline JavaScript/TypeScript code
    pub(crate) code: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct UnloadScriptParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct DiffTransactionsParams {
    /// First transaction ID (from search_traffic results)
    pub(crate) transaction_id_a: String,
    /// Second transaction ID (from search_traffic results)
    pub(crate) transaction_id_b: String,
}
