/// TLS 1.0/1.1 하이브리드 핸들러 테스트
///
/// 이 예제는 TLS 버전을 감지하고 적절한 핸들러를 선택하는 기능을 보여줍니다.
use proxyapi_v2::{
    certificate_authority::build_ca, hybrid_tls_handler::HybridTlsHandler,
    tls_version_detector::TlsVersionDetector,
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // 로깅 초기화
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 TLS 하이브리드 핸들러 테스트");
    println!("================================\n");

    // CA 생성
    let ca = match build_ca() {
        Ok(ca) => {
            println!("✅ CA 인증서 로드 성공");
            Arc::new(ca)
        }
        Err(e) => {
            eprintln!("❌ CA 인증서 로드 실패: {}", e);
            return;
        }
    };

    // 하이브리드 핸들러 생성
    match HybridTlsHandler::new(Arc::clone(&ca)).await {
        Ok(_handler) => {
            println!("✅ HybridTlsHandler 초기화 성공\n");
        }
        Err(e) => {
            eprintln!("❌ HybridTlsHandler 초기화 실패: {}", e);
            return;
        }
    }

    // TLS 버전 감지 테스트
    println!("📋 TLS 버전 감지 테스트:");
    println!("--------------------------");

    test_tls_version_detection(b"\x16\x03\x00\x03\x01", "TLS 1.0");
    test_tls_version_detection(b"\x16\x03\x00\x03\x02", "TLS 1.1");
    test_tls_version_detection(b"\x16\x03\x00\x03\x03", "TLS 1.2");
    test_tls_version_detection(b"\x16\x03\x00\x03\x04", "TLS 1.3");
    test_tls_version_detection(b"\x16\x03\x00\x03\x05", "알 수 없는 버전");

    println!("\n✅ 모든 테스트 완료!");
    println!("\n💡 프록시 서버를 시작하려면 다음 명령어를 사용하세요:");
    println!(
        "   cargo run --example hybrid_tls_proxy --features native-tls-client,rcgen-ca,openssl-ca"
    );
}

fn test_tls_version_detection(buffer: &[u8], description: &str) {
    match TlsVersionDetector::detect_tls_version(buffer) {
        Some(version) => {
            let rustls_support = if TlsVersionDetector::is_rustls_supported(version) {
                "rustls"
            } else {
                "native-tls"
            };

            println!(
                "  {} → {:?} ({})",
                description,
                version.as_str(),
                rustls_support
            );
        }
        None => {
            println!("  {} → 감지 실패", description);
        }
    }
}
