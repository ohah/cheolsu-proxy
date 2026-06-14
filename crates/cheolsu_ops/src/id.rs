use std::sync::atomic::{AtomicU32, Ordering};

static RULE_COUNTER: AtomicU32 = AtomicU32::new(0);
static BREAKPOINT_COUNTER: AtomicU32 = AtomicU32::new(0);
static MAPPING_COUNTER: AtomicU32 = AtomicU32::new(0);
static REVERSE_PROXY_COUNTER: AtomicU32 = AtomicU32::new(0);
static SERVER_REPLAY_COUNTER: AtomicU32 = AtomicU32::new(0);

/// 외부에서 로드된 ID("{prefix}{n}")를 관찰하여 카운터를 n+1 이상으로 끌어올린다.
/// (세션/HAR 로드 시 기존 ID와 새 ID가 충돌하는 것을 방지)
fn observe_counter(counter: &AtomicU32, id: &str, prefix: &str) {
    let Some(n) = id.strip_prefix(prefix).and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    let mut cur = counter.load(Ordering::Relaxed);
    while n >= cur {
        match counter.compare_exchange_weak(cur, n + 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => cur = actual,
        }
    }
}

/// 로드된 규칙 ID를 관찰하여 rule 카운터 충돌을 방지한다.
pub fn observe_rule_id(id: &str) {
    observe_counter(&RULE_COUNTER, id, "rule_");
}

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
