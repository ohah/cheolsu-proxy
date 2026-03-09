use super::App;

impl App {
    /// 로컬 네트워크 IP 주소 목록을 반환합니다.
    pub(crate) fn get_local_ips() -> Vec<String> {
        let mut ips = Vec::new();

        if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(local_addr) = socket.local_addr() {
                    let ip = local_addr.ip().to_string();
                    if ip != "0.0.0.0" && !ips.contains(&ip) {
                        ips.push(ip);
                    }
                }
            }
        }

        #[cfg(unix)]
        {
            if let Ok(output) = std::process::Command::new("ifconfig").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let line = line.trim();
                    if line.starts_with("inet ") && !line.contains("127.0.0.1") {
                        if let Some(ip) = line.split_whitespace().nth(1) {
                            let ip = ip.to_string();
                            if !ips.contains(&ip) {
                                ips.push(ip);
                            }
                        }
                    }
                }
            }
        }

        ips
    }

    pub(crate) fn get_ca_storage_dir() -> Option<std::path::PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME").ok()?;
            Some(
                std::path::PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("com.cheolsu-proxy"),
            )
        }
        #[cfg(target_os = "windows")]
        {
            let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
            Some(std::path::PathBuf::from(local_app_data).join("com.cheolsu-proxy"))
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            None
        }
    }

    pub fn check_ca_status(&mut self) {
        let Some(storage_dir) = Self::get_ca_storage_dir() else {
            return;
        };
        let cer_path = storage_dir.join("cheolsu-proxy.cer");
        if cer_path.exists() {
            self.ca_cert_path = Some(cer_path.to_string_lossy().to_string());

            #[cfg(target_os = "macos")]
            {
                if let Ok(output) = std::process::Command::new("security")
                    .args(["find-certificate", "-c", "Cheolsu Proxy", "-Z"])
                    .output()
                {
                    self.ca_cert_installed = output.status.success()
                        && String::from_utf8_lossy(&output.stdout).contains("Cheolsu Proxy");
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                self.ca_cert_installed = false;
            }
        } else {
            self.ca_cert_path = None;
            self.ca_cert_installed = false;
        }
    }

    pub(crate) fn install_ca_cert(&mut self) {
        let Some(storage_dir) = Self::get_ca_storage_dir() else {
            self.set_status("CA cert: unsupported OS");
            return;
        };
        let cer_path = storage_dir.join("cheolsu-proxy.cer");
        if !cer_path.exists() {
            self.set_status("CA cert not found. Start proxy first.");
            return;
        }

        #[cfg(target_os = "macos")]
        {
            let keychain_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Library/Keychains/login.keychain-db");

            // 키체인에 인증서 추가
            let add_output = std::process::Command::new("security")
                .args(["add-certificates", "-k"])
                .arg(&keychain_path)
                .arg(&cer_path)
                .output();

            if let Ok(output) = &add_output {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.contains("-25299") && !stderr.contains("already in") {
                        self.set_status(&format!("Failed: {}", stderr.trim()));
                        return;
                    }
                }
            }

            // 신뢰 설정
            let trust_output = std::process::Command::new("security")
                .args(["add-trusted-cert", "-p", "ssl", "-k"])
                .arg(&keychain_path)
                .arg(&cer_path)
                .output();

            match trust_output {
                Ok(output) if output.status.success() => {
                    self.ca_cert_installed = true;
                    self.set_status("CA certificate installed & trusted");
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    self.set_status(&format!("Trust failed: {}", stderr.trim()));
                }
                Err(e) => {
                    self.set_status(&format!("Failed: {}", e));
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let cer_str = cer_path.to_string_lossy().to_string();
            match std::process::Command::new("certutil")
                .args(["-addstore", "-user", "Root", &cer_str])
                .output()
            {
                Ok(output) if output.status.success() => {
                    self.ca_cert_installed = true;
                    self.set_status("CA certificate installed & trusted");
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    self.set_status(&format!("Failed: {}", stderr.trim()));
                }
                Err(e) => {
                    self.set_status(&format!("Failed: {}", e));
                }
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            self.set_status("CA cert install: unsupported OS");
        }
    }

    pub(crate) fn uninstall_ca_cert(&mut self) {
        #[cfg(target_os = "macos")]
        {
            let keychain_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Library/Keychains/login.keychain-db");

            let output = std::process::Command::new("security")
                .args(["delete-certificate", "-c", "Cheolsu Proxy Root CA", "-t"])
                .arg(&keychain_path)
                .output();

            match output {
                Ok(o) if o.status.success() => {
                    self.ca_cert_installed = false;
                    self.set_status("CA certificate removed from keychain");
                }
                _ => {
                    // CN이 다를 수 있으므로 재시도
                    let output2 = std::process::Command::new("security")
                        .args(["delete-certificate", "-c", "Cheolsu Proxy", "-t"])
                        .arg(&keychain_path)
                        .output();

                    match output2 {
                        Ok(o) if o.status.success() => {
                            self.ca_cert_installed = false;
                            self.set_status("CA certificate removed from keychain");
                        }
                        _ => {
                            self.set_status("Certificate not found in keychain");
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            match std::process::Command::new("certutil")
                .args(["-delstore", "-user", "Root", "Cheolsu Proxy Root CA"])
                .output()
            {
                Ok(o) if o.status.success() => {
                    self.ca_cert_installed = false;
                    self.set_status("CA certificate removed");
                }
                _ => {
                    self.set_status("Certificate not found");
                }
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            self.set_status("CA cert uninstall: unsupported OS");
        }
    }
}
