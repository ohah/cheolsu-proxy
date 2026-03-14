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

/// 등록된 .proto 파일을 사용하여 gRPC 바이너리 데이터를 JSON으로 디코딩합니다.
#[tauri::command]
pub(crate) async fn decode_grpc_message(
    state: tauri::State<'_, ProtoFileStateHandle>,
    service: String,
    method: String,
    data: Vec<u8>,
    is_request: bool,
) -> Result<Option<serde_json::Value>, String> {
    let files = {
        let s = state.lock().await;
        s.files.clone()
    };

    if files.is_empty() {
        return Ok(None);
    }

    let result = tokio::task::spawn_blocking(move || {
        use proxy_daemon::proto_registry::ProtoRegistry;
        let rt = tokio::runtime::Handle::current();
        let registry = ProtoRegistry::new();

        // proto 파일 로드
        if rt.block_on(registry.load_proto_files(&files)).is_err() {
            return None;
        }

        // 메시지 디코딩
        rt.block_on(registry.decode_to_json(&service, &method, &data, is_request))
    })
    .await
    .map_err(|e| format!("gRPC 디코딩 태스크 실패: {}", e))?;

    Ok(result)
}
