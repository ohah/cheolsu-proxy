use std::fs;
use std::path::PathBuf;
use tracing::{debug, error};

/// 앱 데이터 디렉토리 경로를 반환합니다.
///
/// # Returns
/// - macOS: `~/Library/Application Support/com.cheolsu-proxy/`
/// - Windows: `%APPDATA%/com.cheolsu-proxy/`
/// - Linux: `~/.config/com.cheolsu-proxy/` (향후 구현)
pub fn get_ca_storage_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| "Could not find HOME environment variable")?;

        // 앱 identifier (고정값)
        let identifier = "com.cheolsu-proxy";

        let dir = PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(identifier);

        // 디렉토리 생성
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create directory: {}", e))?;

        Ok(dir)
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "Could not find LOCALAPPDATA environment variable")?;

        // 앱 identifier (고정값)
        let identifier = "com.cheolsu-proxy";

        let dir = PathBuf::from(local_app_data).join(identifier);

        // 디렉토리 생성
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create directory: {}", e))?;

        Ok(dir)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Currently only macOS and Windows are supported".to_string())
    }
}

/// 캐시 저장 디렉토리 경로를 반환합니다.
///
/// # Returns
/// - macOS: `~/Library/Caches/com.cheolsu-proxy/data/{session_hash}/`
/// - Windows: `%LOCALAPPDATA%/com.cheolsu-proxy/Cache/data/{session_hash}/`
/// - Linux: `~/.cache/com.cheolsu-proxy/data/{session_hash}/` (향후 구현)
pub fn get_cache_storage_dir(session_hash: &str) -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| "Could not find HOME environment variable")?;

        // 앱 identifier (고정값)
        let identifier = "com.cheolsu-proxy";

        let dir = PathBuf::from(home)
            .join("Library")
            .join("Caches")
            .join(identifier)
            .join("data")
            .join(session_hash);

        // 디렉토리 생성
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create cache directory: {}", e))?;

        Ok(dir)
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "Could not find LOCALAPPDATA environment variable")?;

        // 앱 identifier (고정값)
        let identifier = "com.cheolsu-proxy";

        let dir = PathBuf::from(local_app_data)
            .join(identifier)
            .join("Cache")
            .join("data")
            .join(session_hash);

        // 디렉토리 생성
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create cache directory: {}", e))?;

        Ok(dir)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Currently only macOS and Windows are supported".to_string())
    }
}

/// 세션 해시를 생성합니다.
/// 타임스탬프 + UUID 일부를 사용하여 고유성 보장
pub fn generate_session_hash() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let uuid_part = uuid::Uuid::new_v4().to_string().replace('-', "");

    format!("{}_{}", timestamp, &uuid_part[..8])
}

/// 앱 종료 시 현재 세션의 캐시만 삭제
pub fn clean_cache_on_exit(session_hash: &str) -> Result<(), String> {
    let cache_dir = get_cache_storage_dir(session_hash)?;

    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to remove session cache directory: {}", e))?;
        debug!(path = %cache_dir.display(), "세션 캐시 정리 완료");
    }

    Ok(())
}

/// 지정된 일수보다 오래된 캐시 폴더 삭제
pub fn clean_old_cache(days: u64) -> Result<(), String> {
    let base_cache_dir = get_base_cache_dir()?;

    if !base_cache_dir.exists() {
        return Ok(());
    }

    let cutoff_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        - (days * 24 * 60 * 60);

    let mut cleaned_count = 0;

    if let Ok(entries) = fs::read_dir(&base_cache_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(modified_secs) = modified.duration_since(std::time::UNIX_EPOCH) {
                        if modified_secs.as_secs() < cutoff_time {
                            if let Err(e) = fs::remove_dir_all(entry.path()) {
                                error!(path = %entry.path().display(), error = %e, "오래된 캐시 삭제 실패");
                            } else {
                                cleaned_count += 1;
                                debug!(path = %entry.path().display(), "오래된 캐시 삭제");
                            }
                        }
                    }
                }
            }
        }
    }

    debug!(cleaned_count, "오래된 캐시 정리 완료");
    Ok(())
}

/// 모든 캐시 데이터 삭제 (수동 정리용)
pub fn clean_all_cache() -> Result<(), String> {
    let base_cache_dir = get_base_cache_dir()?;

    if !base_cache_dir.exists() {
        debug!(path = %base_cache_dir.display(), "캐시 디렉토리가 존재하지 않습니다");
        return Ok(());
    }

    fs::remove_dir_all(&base_cache_dir)
        .map_err(|e| format!("Failed to remove all cache directories: {}", e))?;

    debug!(path = %base_cache_dir.display(), "모든 캐시 정리 완료");
    Ok(())
}

/// 기본 캐시 디렉토리 경로를 반환합니다 (세션 해시 제외)
pub fn get_base_cache_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| "Could not find HOME environment variable")?;
        let identifier = "com.cheolsu-proxy";

        Ok(PathBuf::from(home)
            .join("Library")
            .join("Caches")
            .join(identifier)
            .join("data"))
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "Could not find LOCALAPPDATA environment variable")?;
        let identifier = "com.cheolsu-proxy";

        Ok(PathBuf::from(local_app_data)
            .join(identifier)
            .join("Cache")
            .join("data"))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Currently only macOS and Windows are supported".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_session_hash_is_unique() {
        let hash1 = generate_session_hash();
        let hash2 = generate_session_hash();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn generate_session_hash_format() {
        let hash = generate_session_hash();
        // format: {timestamp}_{uuid_8chars}
        assert!(hash.contains('_'));
        let parts: Vec<&str> = hash.split('_').collect();
        assert_eq!(parts.len(), 2);
        // timestamp part should be numeric
        assert!(parts[0].parse::<u128>().is_ok());
        // uuid part should be 8 hex chars
        assert_eq!(parts[1].len(), 8);
    }

    #[test]
    fn get_ca_storage_dir_returns_valid_path() {
        let dir = get_ca_storage_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.exists());
        assert!(path.to_str().unwrap().contains("com.cheolsu-proxy"));
    }

    #[test]
    fn get_cache_storage_dir_creates_directory() {
        let session = generate_session_hash();
        let dir = get_cache_storage_dir(&session);
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.exists());
        assert!(path.to_str().unwrap().contains(&session));
        // cleanup
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn get_base_cache_dir_returns_valid_path() {
        let dir = get_base_cache_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.to_str().unwrap().contains("com.cheolsu-proxy"));
        assert!(path.to_str().unwrap().contains("data"));
    }

    #[test]
    fn clean_cache_on_exit_removes_session() {
        let session = generate_session_hash();
        let dir = get_cache_storage_dir(&session).unwrap();
        assert!(dir.exists());

        let result = clean_cache_on_exit(&session);
        assert!(result.is_ok());
        assert!(!dir.exists());
    }

    #[test]
    fn clean_cache_on_exit_nonexistent_is_noop() {
        // clean_cache_on_exit creates the dir via get_cache_storage_dir, then removes it
        // so it should always succeed
        let session = generate_session_hash();
        let result = clean_cache_on_exit(&session);
        assert!(result.is_ok());
    }
}
