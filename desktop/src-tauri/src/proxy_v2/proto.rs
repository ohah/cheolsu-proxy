use std::sync::Arc;
use tokio::sync::Mutex;

/// Proto 파일 경로만 관리하는 간단한 상태
/// ProtoRegistry 자체는 Send가 아니므로 spawn_blocking에서 생성/사용
pub(crate) struct ProtoFileState {
    pub(crate) files: Vec<String>,
}

pub(crate) type ProtoFileStateHandle = Arc<Mutex<ProtoFileState>>;

pub(crate) fn create_proto_file_state() -> ProtoFileStateHandle {
    Arc::new(Mutex::new(ProtoFileState { files: Vec::new() }))
}

#[tauri::command]
pub(crate) async fn load_proto_files(
    state: tauri::State<'_, ProtoFileStateHandle>,
    paths: Vec<String>,
) -> Result<usize, String> {
    // protox Compiler는 Send가 아니므로 spawn_blocking 사용
    let paths_clone = paths.clone();
    let count = tokio::task::spawn_blocking(move || {
        use proxy_daemon::proto_registry::ProtoRegistry;
        let rt = tokio::runtime::Handle::current();
        let registry = ProtoRegistry::new();
        rt.block_on(registry.load_proto_files(&paths_clone))
    })
    .await
    .map_err(|e| format!("Proto 로드 태스크 실패: {}", e))?
    .map_err(|e| format!("Proto 로드 실패: {}", e))?;

    let mut s = state.lock().await;
    for path in paths {
        if !s.files.contains(&path) {
            s.files.push(path);
        }
    }

    Ok(count)
}

#[tauri::command]
pub(crate) async fn list_proto_files(
    state: tauri::State<'_, ProtoFileStateHandle>,
) -> Result<Vec<String>, String> {
    let s = state.lock().await;
    Ok(s.files.clone())
}

#[tauri::command]
pub(crate) async fn remove_proto_file(
    state: tauri::State<'_, ProtoFileStateHandle>,
    path: String,
) -> Result<(), String> {
    let mut s = state.lock().await;
    s.files.retain(|p| p != &path);
    Ok(())
}
