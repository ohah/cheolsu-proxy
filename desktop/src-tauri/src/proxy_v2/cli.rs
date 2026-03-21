use tauri::{AppHandle, Runtime};

fn install_sidecar_binary(
    app: &AppHandle<impl Runtime>,
    sidecar_base: &str,
    dest_name: &str,
) -> Result<String, String> {
    use tauri::Manager;

    let current_exe =
        std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| "실행 파일의 부모 디렉토리를 찾을 수 없습니다".to_string())?;

    if cfg!(dev) {
        let bin_path = exe_dir.join(dest_name);
        if bin_path.exists() {
            return Ok(bin_path.display().to_string());
        }
    }

    let source = exe_dir.join(sidecar_base);

    if !source.exists() {
        return Err(format!(
            "Sidecar 바이너리가 존재하지 않습니다: {}",
            source.display()
        ));
    }

    let home = app
        .path()
        .home_dir()
        .map_err(|e| format!("홈 디렉토리를 찾을 수 없습니다: {}", e))?;
    let bin_dir = home.join(".cheolsu").join("bin");
    std::fs::create_dir_all(&bin_dir).map_err(|e| format!("디렉토리 생성 실패: {}", e))?;

    let dest = bin_dir.join(dest_name);

    let needs_copy = if dest.exists() {
        let src_meta = std::fs::metadata(&source).ok();
        let dst_meta = std::fs::metadata(&dest).ok();
        match (src_meta, dst_meta) {
            (Some(s), Some(d)) => s.len() != d.len(),
            _ => true,
        }
    } else {
        true
    };

    if needs_copy {
        std::fs::copy(&source, &dest).map_err(|e| format!("바이너리 복사 실패: {}", e))?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("실행 권한 설정 실패: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("xattr")
            .args(["-cr", &dest.display().to_string()])
            .output();
    }

    Ok(dest.display().to_string())
}

#[tauri::command]
pub(crate) fn get_mcp_server_path(app: AppHandle<impl Runtime>) -> Result<String, String> {
    let path = install_sidecar_binary(&app, "cheolsu-proxy-mcp", "cheolsu-proxy-mcp")?;

    #[cfg(target_os = "macos")]
    install_frameworks(&app)?;

    Ok(path)
}

#[cfg(target_os = "macos")]
fn run_with_admin_privileges(shell_cmd: &str) -> Result<(), String> {
    let script = format!(
        r#"do shell script "{}" with administrator privileges"#,
        shell_cmd.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("osascript 실행 실패: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("User canceled") || stderr.contains("-128") {
            Err("사용자가 취소했습니다".to_string())
        } else {
            Err(format!("관리자 권한 명령 실패: {}", stderr.trim()))
        }
    }
}

#[cfg(target_os = "macos")]
fn install_frameworks(app: &AppHandle<impl Runtime>) -> Result<(), String> {
    use tauri::Manager;

    let current_exe =
        std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| "실행 파일의 부모 디렉토리를 찾을 수 없습니다".to_string())?;

    // 앱 번들의 Frameworks 디렉토리: Contents/MacOS/../Frameworks
    let frameworks_src = exe_dir.join("../Frameworks");

    let home = app
        .path()
        .home_dir()
        .map_err(|e| format!("홈 디렉토리를 찾을 수 없습니다: {}", e))?;
    let frameworks_dest = home.join(".cheolsu").join("Frameworks");
    std::fs::create_dir_all(&frameworks_dest)
        .map_err(|e| format!("Frameworks 디렉토리 생성 실패: {}", e))?;

    for dylib_name in &["libssl.3.dylib", "libcrypto.3.dylib"] {
        let src = frameworks_src.join(dylib_name);
        let dest = frameworks_dest.join(dylib_name);

        if !src.exists() {
            continue;
        }

        let needs_copy = if dest.exists() {
            let src_meta = std::fs::metadata(&src).ok();
            let dst_meta = std::fs::metadata(&dest).ok();
            match (src_meta, dst_meta) {
                (Some(s), Some(d)) => s.len() != d.len(),
                _ => true,
            }
        } else {
            true
        };

        if needs_copy {
            std::fs::copy(&src, &dest).map_err(|e| format!("{} 복사 실패: {}", dylib_name, e))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub(crate) async fn install_cli(app: AppHandle<impl Runtime>) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let tui_path = install_sidecar_binary(&app, "cheolsu-tui", "cheolsu-tui")?;

        #[cfg(target_os = "macos")]
        install_frameworks(&app)?;

        let link_path = "/usr/local/bin/cheolsu";
        let link = std::path::Path::new(link_path);

        let needs_admin = if link.exists() || link.is_symlink() {
            std::fs::remove_file(link).is_err()
        } else {
            false
        };

        #[cfg(unix)]
        {
            if needs_admin || std::os::unix::fs::symlink(&tui_path, link).is_err() {
                #[cfg(target_os = "macos")]
                {
                    let cmd = format!("rm -f {} && ln -sf {} {}", link_path, tui_path, link_path);
                    run_with_admin_privileges(&cmd)?;
                }

                #[cfg(not(target_os = "macos"))]
                {
                    return Err("심볼릭 링크 생성 실패: sudo 권한이 필요합니다".to_string());
                }
            }
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(&tui_path, link)
                .map_err(|e| format!("심볼릭 링크 생성 실패: {}", e))?;
        }

        Ok(format!(
            "터미널 명령어가 설치되었습니다: {} -> {}",
            link_path, tui_path
        ))
    })
    .await
    .map_err(|e| format!("작업 실행 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn uninstall_cli() -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let link_path = "/usr/local/bin/cheolsu";
        let link = std::path::Path::new(link_path);

        if !link.exists() && !link.is_symlink() {
            return Err("터미널 명령어가 설치되어 있지 않습니다".to_string());
        }

        if std::fs::remove_file(link).is_err() {
            #[cfg(target_os = "macos")]
            {
                let cmd = format!("rm -f {}", link_path);
                run_with_admin_privileges(&cmd)?;
            }

            #[cfg(not(target_os = "macos"))]
            {
                return Err("제거 실패: sudo 권한이 필요합니다".to_string());
            }
        }

        Ok("터미널 명령어가 제거되었습니다".to_string())
    })
    .await
    .map_err(|e| format!("작업 실행 실패: {}", e))?
}

#[tauri::command]
pub(crate) fn check_cli_installed() -> bool {
    let link_path = std::path::Path::new("/usr/local/bin/cheolsu");
    link_path.exists()
}
