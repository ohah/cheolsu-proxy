#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct LogFileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: u64,
}

#[tauri::command]
pub(crate) async fn get_log_files() -> Result<Vec<LogFileInfo>, String> {
    let log_dir = proxy_daemon::daemon::app_support_dir()
        .map(|d| d.join("logs"))
        .map_err(|e| format!("로그 디렉토리를 찾을 수 없습니다: {}", e))?;

    let mut files = Vec::new();

    if let Ok(parent) = proxy_daemon::daemon::app_support_dir() {
        let daemon_log = parent.join("daemon.log");
        if daemon_log.exists() {
            if let Ok(meta) = std::fs::metadata(&daemon_log) {
                files.push(LogFileInfo {
                    name: "daemon.log".to_string(),
                    path: daemon_log.display().to_string(),
                    size: meta.len(),
                    modified: meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                });
            }
        }
    }

    if log_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    if let Ok(meta) = entry.metadata() {
                        files.push(LogFileInfo {
                            name: entry.file_name().to_string_lossy().to_string(),
                            path: path.display().to_string(),
                            size: meta.len(),
                            modified: meta
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                                .unwrap_or(0),
                        });
                    }
                }
            }
        }
    }

    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(files)
}

#[tauri::command]
pub(crate) async fn read_log_file(
    path: String,
    tail_lines: Option<usize>,
) -> Result<String, String> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("로그 파일 읽기 실패: {}", e))?;

    let max_lines = tail_lines.unwrap_or(1000);
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].join("\n"))
}

#[tauri::command]
pub(crate) async fn clear_log_file(path: String) -> Result<(), String> {
    std::fs::write(&path, "").map_err(|e| format!("로그 파일 초기화 실패: {}", e))
}

#[tauri::command]
pub(crate) async fn delete_log_file(path: String) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    let app_dir = proxy_daemon::daemon::app_support_dir()
        .map_err(|e| format!("앱 디렉토리 확인 실패: {}", e))?;
    if !path.starts_with(&app_dir) {
        return Err("로그 디렉토리 외부 파일은 삭제할 수 없습니다".to_string());
    }
    std::fs::remove_file(path).map_err(|e| format!("로그 파일 삭제 실패: {}", e))
}

#[tauri::command]
pub(crate) async fn get_log_dir() -> Result<String, String> {
    proxy_daemon::daemon::app_support_dir()
        .map(|d| d.display().to_string())
        .map_err(|e| format!("로그 디렉토리를 찾을 수 없습니다: {}", e))
}
