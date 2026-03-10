use http::uri::Authority;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// TLS 핸드셰이크 실패 도메인을 기록하고, 이후 연결 시 자동으로 바이패스(터널)하는 모듈.
/// 한 번이라도 실패한 도메인은 이후 MITM 없이 TCP 파이프로 통과시킵니다.
#[derive(Clone)]
pub struct TlsPassthrough {
    /// host → 실패 횟수
    failures: Arc<RwLock<HashMap<String, u32>>>,
    /// 저장 파일 경로
    file_path: Option<PathBuf>,
    /// 변경 사항 알림 채널 (실시간 UI 업데이트용)
    change_tx: Option<tokio::sync::mpsc::Sender<Vec<(String, u32)>>>,
}

impl TlsPassthrough {
    pub fn new(file_path: Option<PathBuf>) -> Self {
        let mut initial = HashMap::new();

        // 파일에서 이전 기록 로드
        if let Some(ref path) = file_path {
            if path.exists() {
                if let Ok(data) = std::fs::read_to_string(path) {
                    if let Ok(loaded) = serde_json::from_str::<HashMap<String, u32>>(&data) {
                        info!(
                            "[TLS-PASSTHROUGH] 이전 기록 로드: {}개 도메인",
                            loaded.len()
                        );
                        initial = loaded;
                    }
                }
            }
        }

        Self {
            failures: Arc::new(RwLock::new(initial)),
            file_path,
            change_tx: None,
        }
    }

    /// 변경 알림 채널을 설정합니다.
    pub fn with_change_notifier(
        mut self,
        tx: tokio::sync::mpsc::Sender<Vec<(String, u32)>>,
    ) -> Self {
        self.change_tx = Some(tx);
        self
    }

    /// 내부 failures 맵에 대한 참조 (blocking context에서 사용)
    pub fn failures_ref(&self) -> &Arc<RwLock<HashMap<String, u32>>> {
        &self.failures
    }

    /// 변경 사항을 알림 채널로 전송
    fn notify_change(&self, failures: &HashMap<String, u32>) {
        if let Some(ref tx) = self.change_tx {
            let entries: Vec<(String, u32)> =
                failures.iter().map(|(k, v)| (k.clone(), *v)).collect();
            let _ = tx.try_send(entries);
        }
    }

    /// 핸드셰이크 실패 기록
    pub async fn record_failure(&self, authority: &Authority) {
        let host = authority.host().to_string();
        let mut failures = self.failures.write().await;
        let count = failures.entry(host.clone()).or_insert(0);
        *count += 1;
        warn!(
            "[TLS-PASSTHROUGH] 핸드셰이크 실패 기록: {} ({}회)",
            host, count
        );

        // 파일에 저장
        self.save_to_file(&failures);
        // 변경 알림
        self.notify_change(&failures);
    }

    /// 해당 도메인을 바이패스해야 하는지 확인 (1회 이상 실패한 경우)
    pub async fn should_bypass(&self, authority: &Authority) -> bool {
        let host = authority.host();
        let failures = self.failures.read().await;
        let bypass = failures.contains_key(host);
        if bypass {
            debug!(
                "[TLS-PASSTHROUGH] 바이패스 적용: {} (이전 실패 {}회)",
                host,
                failures.get(host).unwrap_or(&0)
            );
        }
        bypass
    }

    /// 핸드셰이크 성공 시 실패 기록에서 제거
    pub async fn record_success(&self, authority: &Authority) {
        let host = authority.host().to_string();
        let mut failures = self.failures.write().await;
        if failures.remove(&host).is_some() {
            info!("[TLS-PASSTHROUGH] 성공으로 바이패스 해제: {}", host);
            self.save_to_file(&failures);
            self.notify_change(&failures);
        }
    }

    /// 현재 바이패스 목록 조회
    pub async fn list_bypassed(&self) -> Vec<(String, u32)> {
        let failures = self.failures.read().await;
        failures.iter().map(|(k, v)| (k.clone(), *v)).collect()
    }

    /// 특정 도메인 바이패스 해제
    pub async fn clear_domain(&self, host: &str) {
        let mut failures = self.failures.write().await;
        failures.remove(host);
        self.save_to_file(&failures);
        self.notify_change(&failures);
    }

    /// 전체 바이패스 기록 초기화
    pub async fn clear_all(&self) {
        let mut failures = self.failures.write().await;
        failures.clear();
        self.save_to_file(&failures);
        self.notify_change(&failures);
        info!("[TLS-PASSTHROUGH] 전체 바이패스 기록 초기화");
    }

