use crate::context::OpsContext;
use crate::params::*;
use crate::result::OpResult;

pub fn export_har(ctx: &OpsContext, p: ExportHarParams) -> OpResult {
    let txns = ctx.store.transactions.lock();

    let filtered: Vec<proxy_v2_models::RequestInfo> = txns
        .iter()
        .filter(|info| {
            let Some(req) = &info.request else {
                return false;
            };
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
        .cloned()
        .collect();

    if filtered.is_empty() {
        return OpResult::ok("No matching transactions found.");
    }

    let count = filtered.len();
    let json_str = match proxy_v2_models::har::build_har_json(&filtered) {
        Ok(s) => s,
        Err(e) => return OpResult::err(format!("Failed to serialize HAR: {}", e)),
    };

    let path = &p.output_path;
    let file_path = std::path::Path::new(path);
    if let Some(parent) = file_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return OpResult::err(format!("Failed to create directory: {}", e));
        }
    }

    match std::fs::write(file_path, &json_str) {
        Ok(()) => OpResult::ok(format!("HAR file saved to '{}' ({} entries).", path, count)),
        Err(e) => OpResult::err(format!("Failed to write HAR file: {}", e)),
    }
}
