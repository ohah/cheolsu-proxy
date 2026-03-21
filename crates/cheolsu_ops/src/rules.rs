use proxy_daemon::{InterceptAction, InterceptRule};

use crate::context::OpsContext;
use crate::helpers::{add_and_sync, list_items, remove_and_sync};
use crate::id::next_rule_id;
use crate::params::*;
use crate::result::OpResult;

pub fn list_rules(ctx: &OpsContext) -> OpResult {
    list_items(&ctx.store.rules, "rules", "No intercept rules configured.")
}

pub async fn add_rule(ctx: &OpsContext, p: AddRuleParams) -> OpResult {
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
                return OpResult::err("file_path is required for map_local");
            };
            InterceptAction::MapLocal {
                file_path,
                status_code: p.status_code.unwrap_or(200),
                headers: p.add_headers.unwrap_or_default(),
            }
        }
        "map_remote" => {
            let Some(target_url) = p.target_url else {
                return OpResult::err("target_url is required for map_remote");
            };
            InterceptAction::MapRemote {
                target_url,
                preserve_path: p.preserve_path.unwrap_or(true),
            }
        }
        other => {
            return OpResult::err(format!(
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

    add_and_sync(&ctx.store.rules, rule, &id, "Rule", || ctx.send_rules()).await
}

pub async fn remove_rule(ctx: &OpsContext, p: RemoveRuleParams) -> OpResult {
    remove_and_sync(
        &ctx.store.rules,
        &p.id,
        |r| &r.id,
        "Rule",
        || ctx.send_rules(),
    )
    .await
}