    fn save_to_file(&self, failures: &HashMap<String, u32>) {
        if let Some(ref path) = self.file_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match serde_json::to_string_pretty(failures) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_and_bypass() {
        let passthrough = TlsPassthrough::new(None);
        let authority: Authority = "example.com:443".parse().unwrap();

        assert!(!passthrough.should_bypass(&authority).await);

        passthrough.record_failure(&authority).await;
        assert!(passthrough.should_bypass(&authority).await);
    }

    #[tokio::test]
    async fn test_success_clears_bypass() {
        let passthrough = TlsPassthrough::new(None);
        let authority: Authority = "example.com:443".parse().unwrap();

        passthrough.record_failure(&authority).await;
        assert!(passthrough.should_bypass(&authority).await);

        passthrough.record_success(&authority).await;
        assert!(!passthrough.should_bypass(&authority).await);
    }

    #[tokio::test]
    async fn test_different_domains_independent() {
        let passthrough = TlsPassthrough::new(None);
        let auth1: Authority = "apple.com:443".parse().unwrap();
        let auth2: Authority = "github.com:443".parse().unwrap();

        passthrough.record_failure(&auth1).await;
        assert!(passthrough.should_bypass(&auth1).await);
        assert!(!passthrough.should_bypass(&auth2).await);
    }

    #[tokio::test]
    async fn test_clear_all() {
        let passthrough = TlsPassthrough::new(None);
        let auth1: Authority = "apple.com:443".parse().unwrap();
        let auth2: Authority = "google.com:443".parse().unwrap();

        passthrough.record_failure(&auth1).await;
        passthrough.record_failure(&auth2).await;
        passthrough.clear_all().await;

        assert!(!passthrough.should_bypass(&auth1).await);
        assert!(!passthrough.should_bypass(&auth2).await);
    }

    #[tokio::test]
    async fn test_file_persistence() {
        let dir = std::env::temp_dir().join("cheolsu_test_passthrough");
        let _ = std::fs::create_dir_all(&dir);
        let file_path = dir.join("test_passthrough.json");
        let _ = std::fs::remove_file(&file_path);

        // 기록
        {
            let passthrough = TlsPassthrough::new(Some(file_path.clone()));
            let authority: Authority = "pinned.example.com:443".parse().unwrap();
            passthrough.record_failure(&authority).await;
        }

        // 새 인스턴스에서 로드
        {
            let passthrough = TlsPassthrough::new(Some(file_path.clone()));
            let authority: Authority = "pinned.example.com:443".parse().unwrap();
            assert!(passthrough.should_bypass(&authority).await);
        }

        let _ = std::fs::remove_file(&file_path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn test_failure_count_increments() {
        let passthrough = TlsPassthrough::new(None);
        let authority: Authority = "example.com:443".parse().unwrap();

        passthrough.record_failure(&authority).await;
        passthrough.record_failure(&authority).await;
        passthrough.record_failure(&authority).await;

        let list = passthrough.list_bypassed().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].1, 3);
    }

    #[tokio::test]
    async fn test_clear_domain() {
        let passthrough = TlsPassthrough::new(None);
        let auth1: Authority = "apple.com:443".parse().unwrap();
        let auth2: Authority = "google.com:443".parse().unwrap();

        passthrough.record_failure(&auth1).await;
        passthrough.record_failure(&auth2).await;

        passthrough.clear_domain("apple.com").await;

        assert!(!passthrough.should_bypass(&auth1).await);
        assert!(passthrough.should_bypass(&auth2).await);
    }

    #[tokio::test]
    async fn test_list_bypassed() {
        let passthrough = TlsPassthrough::new(None);
        let auth1: Authority = "a.com:443".parse().unwrap();
        let auth2: Authority = "b.com:443".parse().unwrap();

        passthrough.record_failure(&auth1).await;
        passthrough.record_failure(&auth2).await;
        passthrough.record_failure(&auth2).await;

        let mut list = passthrough.list_bypassed().await;
        list.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], ("a.com".to_string(), 1));
        assert_eq!(list[1], ("b.com".to_string(), 2));
    }

    #[tokio::test]
    async fn test_change_notifier() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let passthrough = TlsPassthrough::new(None).with_change_notifier(tx);
        let authority: Authority = "example.com:443".parse().unwrap();

        passthrough.record_failure(&authority).await;

        let entries = rx.try_recv().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "example.com");
        assert_eq!(entries[0].1, 1);
    }
}
