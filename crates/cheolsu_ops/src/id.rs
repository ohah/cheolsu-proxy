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
    // n == u32::MAX인 ID("rule_4294967295")를 로드하면 n + 1이 오버플로(디버그 패닉/
    // 릴리스 wrap)된다. saturating_add로 패닉 없이 카운터를 끌어올린다.
    let target = n.saturating_add(1);
    while n >= cur {
        match counter.compare_exchange_weak(cur, target, Ordering::Relaxed, Ordering::Relaxed) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_at_u32_max_does_not_overflow() {
        let counter = AtomicU32::new(0);
        // n == u32::MAX인 ID를 관찰해도 패닉 없이 종료해야 한다.
        observe_counter(&counter, "rule_4294967295", "rule_");
        assert_eq!(counter.load(Ordering::Relaxed), u32::MAX);
    }

    #[test]
    fn observe_raises_counter_above_loaded_id() {
        let counter = AtomicU32::new(0);
        observe_counter(&counter, "rule_41", "rule_");
        assert_eq!(counter.load(Ordering::Relaxed), 42);
    }

    #[test]
    fn observe_ignores_mismatched_prefix_or_garbage() {
        let counter = AtomicU32::new(7);
        observe_counter(&counter, "bp_100", "rule_");
        observe_counter(&counter, "rule_abc", "rule_");
        assert_eq!(counter.load(Ordering::Relaxed), 7);
    }
}
