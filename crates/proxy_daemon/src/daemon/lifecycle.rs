//! 락 파일 관리, 파일시스템 초기화, stale lock 정리 등 데몬 생명주기 관련 기능

use std::path::{Path, PathBuf};

use tracing::{debug, error, warn};

use crate::error::DaemonError;
use crate::protocol::ProxyLockInfo;

/// 앱 지원 디렉토리 경로를 반환합니다.
pub fn app_support_dir() -> Result<PathBuf, DaemonError> {
    dirs::data_dir()
        .ok_or(DaemonError::DataDirNotFound)
        .map(|dir| dir.join("com.cheolsu-proxy"))
}

/// 락 파일 경로를 반환합니다.
pub fn lock_file_path() -> Result<PathBuf, DaemonError> {
    Ok(app_support_dir()?.join("proxy.lock"))
}

/// UDS 소켓 파일 경로를 반환합니다.
pub fn uds_socket_path() -> Result<PathBuf, DaemonError> {
    Ok(app_support_dir()?.join("proxy.sock"))
}

/// 락 파일을 작성합니다.
pub(super) fn write_lock_file(port: u16, uds_path: &str) -> Result<(), DaemonError> {
    let dir = app_support_dir()?;
    std::fs::create_dir_all(&dir)?;
    let info = ProxyLockInfo {
        pid: std::process::id(),
        port,
        uds_path: uds_path.to_string(),
    };
    let json = serde_json::to_string_pretty(&info)?;
    std::fs::write(lock_file_path()?, json)?;
    Ok(())
}

/// 락 파일을 삭제합니다.
pub(super) fn remove_lock_file() {
    if let Ok(path) = lock_file_path() {
        if let Err(e) = std::fs::remove_file(&path) {
            debug!("락 파일 삭제 실패 (무시 가능): {} - {}", path.display(), e);
        }
    }
}

/// UDS 소켓 파일을 삭제합니다.
pub(super) fn remove_uds_socket(path: &Path) {
    if let Err(e) = std::fs::remove_file(path) {
        debug!(
            "UDS 소켓 파일 삭제 실패 (무시 가능): {} - {}",
            path.display(),
            e
        );
    }
}

/// Checks if a stale lock file exists and cleans it up.
/// Returns true if the lock was stale and cleaned up (or didn't exist).
pub fn check_and_cleanup_stale_lock() -> bool {
    let lock_path = match lock_file_path() {
        Ok(p) => p,
        Err(_) => return true,
    };
    if !lock_path.exists() {
        return true;
    }

    let contents = match std::fs::read_to_string(&lock_path) {
        Ok(c) => c,
        Err(_) => {
            if let Err(e) = std::fs::remove_file(&lock_path) {
                debug!("stale 락 파일 삭제 실패: {}", e);
            }
            return true;
        }
    };

    let info: ProxyLockInfo = match serde_json::from_str(&contents) {
        Ok(i) => i,
        Err(_) => {
            if let Err(e) = std::fs::remove_file(&lock_path) {
                debug!("잘못된 형식의 락 파일 삭제 실패: {}", e);
            }
            return true;
        }
    };

    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    let pid = Pid::from_raw(info.pid as i32);
    if kill(pid, None).is_err() {
        warn!(
            "Stale lock file detected (PID {} is dead), cleaning up",
            info.pid
        );
        if let Err(e) = std::fs::remove_file(&lock_path) {
            debug!("dead 프로세스 락 파일 삭제 실패: {}", e);
        }
        if let Err(e) = std::fs::remove_file(&info.uds_path) {
            debug!("dead 프로세스 UDS 소켓 삭제 실패: {}", e);
        }
        return true;
    }

    false
}

/// 파일시스템 초기화: stale lock 정리, UDS 경로 확보, lock 파일 작성.
pub(super) fn init_filesystem(port: u16) -> Result<(PathBuf, String), i32> {
    if !check_and_cleanup_stale_lock() {
        error!("Another daemon is already running. Exiting.");
        return Err(1);
    }

    let uds_path = match uds_socket_path() {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to get UDS socket path: {}", e);
            return Err(1);
        }
    };
    let uds_path_str = uds_path.to_string_lossy().to_string();

    remove_uds_socket(&uds_path);

    if let Some(parent) = uds_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            debug!(
                "UDS 소켓 상위 디렉토리 생성 실패: {} - {}",
                parent.display(),
                e
            );
        }
    }

    if let Err(e) = write_lock_file(port, &uds_path_str) {
        error!("Failed to write lock file: {}", e);
        return Err(1);
    }

    Ok((uds_path, uds_path_str))
}
