use proxy_daemon::get_local_ips;

fn get_ca_storage_dir() -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home =
            std::env::var("HOME").map_err(|_| "HOME 환경 변수를 찾을 수 없습니다".to_string())?;
        let dir = std::path::PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.cheolsu-proxy");
        std::fs::create_dir_all(&dir).map_err(|e| format!("디렉토리 생성 실패: {}", e))?;
        Ok(dir)
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "LOCALAPPDATA 환경 변수를 찾을 수 없습니다".to_string())?;
        let dir = std::path::PathBuf::from(local_app_data).join("com.cheolsu-proxy");
        std::fs::create_dir_all(&dir).map_err(|e| format!("디렉토리 생성 실패: {}", e))?;
        Ok(dir)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("현재 macOS와 Windows만 지원합니다".to_string())
    }
}

#[tauri::command]
pub(crate) fn get_ca_cert_path() -> Result<String, String> {
    let storage_dir = get_ca_storage_dir()?;
    let cer_path = storage_dir.join("cheolsu-proxy.cer");
    if cer_path.exists() {
        Ok(cer_path.to_string_lossy().to_string())
    } else {
        Err("CA 인증서가 아직 생성되지 않았습니다. 프록시를 먼저 실행해주세요.".to_string())
    }
}

#[tauri::command]
pub(crate) fn check_ca_installed() -> Result<bool, String> {
    let storage_dir = get_ca_storage_dir()?;
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    if !cer_path.exists() {
        return Ok(false);
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("security")
            .args(["find-certificate", "-c", "Cheolsu Proxy", "-Z"])
            .output()
            .map_err(|e| format!("security 명령 실행 실패: {}", e))?;

        Ok(output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("Cheolsu Proxy"))
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("certutil")
            .args(["-verifystore", "Root", "Cheolsu Proxy Root CA"])
            .output()
            .map_err(|e| format!("certutil 명령 실행 실패: {}", e))?;

        Ok(output.status.success())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Ok(false)
    }
}

