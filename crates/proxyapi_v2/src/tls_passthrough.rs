use dashmap::DashMap;
use dashmap::DashSet;
use http::uri::Authority;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// 자동 TLS passthrough가 활성화되기 위해 필요한 최소 연속 실패 횟수.
pub const MIN_FAILURES_FOR_BYPASS: u32 = 2;

/// 자동 TLS passthrough 실패 기록 유효 시간. 만료된 기록은 우회에 사용하지 않습니다.
pub const FAILURE_TTL_SECS: u64 = 60 * 60;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FailureRecord {
    failure_count: u32,
    last_failure_unix_secs: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum StoredFailureRecord {
    Count(u32),
    Record(FailureRecord),
}

/// TLS passthrough 실패/우회 상태 스냅샷.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsPassthroughSnapshot {
    pub host: String,
    pub failure_count: u32,
    pub active: bool,
    pub blocked_by_never_passthrough: bool,
    pub last_failure_unix_secs: u64,
    pub expires_at_unix_secs: Option<u64>,
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 와일드카드 도메인 패턴 매칭 (* = 임의 문자열, ? = 단일 문자)
/// 대소문자 구분 없음
fn domain_wildcard_matches(pattern: &str, host: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let host = host.to_lowercase();
    wildcard_match_recursive(pattern.as_bytes(), host.as_bytes())
}

fn wildcard_match_recursive(pattern: &[u8], text: &[u8]) -> bool {
    match (pattern.first(), text.first()) {
        (None, None) => true,
        (Some(b'*'), _) => {
            // '*' matches zero or more characters
            wildcard_match_recursive(&pattern[1..], text)
                || (!text.is_empty() && wildcard_match_recursive(pattern, &text[1..]))
        }
        (Some(b'?'), Some(_)) => wildcard_match_recursive(&pattern[1..], &text[1..]),
        (Some(a), Some(b)) if a == b => wildcard_match_recursive(&pattern[1..], &text[1..]),
        _ => false,
    }
}

/// TLS 핸드셰이크 실패 도메인을 기록하고, 이후 연결 시 자동으로 바이패스(터널)하는 모듈.
/// 일정 횟수 이상 연속 실패한 도메인은 제한된 시간 동안 MITM 없이 TCP 파이프로 통과시킵니다.
#[derive(Clone)]
pub struct TlsPassthrough {
    /// host → 실패 기록 (lock-free concurrent map)
    failures: Arc<DashMap<String, FailureRecord>>,
    /// 저장 파일 경로
    file_path: Option<PathBuf>,
    /// 변경 사항 알림 채널 (실시간 UI 업데이트용)
    change_tx: Option<tokio::sync::mpsc::Sender<Vec<TlsPassthroughSnapshot>>>,

    /// 절대 패스스루하지 않는 도메인 패턴 목록 (lock-free concurrent set)
    never_passthrough: Arc<DashSet<String>>,
    /// never_passthrough 저장 파일 경로
    never_passthrough_file: Option<PathBuf>,
    /// never_passthrough 변경 알림 채널
    never_passthrough_change_tx: Option<tokio::sync::mpsc::Sender<Vec<String>>>,
    /// failures 파일 저장이 필요한지 (dirty flag)
    failures_dirty: Arc<AtomicBool>,
    /// never_passthrough 파일 저장이 필요한지 (dirty flag)
    never_passthrough_dirty: Arc<AtomicBool>,
}

impl TlsPassthrough {
    pub fn new(file_path: Option<PathBuf>) -> Self {
        let failures = DashMap::new();

        // 파일에서 이전 기록 로드
        if let Some(ref path) = file_path {
            if path.exists() {
                if let Ok(data) = std::fs::read_to_string(path) {
                    if let Ok(loaded) =
                        serde_json::from_str::<HashMap<String, StoredFailureRecord>>(&data)
                    {
                        info!(
                            "[TLS-PASSTHROUGH] 이전 기록 로드: {}개 도메인",
                            loaded.len()
                        );
                        for (k, v) in loaded {
                            let record = match v {
                                StoredFailureRecord::Count(failure_count) => FailureRecord {
                                    failure_count,
                                    last_failure_unix_secs: now_unix_secs(),
                                },
                                StoredFailureRecord::Record(record) => record,
                            };
                            failures.insert(k, record);
                        }
                    }
                }
            }
        }

        Self {
            failures: Arc::new(failures),
            file_path,
            change_tx: None,
            never_passthrough: Arc::new(DashSet::new()),
            never_passthrough_file: None,
            never_passthrough_change_tx: None,
            failures_dirty: Arc::new(AtomicBool::new(false)),
            never_passthrough_dirty: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Never Passthrough 파일 경로 설정 및 로드
    pub fn with_never_passthrough_file(mut self, path: PathBuf) -> Self {
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(loaded) = serde_json::from_str::<Vec<String>>(&data) {
                    info!(
                        "[NEVER-PASSTHROUGH] 이전 기록 로드: {}개 도메인",
                        loaded.len()
                    );
                    let set = DashSet::new();
                    for entry in loaded {
                        set.insert(entry);
                    }
                    self.never_passthrough = Arc::new(set);
                }
            }
        }
        self.never_passthrough_file = Some(path);
        self
    }

    /// 변경 알림 채널을 설정합니다.
    pub fn with_change_notifier(
        mut self,
        tx: tokio::sync::mpsc::Sender<Vec<TlsPassthroughSnapshot>>,
    ) -> Self {
        self.change_tx = Some(tx);
        self
    }

    /// Never Passthrough 변경 알림 채널 설정
    pub fn with_never_passthrough_notifier(
        mut self,
        tx: tokio::sync::mpsc::Sender<Vec<String>>,
    ) -> Self {
        self.never_passthrough_change_tx = Some(tx);
        self
    }

    /// 해당 호스트가 현재 자동 우회 대상인지 확인 (lock-free, 동기)
    pub fn has_failure(&self, host: &str) -> bool {
        self.should_bypass_host(host)
    }

    /// failures 스냅샷 생성
    fn snapshot_failures(&self) -> Vec<TlsPassthroughSnapshot> {
        let now = now_unix_secs();
        self.failures
            .iter()
            .map(|entry| self.snapshot_for(entry.key(), entry.value(), now))
            .collect()
    }

    fn snapshot_for(&self, host: &str, record: &FailureRecord, now: u64) -> TlsPassthroughSnapshot {
        let blocked_by_never_passthrough = self.is_never_passthrough(host);
        let expires_at = record.last_failure_unix_secs + FAILURE_TTL_SECS;
        let expired = now >= expires_at;
        let active = record.failure_count >= MIN_FAILURES_FOR_BYPASS
            && !blocked_by_never_passthrough
            && !expired;
        TlsPassthroughSnapshot {
            host: host.to_string(),
            failure_count: record.failure_count,
            active,
            blocked_by_never_passthrough,
            last_failure_unix_secs: record.last_failure_unix_secs,
            expires_at_unix_secs: if expired { None } else { Some(expires_at) },
        }
    }

    /// never_passthrough 스냅샷 생성
    fn snapshot_never_passthrough(&self) -> Vec<String> {
        self.never_passthrough
            .iter()
            .map(|e| e.key().clone())
            .collect()
    }

    /// 변경 사항을 알림만 전송하고 파일 저장은 dirty 마킹 (hot-path용)
    fn notify_failures_and_mark_dirty(&self) {
        self.failures_dirty.store(true, Ordering::Release);
        if let Some(ref tx) = self.change_tx {
            let _ = tx.try_send(self.snapshot_failures());
        }
    }

    /// 변경 사항을 파일 저장 + 알림 채널로 전송 (즉시 저장이 필요한 경우)
    fn persist_and_notify_failures(&self) {
        self.failures_dirty.store(true, Ordering::Release);
        let snapshot = self.flush_failures_inner();
        if let Some(ref tx) = self.change_tx {
            let _ = tx.try_send(snapshot);
        }
    }

    /// dirty 상태인 failures를 파일에 저장
    pub fn flush_failures(&self) {
        self.flush_failures_inner();
    }

    /// flush 내부 구현 — 스냅샷을 반환하여 재사용 가능
    fn flush_failures_inner(&self) -> Vec<TlsPassthroughSnapshot> {
        let was_dirty = self.failures_dirty.swap(false, Ordering::Acquire);
        let snapshot = self.snapshot_failures();
        if was_dirty {
            if let Some(ref path) = self.file_path {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let map: HashMap<String, FailureRecord> = self
                    .failures
                    .iter()
                    .map(|entry| (entry.key().clone(), entry.value().clone()))
                    .collect();
                match serde_json::to_string_pretty(&map) {
                    Ok(json) => {
                        if let Err(e) = std::fs::write(path, json) {
                            warn!("[TLS-PASSTHROUGH] 파일 저장 실패: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("[TLS-PASSTHROUGH] JSON 직렬화 실패: {}", e);
                    }
                }
            }
        }
        snapshot
    }

    /// dirty 상태인 never_passthrough를 파일에 저장
    pub fn flush_never_passthrough(&self) {
        self.flush_never_passthrough_inner();
    }

    /// flush 내부 구현 — 스냅샷을 반환하여 재사용 가능
    fn flush_never_passthrough_inner(&self) -> Vec<String> {
        let was_dirty = self.never_passthrough_dirty.swap(false, Ordering::Acquire);
        let snapshot = self.snapshot_never_passthrough();
        if was_dirty {
            if let Some(ref path) = self.never_passthrough_file {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match serde_json::to_string_pretty(&snapshot) {
                    Ok(json) => {
                        if let Err(e) = std::fs::write(path, json) {
                            warn!("[NEVER-PASSTHROUGH] 파일 저장 실패: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("[NEVER-PASSTHROUGH] JSON 직렬화 실패: {}", e);
                    }
                }
            }
        }
        snapshot
    }

    /// 모든 dirty 상태를 파일에 저장
    pub fn flush_all(&self) {
        self.flush_failures();
        self.flush_never_passthrough();
    }

    /// Never Passthrough 변경 사항을 파일 저장 + 알림 (저빈도 호출용, 즉시 저장)
    fn persist_and_notify_never_passthrough(&self) {
        self.never_passthrough_dirty.store(true, Ordering::Release);
        let snapshot = self.flush_never_passthrough_inner();
        if let Some(ref tx) = self.never_passthrough_change_tx {
            let _ = tx.try_send(snapshot);
        }
    }

    /// 해당 호스트가 never_passthrough 패턴에 매칭되는지 확인 (lock-free)
    pub fn is_never_passthrough(&self, host: &str) -> bool {
        self.never_passthrough
            .iter()
            .any(|pattern| domain_wildcard_matches(pattern.key(), host))
    }

    /// 핸드셰이크 실패 기록
    pub fn record_failure(&self, authority: &Authority) {
        let host = authority.host().to_string();

        let is_never = self.is_never_passthrough(&host);
        let now = now_unix_secs();

        let count = {
            let mut entry = self.failures.entry(host.clone()).or_insert(FailureRecord {
                failure_count: 0,
                last_failure_unix_secs: now,
            });
            if now.saturating_sub(entry.last_failure_unix_secs) > FAILURE_TTL_SECS {
                entry.failure_count = 0;
            }
            entry.failure_count += 1;
            entry.last_failure_unix_secs = now;
            entry.failure_count
        };

        if is_never {
            warn!(
                "[TLS-PASSTHROUGH] 핸드셰이크 실패 기록: {} ({}회) — never_passthrough 설정으로 바이패스 안 함",
                host, count
            );
        } else {
            warn!(
                "[TLS-PASSTHROUGH] 핸드셰이크 실패 기록: {} ({}회, 자동 우회 임계값 {}회)",
                host, count, MIN_FAILURES_FOR_BYPASS
            );
        }

        self.notify_failures_and_mark_dirty();
    }

    fn should_bypass_host(&self, host: &str) -> bool {
        if self.is_never_passthrough(host) {
            debug!(
                "[TLS-PASSTHROUGH] never_passthrough 설정으로 바이패스 차단: {}",
                host
            );
            return false;
        }

        let Some(record) = self.failures.get(host) else {
            return false;
        };
        let now = now_unix_secs();
        let expires_at = record.last_failure_unix_secs + FAILURE_TTL_SECS;
        if now >= expires_at {
            debug!(
                "[TLS-PASSTHROUGH] 만료된 실패 기록으로 바이패스 미적용: {}",
                host
            );
            return false;
        }
        if record.failure_count < MIN_FAILURES_FOR_BYPASS {
            debug!(
                "[TLS-PASSTHROUGH] 실패 횟수 부족으로 바이패스 대기: {} ({}/{})",
                host, record.failure_count, MIN_FAILURES_FOR_BYPASS
            );
            return false;
        }

        debug!(
            "[TLS-PASSTHROUGH] 바이패스 적용: {} (이전 실패 {}회)",
            host, record.failure_count
        );
        true
    }

    /// 해당 도메인을 바이패스해야 하는지 확인 (임계값 이상 실패 && 만료 전 && never_passthrough 아닌 경우)
    pub fn should_bypass(&self, authority: &Authority) -> bool {
        self.should_bypass_host(authority.host())
    }

    /// 핸드셰이크 성공 시 실패 기록에서 제거
    pub fn record_success(&self, authority: &Authority) {
        let host = authority.host().to_string();
        if self.failures.remove(&host).is_some() {
            info!("[TLS-PASSTHROUGH] 성공으로 바이패스 해제: {}", host);
            self.notify_failures_and_mark_dirty();
        }
    }

    /// 현재 바이패스 목록 조회
    pub fn list_bypassed(&self) -> Vec<TlsPassthroughSnapshot> {
        self.snapshot_failures()
    }

    /// 특정 도메인 바이패스 해제
    pub fn clear_domain(&self, host: &str) {
        self.failures.remove(host);
        self.persist_and_notify_failures();
    }

    /// 전체 바이패스 기록 초기화
    pub fn clear_all(&self) {
        self.failures.clear();
        self.persist_and_notify_failures();
        info!("[TLS-PASSTHROUGH] 전체 바이패스 기록 초기화");
    }

    /// Never Passthrough 목록 설정 (전체 교체)
    pub fn set_never_passthrough(&self, entries: Vec<String>) {
        // 새 항목을 먼저 삽입한 뒤 기존 항목을 제거 (빈 상태 노출 방지)
        for entry in &entries {
            self.never_passthrough.insert(entry.clone());
        }
        let entries_set: HashSet<&str> = entries.iter().map(|s| s.as_str()).collect();
        self.never_passthrough
            .retain(|existing| entries_set.contains(existing.as_str()));
        info!(
            "[NEVER-PASSTHROUGH] 목록 업데이트: {}개 도메인",
            entries.len()
        );
        self.persist_and_notify_never_passthrough();
    }

    /// Never Passthrough 목록 조회
    pub fn list_never_passthrough(&self) -> Vec<String> {
        self.snapshot_never_passthrough()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_wildcard_matches() {
        assert!(domain_wildcard_matches("*.example.com", "sub.example.com"));
        assert!(domain_wildcard_matches("*.example.com", "a.b.example.com"));
        assert!(!domain_wildcard_matches("*.example.com", "example.com"));
        assert!(domain_wildcard_matches("example.com", "example.com"));
        assert!(domain_wildcard_matches("*", "anything.com"));
        assert!(domain_wildcard_matches("*.com", "example.com"));
        assert!(!domain_wildcard_matches("*.org", "example.com"));
        // case insensitive
        assert!(domain_wildcard_matches("*.Example.COM", "sub.example.com"));
    }

    #[test]
    fn test_record_and_bypass() {
        let passthrough = TlsPassthrough::new(None);
        let authority: Authority = "example.com:443".parse().unwrap();

        assert!(!passthrough.should_bypass(&authority));

        passthrough.record_failure(&authority);
        assert!(!passthrough.should_bypass(&authority));
        passthrough.record_failure(&authority);
        assert!(passthrough.should_bypass(&authority));
    }

    #[test]
    fn test_success_clears_bypass() {
        let passthrough = TlsPassthrough::new(None);
        let authority: Authority = "example.com:443".parse().unwrap();

        passthrough.record_failure(&authority);
        passthrough.record_failure(&authority);
        assert!(passthrough.should_bypass(&authority));

        passthrough.record_success(&authority);
        assert!(!passthrough.should_bypass(&authority));
    }

    #[test]
    fn test_different_domains_independent() {
        let passthrough = TlsPassthrough::new(None);
        let auth1: Authority = "apple.com:443".parse().unwrap();
        let auth2: Authority = "github.com:443".parse().unwrap();

        passthrough.record_failure(&auth1);
        passthrough.record_failure(&auth1);
        assert!(passthrough.should_bypass(&auth1));
        assert!(!passthrough.should_bypass(&auth2));
    }

    #[test]
    fn test_clear_all() {
        let passthrough = TlsPassthrough::new(None);
        let auth1: Authority = "apple.com:443".parse().unwrap();
        let auth2: Authority = "google.com:443".parse().unwrap();

        passthrough.record_failure(&auth1);
        passthrough.record_failure(&auth2);
        passthrough.clear_all();

        assert!(!passthrough.should_bypass(&auth1));
        assert!(!passthrough.should_bypass(&auth2));
    }

    #[test]
    fn test_file_persistence() {
        let dir = std::env::temp_dir().join("cheolsu_test_passthrough");
        let _ = std::fs::create_dir_all(&dir);
        let file_path = dir.join("test_passthrough.json");
        let _ = std::fs::remove_file(&file_path);

        // 기록
        {
            let passthrough = TlsPassthrough::new(Some(file_path.clone()));
            let authority: Authority = "pinned.example.com:443".parse().unwrap();
            passthrough.record_failure(&authority);
            passthrough.record_failure(&authority);
            passthrough.flush_all();
        }

        // 새 인스턴스에서 로드
        {
            let passthrough = TlsPassthrough::new(Some(file_path.clone()));
            let authority: Authority = "pinned.example.com:443".parse().unwrap();
            assert!(passthrough.should_bypass(&authority));
        }

        let _ = std::fs::remove_file(&file_path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_failure_count_increments() {
        let passthrough = TlsPassthrough::new(None);
        let authority: Authority = "example.com:443".parse().unwrap();

        passthrough.record_failure(&authority);
        passthrough.record_failure(&authority);
        passthrough.record_failure(&authority);

        let list = passthrough.list_bypassed();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].failure_count, 3);
        assert!(list[0].active);
    }

    #[test]
    fn test_clear_domain() {
        let passthrough = TlsPassthrough::new(None);
        let auth1: Authority = "apple.com:443".parse().unwrap();
        let auth2: Authority = "google.com:443".parse().unwrap();

        passthrough.record_failure(&auth1);
        passthrough.record_failure(&auth2);
        passthrough.record_failure(&auth2);

        passthrough.clear_domain("apple.com");

        assert!(!passthrough.should_bypass(&auth1));
        assert!(passthrough.should_bypass(&auth2));
    }

    #[test]
    fn test_list_bypassed() {
        let passthrough = TlsPassthrough::new(None);
        let auth1: Authority = "a.com:443".parse().unwrap();
        let auth2: Authority = "b.com:443".parse().unwrap();

        passthrough.record_failure(&auth1);
        passthrough.record_failure(&auth2);
        passthrough.record_failure(&auth2);

        let mut list = passthrough.list_bypassed();
        list.sort_by(|a, b| a.host.cmp(&b.host));
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].host, "a.com");
        assert_eq!(list[0].failure_count, 1);
        assert!(!list[0].active);
        assert_eq!(list[1].host, "b.com");
        assert_eq!(list[1].failure_count, 2);
        assert!(list[1].active);
    }

    #[test]
    fn test_change_notifier() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let passthrough = TlsPassthrough::new(None).with_change_notifier(tx);
        let authority: Authority = "example.com:443".parse().unwrap();

        passthrough.record_failure(&authority);

        let entries = rx.try_recv().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].host, "example.com");
        assert_eq!(entries[0].failure_count, 1);
        assert!(!entries[0].active);
    }

    #[test]
    fn test_never_passthrough_blocks_bypass() {
        let passthrough = TlsPassthrough::new(None);
        let authority: Authority = "secure.example.com:443".parse().unwrap();

        // never_passthrough에 추가
        passthrough.set_never_passthrough(vec!["*.example.com".to_string()]);

        // 실패 기록은 되지만 바이패스는 안 됨
        passthrough.record_failure(&authority);
        assert!(!passthrough.should_bypass(&authority));

        // 실패 횟수는 기록됨
        let list = passthrough.list_bypassed();
        assert_eq!(list.len(), 1);
        assert!(!list[0].active);
        assert!(list[0].blocked_by_never_passthrough);
    }

    #[test]
    fn test_never_passthrough_exact_match() {
        let passthrough = TlsPassthrough::new(None);
        let auth_blocked: Authority = "blocked.com:443".parse().unwrap();
        let auth_allowed: Authority = "other.com:443".parse().unwrap();

        passthrough.set_never_passthrough(vec!["blocked.com".to_string()]);

        passthrough.record_failure(&auth_blocked);
        passthrough.record_failure(&auth_allowed);
        passthrough.record_failure(&auth_allowed);

        assert!(!passthrough.should_bypass(&auth_blocked));
        assert!(passthrough.should_bypass(&auth_allowed));
    }

    #[test]
    fn test_never_passthrough() {
        let passthrough = TlsPassthrough::new(None);
        passthrough.set_never_passthrough(vec!["*.example.com".to_string()]);

        assert!(passthrough.is_never_passthrough("sub.example.com"));
        assert!(!passthrough.is_never_passthrough("other.com"));
    }

    #[test]
    fn test_never_passthrough_persistence() {
        let dir = std::env::temp_dir().join("cheolsu_test_never_pt");
        let _ = std::fs::create_dir_all(&dir);
        let file_path = dir.join("test_never_passthrough.json");
        let _ = std::fs::remove_file(&file_path);

        // 기록
        {
            let passthrough =
                TlsPassthrough::new(None).with_never_passthrough_file(file_path.clone());
            passthrough.set_never_passthrough(vec!["*.example.com".to_string()]);
        }

        // 새 인스턴스에서 로드
        {
            let passthrough =
                TlsPassthrough::new(None).with_never_passthrough_file(file_path.clone());
            let list = passthrough.list_never_passthrough();
            assert_eq!(list.len(), 1);
            assert!(list.contains(&"*.example.com".to_string()));
        }

        let _ = std::fs::remove_file(&file_path);
        let _ = std::fs::remove_dir(&dir);
    }
}
