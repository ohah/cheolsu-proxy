/// 로컬 네트워크 IP 주소 목록을 반환합니다.
///
/// UDP 소켓을 통해 기본 네트워크 인터페이스의 IP를 조회하고,
/// ifconfig 명령어로 추가 IP를 수집합니다.
pub fn get_local_ips() -> Vec<String> {
    let mut ips = Vec::new();

    // 소켓을 통해 기본 네트워크 인터페이스의 IP를 알아냄
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        // 실제로 연결하지는 않고, 라우팅 테이블을 참조해 로컬 IP를 얻음
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                let ip = local_addr.ip().to_string();
                if ip != "0.0.0.0" && !ips.contains(&ip) {
                    ips.push(ip);
                }
            }
        }
    }

    // ifconfig 명령어로 추가 IP 수집 (macOS/Linux)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_local_ips_returns_non_empty() {
        let ips = get_local_ips();
        // 네트워크가 있는 환경에서는 최소 1개 IP가 있어야 함
        assert!(!ips.is_empty(), "로컬 IP가 하나도 없음");
    }

    #[test]
    fn get_local_ips_excludes_loopback() {
        let ips = get_local_ips();
        for ip in &ips {
            assert_ne!(ip, "127.0.0.1", "루프백 주소가 포함되면 안 됨");
            assert_ne!(ip, "0.0.0.0", "0.0.0.0이 포함되면 안 됨");
        }
    }

    #[test]
    fn get_local_ips_no_duplicates() {
        let ips = get_local_ips();
        let mut seen = std::collections::HashSet::new();
        for ip in &ips {
            assert!(seen.insert(ip), "중복 IP 발견: {}", ip);
        }
    }

    #[test]
    fn get_local_ips_valid_ipv4_format() {
        let ips = get_local_ips();
        for ip in &ips {
            let parts: Vec<&str> = ip.split('.').collect();
            assert_eq!(parts.len(), 4, "IPv4 형식이 아님: {}", ip);
            for part in parts {
                let num: u8 = part
                    .parse()
                    .unwrap_or_else(|_| panic!("유효한 숫자가 아님: {}", ip));
                let _ = num; // 0-255 범위는 u8 파싱으로 보장
            }
        }
    }
}
