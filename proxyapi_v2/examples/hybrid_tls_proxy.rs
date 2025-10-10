use proxyapi_v2::{
    certificate_authority::RcgenAuthority, 
    TlsVersionDetector,
    NoopHandler, Proxy,
};
use std::net::SocketAddr;
use tokio::signal;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 로깅 설정
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🚀 하이브리드 TLS 프록시 시작");

    // CA 인증서 동적 생성
    let key_pair = rcgen::KeyPair::generate().expect("Failed to generate key pair");
    let mut ca_cert = rcgen::CertificateParams::new(vec!["localhost".to_string()])
        .expect("Failed to create certificate params");
    ca_cert.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let ca_cert = ca_cert.self_signed(&key_pair).expect("Failed to sign CA certificate");

    let ca = RcgenAuthority::new(key_pair, ca_cert, 1000, tokio_rustls::rustls::crypto::aws_lc_rs::default_provider());
    info!("✅ CA 인증서 생성 완료");

    // TLS 버전 감지 테스트
    info!("🔍 TLS 버전 감지 테스트:");
    
    // TLS 1.0 ClientHello
    let tls10_hello = [0x16, 0x03, 0x01, 0x03, 0x01, 0x00, 0x98];
    if let Some(version) = TlsVersionDetector::detect_tls_version(&tls10_hello) {
        info!("  - TLS 1.0 감지: {} (rustls 지원: {})", 
              version, TlsVersionDetector::is_rustls_supported(version));
    }

    // TLS 1.1 ClientHello
    let tls11_hello = [0x16, 0x03, 0x01, 0x03, 0x02, 0x00, 0x98];
    if let Some(version) = TlsVersionDetector::detect_tls_version(&tls11_hello) {
        info!("  - TLS 1.1 감지: {} (rustls 지원: {})", 
              version, TlsVersionDetector::is_rustls_supported(version));
    }

    // TLS 1.2 ClientHello
    let tls12_hello = [0x16, 0x03, 0x01, 0x03, 0x03, 0x00, 0x98];
    if let Some(version) = TlsVersionDetector::detect_tls_version(&tls12_hello) {
        info!("  - TLS 1.2 감지: {} (rustls 지원: {})", 
              version, TlsVersionDetector::is_rustls_supported(version));
    }

    // TLS 1.3 ClientHello
    let tls13_hello = [0x16, 0x03, 0x01, 0x03, 0x04, 0x00, 0x98];
    if let Some(version) = TlsVersionDetector::detect_tls_version(&tls13_hello) {
        info!("  - TLS 1.3 감지: {} (rustls 지원: {})", 
              version, TlsVersionDetector::is_rustls_supported(version));
    }

    // 프록시 서버 시작
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("🌐 프록시 서버 주소: {}", addr);

    let proxy = Proxy::builder()
        .with_addr(addr)
        .with_ca(ca)
        .with_rustls_client(tokio_rustls::rustls::crypto::aws_lc_rs::default_provider())
        .build()
        .expect("프록시 빌드 실패");

    // Graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("🛑 Shutdown signal received");
    };

    info!("✅ 하이브리드 TLS 프록시가 시작되었습니다");
    info!("📝 TLS 1.0/1.1 클라이언트는 OpenSSL로 처리 예정");
    info!("📝 TLS 1.2/1.3 클라이언트는 rustls로 처리");
    info!("🔧 프록시 설정: http://127.0.0.1:8080");

    proxy.start().await?;

    Ok(())
}
