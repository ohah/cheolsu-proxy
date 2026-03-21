use proxy_daemon::ClientCommand;

use crate::context::OpsContext;
use crate::helpers::with_daemon_conn;
use crate::params::*;
use crate::result::OpResult;

pub async fn load_script(ctx: &OpsContext, p: LoadScriptParams) -> OpResult {
    if p.path.is_none() && p.code.is_none() {
        return OpResult::err("Either 'path' or 'code' must be provided.");
    }
    let cmd = ClientCommand::LoadScript {
        path: p.path.clone(),
        code: p.code.clone(),
    };
    match with_daemon_conn(&ctx.daemon_conn, &cmd).await {
        Ok(()) => {
            let source = if let Some(ref path) = p.path {
                format!("file '{}'", path)
            } else {
                "inline code".to_string()
            };
            OpResult::ok(format!("Script loaded from {}.", source))
        }
        Err(e) => OpResult::err(format!("Failed to load script: {}", e)),
    }
}

pub async fn unload_script(ctx: &OpsContext) -> OpResult {
    match with_daemon_conn(&ctx.daemon_conn, &ClientCommand::UnloadScript).await {
        Ok(()) => OpResult::ok("Script unloaded."),
        Err(e) => OpResult::err(format!("Failed to unload script: {}", e)),
    }
}
