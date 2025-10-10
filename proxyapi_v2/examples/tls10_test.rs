use proxyapi_v2::{
    certificate_authority::RcgenAuthority, 
    TlsVersionDetector,
    NoopHandler, Proxy,
};
use std::net::SocketAddr;
use tokio::signal;
use tracing::{info, Level, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 로깅 설정
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🧪 TLS 1.0 테스트 프록시 시작");

    // CA 인증서 동적 생성
    let key_pair = rcgen::KeyPair::generate().expect("Failed to generate key pair");
    let mut ca_cert = rcgen::CertificateParams::new(vec!["localhost".to_string()])
        .expect("Failed to create certificate params");
    ca_cert.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let ca_cert = ca_cert.self_signed(&key_pair).expect("Failed to sign CA certificate");

    let ca = RcgenAuthority::new(key_pair, ca_cert, 1000, tokio_rustls::rustls::crypto::aws_lc_rs::default_provider());
    info!("✅ CA 인증서 생성 완료");

    // gateway.icloud.com:443에 대한 TLS 1.0 ClientHello 시뮬레이션
    info!("🔍 gateway.icloud.com:443 TLS 1.0 테스트:");
    
    // 실제 TLS 1.0 ClientHello 패킷 (gateway.icloud.com에서 받은 것과 유사)
    let tls10_hello = [
        0x16, 0x03, 0x01, 0x00, 0x98, 0x01, 0x00, 0x00, 0x94, 0x03, 0x01, 
        0x99, 0xa5, 0x7b, 0x72, 0x2a, 0xaa, 0xcb, 0xc2, 0x96, 0x2b, 0x30, 
        0x63, 0x75, 0x6b, 0xac, 0x72, 0x4f, 0x56, 0xbe, 0x7b, 0xbe
    ];
    
    if let Some(version) = TlsVersionDetector::detect_tls_version(&tls10_hello) {
        info!("  - 감지된 TLS 버전: {}", version);
        info!("  - rustls 지원: {}", TlsVersionDetector::is_rustls_supported(version));
        
        if !TlsVersionDetector::is_rustls_supported(version) {
            error!("❌ TLS 1.0은 rustls에서 지원되지 않음");
            info!("🔧 해결 방법:");
            info!("  1. OpenSSL 기반 TLS 라이브러리 사용");
            info!("  2. 클라이언트를 TLS 1.2 이상으로 업그레이드");
            info!("  3. 하이브리드 TLS 핸들러 완전 구현");
        }
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

    info!("✅ TLS 1.0 테스트 프록시가 시작되었습니다");
    info!("📝 gateway.icloud.com:443 연결 테스트 준비 완료");
    info!("🔧 프록시 설정: http://127.0.0.1:8080");
    info!("🧪 테스트 방법:");
    info!("  1. 브라우저에서 gateway.icloud.com 접속");
    info!("  2. 프록시 설정: 127.0.0.1:8080");
    info!("  3. TLS 핸드셰이크 로그 확인");

    proxy.start().await?;

    Ok(())
}
