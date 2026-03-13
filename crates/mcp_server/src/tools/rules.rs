use proxy_daemon::{InterceptAction, InterceptRule};
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{add_and_sync, list_items, next_rule_id, remove_and_sync, tool_error};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(description = "List all current intercept rules (block, modify, map local/remote).")]
    pub(crate) async fn list_rules(&self) -> Result<CallToolResult, McpError> {
        list_items(&self.store.rules, "rules", "No intercept rules configured.")
    }

    #[tool(
        description = "Add a new intercept rule. Supports: block, modify_request, modify_response, map_local, map_remote."
    )]
    pub(crate) async fn add_rule(
        &self,
        Parameters(p): Parameters<AddRuleParams>,
    ) -> Result<CallToolResult, McpError> {
        let action = match p.action_type.as_str() {
            "block" => InterceptAction::Block {
                status_code: p.status_code.unwrap_or(403),
                body: p.response_body.unwrap_or_default(),
            },
            "modify_request" => InterceptAction::ModifyRequest {
                add_headers: p.add_headers.unwrap_or_default(),
                remove_headers: p.remove_headers.unwrap_or_default(),
                set_body: p.response_body,
            },
            "modify_response" => InterceptAction::ModifyResponse {
                set_status: p.status_code,
                add_headers: p.add_headers.unwrap_or_default(),
                remove_headers: p.remove_headers.unwrap_or_default(),
                set_body: p.response_body,
            },
            "map_local" => {
                let Some(file_path) = p.file_path else {
                    return tool_error("file_path is required for map_local");
                };
                InterceptAction::MapLocal {
                    file_path,
                    status_code: p.status_code.unwrap_or(200),
                    headers: p.add_headers.unwrap_or_default(),
                }
            }
            "map_remote" => {
                let Some(target_url) = p.target_url else {
                    return tool_error("target_url is required for map_remote");
                };
                InterceptAction::MapRemote {
                    target_url,
                    preserve_path: p.preserve_path.unwrap_or(true),
                }
            }
            other => {
                return tool_error(format!(
                    "Unknown action_type '{}'. Use: block, modify_request, modify_response, map_local, map_remote",
                    other
                ));
            }
        };

        let id = next_rule_id();
        let rule = InterceptRule {
            id: id.clone(),
            name: p.name,
            enabled: true,
            pattern: p.pattern,
            method: p.method,
            action,
        };

        add_and_sync(&self.store.rules, rule, &id, "Rule", || self.send_rules()).await
    }

    #[tool(description = "Remove an intercept rule by its ID.")]
    pub(crate) async fn remove_rule(
        &self,
        Parameters(p): Parameters<RemoveRuleParams>,
    ) -> Result<CallToolResult, McpError> {
        remove_and_sync(
            &self.store.rules,
            &p.id,
            |r| &r.id,
            "Rule",
            || self.send_rules(),
        )
        .await
    }
}
