use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, oneshot, Mutex};
use tracing::{info, warn};

use crate::protocol::{
    BreakpointAction, BreakpointData, BreakpointPhase, BreakpointRule, DaemonMessage,
};

/// Pending breakpoint entry waiting for resolution.
struct PendingBreakpoint {
    resolver: oneshot::Sender<BreakpointAction>,
}

/// Manages breakpoint rules, matching, and pending breakpoint state.
#[derive(Clone)]
pub struct BreakpointManager {
    rules: Arc<Mutex<Vec<BreakpointRule>>>,
    pending: Arc<Mutex<HashMap<String, PendingBreakpoint>>>,
    event_tx: broadcast::Sender<String>,
    timeout_secs: u64,
    counter: Arc<std::sync::atomic::AtomicU64>,
}

impl BreakpointManager {
    pub fn new(event_tx: broadcast::Sender<String>) -> Self {
        Self {
            rules: Arc::new(Mutex::new(Vec::new())),
            pending: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            timeout_secs: 60,
            counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn update_rules(&self, rules: Vec<BreakpointRule>) {
        let mut guard = self.rules.lock().await;
        info!("[Breakpoint] Rules updated: {} rules", rules.len());
        *guard = rules;
    }

    pub async fn get_rules(&self) -> Vec<BreakpointRule> {
        self.rules.lock().await.clone()
    }

    fn next_id(&self) -> String {
        let n = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("bp_{}", n)
    }

    /// Check if a URL matches any enabled breakpoint rule for the given phase.
    pub async fn should_break(&self, url: &str, phase: &BreakpointPhase) -> bool {
        let rules = self.rules.lock().await;
        rules.iter().any(|rule| {
            if !rule.enabled {
                return false;
            }
            let phase_match = match phase {
                BreakpointPhase::Request => rule.break_on_request,
                BreakpointPhase::Response => rule.break_on_response,
            };
            if !phase_match {
                return false;
            }
            crate::handler::LoggingHandler::wildcard_matches(&rule.pattern, url)
        })
    }

    /// Pause execution and wait for a client to resolve this breakpoint.
    /// Returns the action chosen by the client, or Forward on timeout.
    pub async fn pause_and_wait(
        &self,
        transaction_id: &str,
        phase: BreakpointPhase,
        data: BreakpointData,
    ) -> BreakpointAction {
        let bp_id = self.next_id();
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            pending.insert(bp_id.clone(), PendingBreakpoint { resolver: tx });
        }

        let msg = DaemonMessage::BreakpointHit {
            id: bp_id.clone(),
            transaction_id: transaction_id.to_string(),
            phase,
            data,
        };
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = self.event_tx.send(json);
        }

        let timeout = tokio::time::Duration::from_secs(self.timeout_secs);
        let result = tokio::time::timeout(timeout, rx).await;

        // Clean up pending entry
        {
            let mut pending = self.pending.lock().await;
            pending.remove(&bp_id);
        }

        match result {
            Ok(Ok(action)) => action,
            Ok(Err(_)) => {
                warn!(
                    "[Breakpoint] Resolver dropped for {}, auto-forwarding",
                    bp_id
                );
                BreakpointAction::Forward
            }
            Err(_) => {
                warn!(
                    "[Breakpoint] Timeout ({}s) for {}, auto-forwarding",
                    self.timeout_secs, bp_id
                );
                BreakpointAction::Forward
            }
        }
    }

    /// Resolve a pending breakpoint by ID.
    pub async fn resolve(&self, id: &str, action: BreakpointAction) -> Result<(), String> {
        let mut pending = self.pending.lock().await;
        match pending.remove(id) {
            Some(entry) => {
                let _ = entry.resolver.send(action);
                Ok(())
            }
            None => Err(format!("No pending breakpoint with id '{}'", id)),
        }
    }