#[tauri::command]
pub(crate) fn install_ca_cert() -> Result<String, String> {
    let storage_dir = get_ca_storage_dir()?;
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    if !cer_path.exists() {
        return Err(
            "CA 인증서가 아직 생성되지 않았습니다. 프록시를 먼저 실행해주세요.".to_string(),
        );
    }

    #[cfg(target_os = "macos")]
    {
        let keychain_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Library/Keychains/login.keychain-db");

        let add_output = std::process::Command::new("security")
            .args(["add-certificates", "-k"])
            .arg(&keychain_path)
            .arg(&cer_path)
            .output()
            .map_err(|e| format!("security 명령 실행 실패: {}", e))?;

        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            if !stderr.contains("-25299") && !stderr.contains("already in") {
                return Err(format!("키체인에 인증서 추가 실패: {}", stderr.trim()));
            }
        }

        let trust_output = std::process::Command::new("security")
            .args(["add-trusted-cert", "-p", "ssl", "-k"])
            .arg(&keychain_path)
            .arg(&cer_path)
            .output()
            .map_err(|e| format!("security 명령 실행 실패: {}", e))?;

        if trust_output.status.success() {
            Ok("CA 인증서가 키체인에 신뢰 인증서로 설치되었습니다.".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&trust_output.stderr);
            Err(format!("인증서 신뢰 설정 실패: {}", stderr.trim()))
        }
    }

    #[cfg(target_os = "windows")]
    {
        let cer_path_str = cer_path.to_string_lossy().to_string();
        let output = std::process::Command::new("certutil")
            .args(["-addstore", "-user", "Root", &cer_path_str])
            .output()
            .map_err(|e| format!("certutil 실행 실패: {}", e))?;

        if output.status.success() {
            Ok("CA 인증서가 신뢰할 수 있는 루트 인증 기관에 설치되었습니다.".to_string())
        } else {
            Err(format!(
                "인증서 설치 실패: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("현재 이 OS에서는 자동 설치를 지원하지 않습니다.".to_string())
    }
}

#[tauri::command]
pub(crate) fn uninstall_ca_cert() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let keychain_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Library/Keychains/login.keychain-db");

        let output = std::process::Command::new("security")
            .args(["delete-certificate", "-c", "Cheolsu Proxy Root CA", "-t"])
            .arg(&keychain_path)
            .output()
            .map_err(|e| format!("security 명령 실행 실패: {}", e))?;

        if output.status.success() {
            Ok("CA 인증서가 키체인에서 제거되었습니다.".to_string())
        } else {
            let output2 = std::process::Command::new("security")
                .args(["delete-certificate", "-c", "Cheolsu Proxy", "-t"])
                .arg(&keychain_path)
                .output()
                .map_err(|e| format!("security 명령 실행 실패: {}", e))?;

            if output2.status.success() {
                Ok("CA 인증서가 키체인에서 제거되었습니다.".to_string())
            } else {
                Err("키체인에서 인증서를 찾을 수 없습니다.".to_string())
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("certutil")
            .args(["-delstore", "-user", "Root", "Cheolsu Proxy Root CA"])
            .output()
            .map_err(|e| format!("certutil 실행 실패: {}", e))?;

        if output.status.success() {
            Ok("CA 인증서가 제거되었습니다.".to_string())
        } else {
            Err(format!(
                "인증서 제거 실패: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("현재 이 OS에서는 자동 제거를 지원하지 않습니다.".to_string())
    }
}

#[tauri::command]
pub(crate) async fn parse_certificate_info(
    cert_path: String,
) -> Result<proxy_daemon::CertificateInfo, String> {
    tokio::task::spawn_blocking(move || {
        proxy_daemon::parse_certificate_info(&cert_path)
            .map_err(|e| format!("인증서 파싱 실패: {}", e))
    })
    .await
    .map_err(|e| format!("파싱 태스크 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn import_custom_ca(
    cert_path: String,
    key_path: String,
) -> Result<proxy_daemon::CertificateInfo, String> {
    tokio::task::spawn_blocking(move || {
        let cert_path = std::fs::canonicalize(&cert_path)
            .map_err(|e| format!("인증서 경로 확인 실패: {}", e))?;
        let key_path =
            std::fs::canonicalize(&key_path).map_err(|e| format!("키 경로 확인 실패: {}", e))?;
        let cert_path = cert_path.to_string_lossy().to_string();
        let key_path = key_path.to_string_lossy().to_string();

        let info = proxy_daemon::validate_ca_certificate(&cert_path)
            .map_err(|e| format!("CA 인증서 검증 실패: {}", e))?;

        let cert_data =
            std::fs::read(&cert_path).map_err(|e| format!("인증서 파일 읽기 실패: {}", e))?;
        let key_data = std::fs::read(&key_path).map_err(|e| format!("키 파일 읽기 실패: {}", e))?;

        if !key_data.starts_with(b"-----BEGIN") {
            return Err("키 파일이 PEM 형식이 아닙니다".to_string());
        }

        proxy_daemon::validate_cert_key_pair(&cert_data, &key_data)
            .map_err(|e| format!("인증서-키 검증 실패: {}", e))?;

        proxy_daemon::save_custom_ca(&cert_data, &key_data)
            .map_err(|e| format!("커스텀 CA 저장 실패: {}", e))?;

        Ok(info)
    })
    .await
    .map_err(|e| format!("태스크 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn import_custom_ca_pkcs12(
    p12_path: String,
    password: String,
) -> Result<proxy_daemon::CertificateInfo, String> {
    tokio::task::spawn_blocking(move || {
        let (cert_pem, key_pem) = proxy_daemon::parse_pkcs12(&p12_path, &password)
            .map_err(|e| format!("PKCS12 파싱 실패: {}", e))?;

        let info = proxy_daemon::validate_ca_certificate_from_bytes(&cert_pem)
            .map_err(|e| format!("CA 인증서 검증 실패: {}", e))?;

        proxy_daemon::save_custom_ca(&cert_pem, &key_pem)
            .map_err(|e| format!("커스텀 CA 저장 실패: {}", e))?;

        Ok(info)
    })
    .await
    .map_err(|e| format!("태스크 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn remove_custom_ca() -> Result<(), String> {
    tokio::task::spawn_blocking(|| {
        proxy_daemon::remove_custom_ca().map_err(|e| format!("커스텀 CA 제거 실패: {}", e))
    })
    .await
    .map_err(|e| format!("태스크 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn get_custom_ca_status() -> Result<Option<proxy_daemon::CertificateInfo>, String>
{
    tokio::task::spawn_blocking(|| {
        proxy_daemon::get_custom_ca_info().map_err(|e| format!("커스텀 CA 상태 확인 실패: {}", e))
    })
    .await
    .map_err(|e| format!("태스크 실패: {}", e))?
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct CertDownloadInfo {
    pub port: u16,
    pub local_ips: Vec<String>,
    pub download_url: String,
    pub direct_url: String,
    pub qr_code_base64: String,
}

fn generate_qr_code_base64(data: &str) -> Result<String, String> {
    use image::Luma;
    use qrcode::QrCode;

    let code = QrCode::new(data.as_bytes()).map_err(|e| format!("QR 코드 생성 실패: {}", e))?;

    let image = code.render::<Luma<u8>>().quiet_zone(true).build();

    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("PNG 인코딩 실패: {}", e))?;

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}

#[tauri::command]
pub(crate) fn get_cert_download_info(port: u16) -> Result<CertDownloadInfo, String> {
    let local_ips = get_local_ips();
    let download_url = "http://cheolsu.proxy/ssl".to_string();

    let primary_ip = local_ips
        .first()
        .cloned()
        .unwrap_or("127.0.0.1".to_string());
    let direct_url = format!("http://cheolsu.proxy/ssl (proxy: {}:{})", primary_ip, port);

    let qr_content = format!("http://{}:{}/ssl", primary_ip, port);
    let qr_code_base64 = generate_qr_code_base64(&qr_content)?;

    Ok(CertDownloadInfo {
        port,
        local_ips,
        download_url,
        direct_url,
        qr_code_base64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_qr_code_base64_returns_valid_base64() {
        let result = generate_qr_code_base64("http://192.168.1.1:8100/ssl");
        assert!(result.is_ok(), "QR 코드 생성 실패: {:?}", result.err());

        let base64_str = result.unwrap();
        assert!(!base64_str.is_empty(), "base64 문자열이 비어있음");

        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD.decode(&base64_str);
        assert!(decoded.is_ok(), "유효한 base64가 아님");

        let bytes = decoded.unwrap();
        assert!(bytes.len() > 8, "PNG 데이터가 너무 짧음");
        assert_eq!(&bytes[..4], b"\x89PNG", "PNG 매직 바이트가 아님");
    }

    #[test]
    fn generate_qr_code_base64_contains_logo() {
        let result = generate_qr_code_base64("http://test.local/ssl");
        assert!(result.is_ok());

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(result.unwrap())
            .unwrap();
        assert!(
            bytes.len() > 1000,
            "로고가 합성된 QR코드가 너무 작음: {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn get_cert_download_info_returns_valid_info() {
        let result = get_cert_download_info(8100);
        assert!(result.is_ok(), "인증서 다운로드 정보 생성 실패");

        let info = result.unwrap();
        assert_eq!(info.port, 8100);
        assert_eq!(info.download_url, "http://cheolsu.proxy/ssl");
        assert!(!info.local_ips.is_empty(), "로컬 IP가 비어있음");
        assert!(!info.qr_code_base64.is_empty(), "QR 코드가 비어있음");
        assert!(info.direct_url.contains("cheolsu.proxy/ssl"));
    }

    #[test]
    fn cert_download_info_struct_fields() {
        let info = CertDownloadInfo {
            port: 9090,
            local_ips: vec!["192.168.1.1".to_string()],
            download_url: "http://cheolsu.proxy/ssl".to_string(),
            direct_url: "http://cheolsu.proxy/ssl (proxy: 192.168.1.1:9090)".to_string(),
            qr_code_base64: "dGVzdA==".to_string(),
        };
        assert_eq!(info.port, 9090);
        assert_eq!(info.local_ips.len(), 1);
    }
}
