use crate::context::OpsContext;
use crate::params::*;
use crate::result::OpResult;

pub fn get_sse_events(ctx: &OpsContext, p: GetSseEventsParams) -> OpResult {
    let events = ctx.store.sse_events.lock();
    let limit = p.limit.unwrap_or(100);

    let results: Vec<String> = events
        .iter()
        .rev()
        .filter(|evt| {
            p.connection_id.as_ref().map_or(true, |cid| {
                evt.connection_id
                    .to_lowercase()
                    .contains(&cid.to_lowercase())
            })
        })
        .take(limit)
        .map(|evt| {
            let event_type = evt.event_type.as_deref().unwrap_or("message").to_string();
            let data = if evt.data.len() > 200 {
                format!("{}...", &evt.data[..200])
            } else {
                evt.data.clone()
            };
            format!(
                "[{}] type={} ({} bytes) [{}]: {}",
                evt.sequence, event_type, evt.size, evt.connection_id, data,
            )
        })
        .collect();

    if results.is_empty() {
        OpResult::ok("No SSE events captured.")
    } else {
        OpResult::ok(format!(
            "Found {} SSE events:\n\n{}",
            results.len(),
            results.join("\n\n")
        ))
    }
}
