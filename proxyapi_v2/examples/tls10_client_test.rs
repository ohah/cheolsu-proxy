use std::io::{Read, Write};
use std::net::TcpStream;

/// TLS 1.0 ClientHello 메시지를 생성합니다
fn create_tls10_client_hello() -> Vec<u8> {
    // TLS 1.0 ClientHello 메시지 (간단한 버전)
    vec![
        // TLS Record Header
        0x16, // ContentType: Handshake (22)
        0x03, 0x01, // ProtocolVersion: TLS 1.0 (0x0301)
        0x00, 0x2f, // Length: 47 bytes
        // Handshake Header
        0x01, // HandshakeType: ClientHello (1)
        0x00, 0x00, 0x2b, // Length: 43 bytes
        // ClientHello
        0x03, 0x01, // ClientVersion: TLS 1.0 (0x0301)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, // Random (32 bytes)
        0x00, // SessionID Length: 0
        0x00, 0x02, // CipherSuites Length: 2
        0x00, 0x2f, // CipherSuite: TLS_RSA_WITH_AES_128_CBC_SHA
        0x01, // CompressionMethods Length: 1
        0x00, // CompressionMethod: null
        0x00, 0x00, // Extensions Length: 0
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 TLS 1.0 클라이언트 시뮬레이션 시작");

    // 프록시에 연결
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    println!("✅ 프록시 연결 성공: 127.0.0.1:8080");

    // CONNECT 요청 전송
    let connect_request =
        "CONNECT gateway.icloud.com:443 HTTP/1.1\r\nHost: gateway.icloud.com:443\r\n\r\n";
    stream.write_all(connect_request.as_bytes())?;
    println!("📤 CONNECT 요청 전송");

    // CONNECT 응답 읽기
    let mut response = [0; 1024];
    let bytes_read = stream.read(&mut response)?;
    let response_str = String::from_utf8_lossy(&response[..bytes_read]);
    println!("📥 CONNECT 응답: {}", response_str);

    if response_str.contains("200 OK") {
        println!("✅ CONNECT 터널 설정 성공");

        // TLS 1.0 ClientHello 전송
        let client_hello = create_tls10_client_hello();
        stream.write_all(&client_hello)?;
        println!("📤 TLS 1.0 ClientHello 전송 ({} bytes)", client_hello.len());

        // 서버 응답 읽기
        let mut tls_response = [0; 1024];
        match stream.read(&mut tls_response) {
            Ok(bytes_read) => {
                println!("📥 TLS 응답 수신 ({} bytes)", bytes_read);
                if bytes_read > 0 {
                    println!(
                        "📦 응답 데이터 (처음 32 bytes): {:02x?}",
                        &tls_response[..std::cmp::min(32, bytes_read)]
                    );
                }
            }
            Err(e) => {
                println!("❌ TLS 응답 읽기 실패: {}", e);
            }
        }
    } else {
        println!("❌ CONNECT 터널 설정 실패");
    }

    println!("🏁 테스트 완료");
    Ok(())
}