    /// List currently pending breakpoint IDs.
    pub async fn list_pending(&self) -> Vec<String> {
        let pending = self.pending.lock().await;
        pending.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{BreakpointAction, BreakpointData, BreakpointPhase, BreakpointRule};

    fn make_rule(pattern: &str, on_req: bool, on_res: bool) -> BreakpointRule {
        BreakpointRule {
            id: "test".to_string(),
            pattern: pattern.to_string(),
            break_on_request: on_req,
            break_on_response: on_res,
            enabled: true,
        }
    }

    fn make_manager() -> BreakpointManager {
        let (tx, _) = broadcast::channel(16);
        BreakpointManager::new(tx)
    }

    #[tokio::test]
    async fn test_should_break_request_match() {
        let mgr = make_manager();
        mgr.update_rules(vec![make_rule("*example.com*", true, false)])
            .await;
        assert!(
            mgr.should_break("https://example.com/api", &BreakpointPhase::Request)
                .await
        );
        assert!(
            !mgr.should_break("https://example.com/api", &BreakpointPhase::Response)
                .await
        );
    }

    #[tokio::test]
    async fn test_should_break_response_match() {
        let mgr = make_manager();
        mgr.update_rules(vec![make_rule("*example.com*", false, true)])
            .await;
        assert!(
            !mgr.should_break("https://example.com/api", &BreakpointPhase::Request)
                .await
        );
        assert!(
            mgr.should_break("https://example.com/api", &BreakpointPhase::Response)
                .await
        );
    }

    #[tokio::test]
    async fn test_should_break_disabled_rule() {
        let mgr = make_manager();
        let mut rule = make_rule("*example.com*", true, true);
        rule.enabled = false;
        mgr.update_rules(vec![rule]).await;
        assert!(
            !mgr.should_break("https://example.com/api", &BreakpointPhase::Request)
                .await
        );
    }

    #[tokio::test]
    async fn test_should_break_no_match() {
        let mgr = make_manager();
        mgr.update_rules(vec![make_rule("*other.com*", true, true)])
            .await;
        assert!(
            !mgr.should_break("https://example.com/api", &BreakpointPhase::Request)
                .await
        );
    }

    #[tokio::test]
    async fn test_resolve_pending() {
        let mgr = make_manager();
        mgr.update_rules(vec![make_rule("*example.com*", true, false)])
            .await;

        let mgr_clone = mgr.clone();
        let handle = tokio::spawn(async move {
            let data = BreakpointData {
                method: "GET".to_string(),
                url: "https://example.com".to_string(),
                headers: Default::default(),
                body: None,
                status: None,
            };
            mgr_clone
                .pause_and_wait("txn_1", BreakpointPhase::Request, data)
                .await
        });

        // Wait a bit for the breakpoint to be registered
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let pending = mgr.list_pending().await;
        assert_eq!(pending.len(), 1);

        let bp_id = pending[0].clone();
        mgr.resolve(&bp_id, BreakpointAction::Forward)
            .await
            .unwrap();

        let action = handle.await.unwrap();
        assert!(matches!(action, BreakpointAction::Forward));
    }

    #[tokio::test]
    async fn test_resolve_nonexistent() {
        let mgr = make_manager();
        let result = mgr.resolve("nonexistent", BreakpointAction::Forward).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_pending_empty() {
        let mgr = make_manager();
        assert!(mgr.list_pending().await.is_empty());
    }

    #[tokio::test]
    async fn test_disabled_rule_does_not_match() {
        let mgr = make_manager();
        let mut rule = make_rule("*example.com*", true, true);
        rule.enabled = false;
        mgr.update_rules(vec![rule]).await;

        assert!(
            !mgr.should_break("https://example.com/api", &BreakpointPhase::Request)
                .await
        );
        assert!(
            !mgr.should_break("https://example.com/api", &BreakpointPhase::Response)
                .await
        );
    }

    #[tokio::test]
    async fn test_break_on_request_false_no_request_trigger() {
        let mgr = make_manager();
        // break_on_request=false, break_on_response=true
        mgr.update_rules(vec![make_rule("*example.com*", false, true)])
            .await;
        assert!(
            !mgr.should_break("https://example.com/api", &BreakpointPhase::Request)
                .await
        );
    }

    #[tokio::test]
    async fn test_break_on_response_false_no_response_trigger() {
        let mgr = make_manager();
        // break_on_request=true, break_on_response=false
        mgr.update_rules(vec![make_rule("*example.com*", true, false)])
            .await;
        assert!(
            !mgr.should_break("https://example.com/api", &BreakpointPhase::Response)
                .await
        );
    }

    #[tokio::test]
    async fn test_first_matching_rule_used() {
        let mgr = make_manager();
        // 첫 번째 규칙: request만, 두 번째 규칙: response만
        let rule1 = BreakpointRule {
            id: "rule1".to_string(),
            pattern: "*example.com*".to_string(),
            break_on_request: true,
            break_on_response: false,
            enabled: true,
        };
        let rule2 = BreakpointRule {
            id: "rule2".to_string(),
            pattern: "*example.com*".to_string(),
            break_on_request: false,
            break_on_response: true,
            enabled: true,
        };
        mgr.update_rules(vec![rule1, rule2]).await;

        // should_break은 any()를 사용하므로 둘 다 매칭됨을 확인
        // 하지만 첫 번째 규칙이 request에 매칭되면 true
        assert!(
            mgr.should_break("https://example.com/api", &BreakpointPhase::Request)
                .await
        );
        // 두 번째 규칙이 response에 매칭되면 true
        assert!(
            mgr.should_break("https://example.com/api", &BreakpointPhase::Response)
                .await
        );
    }

    #[tokio::test]
    async fn test_update_rules_replaces_previous() {
        let mgr = make_manager();
        mgr.update_rules(vec![make_rule("*old.com*", true, true)])
            .await;
        assert!(
            mgr.should_break("https://old.com/api", &BreakpointPhase::Request)
                .await
        );

        // 새 규칙으로 업데이트 → 이전 규칙 사라짐
        mgr.update_rules(vec![make_rule("*new.com*", true, true)])
            .await;
        assert!(
            !mgr.should_break("https://old.com/api", &BreakpointPhase::Request)
                .await
        );
        assert!(
            mgr.should_break("https://new.com/api", &BreakpointPhase::Request)
                .await
        );
    }

    #[tokio::test]
    async fn test_resolve_nonexistent_returns_error() {
        let mgr = make_manager();
        let result = mgr
            .resolve("does_not_exist", BreakpointAction::Forward)
            .await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("does_not_exist"));
    }
}
