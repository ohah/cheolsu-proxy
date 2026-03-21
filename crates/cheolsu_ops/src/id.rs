use std::sync::atomic::{AtomicU32, Ordering};

static RULE_COUNTER: AtomicU32 = AtomicU32::new(0);
static BREAKPOINT_COUNTER: AtomicU32 = AtomicU32::new(0);
static MAPPING_COUNTER: AtomicU32 = AtomicU32::new(0);
static REVERSE_PROXY_COUNTER: AtomicU32 = AtomicU32::new(0);
static SERVER_REPLAY_COUNTER: AtomicU32 = AtomicU32::new(0);

pub fn next_rule_id() -> String {
    format!("rule_{}", RULE_COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn next_breakpoint_id() -> String {
    format!("bp_{}", BREAKPOINT_COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn next_mapping_id() -> String {
    format!("hm_{}", MAPPING_COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn next_reverse_proxy_id() -> String {
    format!(
        "rp_{}",
        REVERSE_PROXY_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

pub fn next_server_replay_id() -> String {
    format!(
        "sr_{}",
        SERVER_REPLAY_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}
