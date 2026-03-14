use tauri::{AppHandle, Runtime};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct LoadSessionResult {
    pub name: Option<String>,
    pub description: Option<String>,
    pub transaction_count: usize,
    pub transactions_json: String,
}

#[tauri::command]
pub(crate) async fn export_har_file(path: String, content: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        std::fs::write(&path, content).map_err(|e| format!("HAR 파일 저장 실패: {} - {}", path, e))
    })
    .await
    .map_err(|e| format!("HAR 저장 태스크 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn save_session(path: String, transactions_json: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        use proxy_daemon::RequestInfo;
        use proxy_daemon::SessionFile;

        let file_path = proxy_daemon::ensure_extension(&path);

        let transactions: Vec<RequestInfo> = serde_json::from_str(&transactions_json)
            .map_err(|e| format!("트랜잭션 역직렬화 실패: {}", e))?;

        let session = SessionFile::from_traffic(0, &transactions, &[], &[], &[], None);
        session
            .save(std::path::Path::new(&file_path))
            .map_err(|e| format!("세션 저장 실패: {}", e))?;

        Ok(())
    })
    .await
    .map_err(|e| format!("세션 저장 태스크 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn load_session(path: String) -> Result<LoadSessionResult, String> {
    tokio::task::spawn_blocking(move || {
        use proxy_daemon::SessionFile;

        let session = SessionFile::load(std::path::Path::new(&path))
            .map_err(|e| format!("세션 로드 실패: {}", e))?;

        let transactions = session.extract_transactions();
        let transactions_json = serde_json::to_string(&transactions)
            .map_err(|e| format!("트랜잭션 직렬화 실패: {}", e))?;

        Ok(LoadSessionResult {
            name: session.metadata.name,
            description: session.metadata.description,
            transaction_count: session.transactions.len(),
            transactions_json,
        })
    })
    .await
    .map_err(|e| format!("세션 로드 태스크 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn import_har_file_cmd(path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let transactions = proxy_daemon::import_har_file(std::path::Path::new(&path))
            .map_err(|e| format!("HAR 가져오기 실패: {}", e))?;

        serde_json::to_string(&transactions).map_err(|e| format!("트랜잭션 직렬화 실패: {}", e))
    })
    .await
    .map_err(|e| format!("HAR 가져오기 태스크 실패: {}", e))?
}

pub(crate) fn build_autosave_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("autosave.cheolsu.gz")
}

pub(crate) fn get_autosave_path(
    app: &AppHandle<impl Runtime>,
) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("앱 데이터 디렉토리 조회 실패: {}", e))?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("앱 데이터 디렉토리 생성 실패: {}", e))?;
    Ok(build_autosave_path(&data_dir))
}

#[tauri::command]
pub(crate) async fn autosave_session(
    app: AppHandle<impl Runtime>,
    transactions_json: String,
) -> Result<(), String> {
    let file_path = get_autosave_path(&app)?;

    tokio::task::spawn_blocking(move || {
        use proxy_daemon::RequestInfo;
        use proxy_daemon::SessionFile;

        let transactions: Vec<RequestInfo> = serde_json::from_str(&transactions_json)
            .map_err(|e| format!("트랜잭션 역직렬화 실패: {}", e))?;

        let session = SessionFile::from_traffic(0, &transactions, &[], &[], &[], None);
        session
            .save(&file_path)
            .map_err(|e| format!("자동 세션 저장 실패: {}", e))?;

        tracing::info!("자동 세션 저장 완료: {:?}", file_path);
        Ok(())
    })
    .await
    .map_err(|e| format!("자동 세션 저장 태스크 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn autoload_session(
    app: AppHandle<impl Runtime>,
) -> Result<Option<LoadSessionResult>, String> {
    let file_path = get_autosave_path(&app)?;

    tokio::task::spawn_blocking(move || {
        use proxy_daemon::SessionFile;

        if !file_path.exists() {
            return Ok(None);
        }

        match SessionFile::load(&file_path) {
            Ok(session) => {
                let transactions = session.extract_transactions();
                let transactions_json = serde_json::to_string(&transactions)
                    .map_err(|e| format!("트랜잭션 직렬화 실패: {}", e))?;

                tracing::info!(
                    "자동 세션 복원 완료: {} 트랜잭션",
                    session.transactions.len()
                );

                Ok(Some(LoadSessionResult {
                    name: session.metadata.name,
                    description: session.metadata.description,
                    transaction_count: session.transactions.len(),
                    transactions_json,
                }))
            }
            Err(e) => {
                tracing::warn!("자동 세션 복원 실패 (무시): {}", e);
                Ok(None)
            }
        }
    })
    .await
    .map_err(|e| format!("세션 복원 태스크 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn generate_openapi_from_transactions(
    transactions_json: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        use proxy_daemon::RequestInfo;

        let transactions: Vec<RequestInfo> = serde_json::from_str(&transactions_json)
            .map_err(|e| format!("트랜잭션 역직렬화 실패: {}", e))?;

        proxy_v2_models::openapi::build_openapi_json(&transactions)
            .map_err(|e| format!("OpenAPI 스펙 생성 실패: {}", e))
    })
    .await
    .map_err(|e| format!("OpenAPI 생성 태스크 실패: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_autosave_path_creates_correct_path() {
        let data_dir = std::path::Path::new("/tmp/cheolsu-proxy");
        let path = build_autosave_path(data_dir);
        assert_eq!(
            path,
            std::path::PathBuf::from("/tmp/cheolsu-proxy/autosave.cheolsu.gz")
        );
    }

    #[test]
    fn build_autosave_path_with_various_dirs() {
        let path = build_autosave_path(std::path::Path::new("/home/user/.local/share/cheolsu"));
        assert!(path.to_string_lossy().ends_with("autosave.cheolsu.gz"));
        assert!(path.to_string_lossy().contains("cheolsu"));

        let path = build_autosave_path(std::path::Path::new("data"));
        assert_eq!(path, std::path::PathBuf::from("data/autosave.cheolsu.gz"));
    }
}
