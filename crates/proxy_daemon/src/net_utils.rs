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
