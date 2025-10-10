use proxyapi_v2::{NoopHandler, Proxy, certificate_authority::RcgenAuthority};
use std::net::SocketAddr;
use tokio::signal;
use tokio_rustls::rustls::crypto::aws_lc_rs;
use tracing::{Level, info};
use tracing_subscriber;

/// macOS에서 네트워크 프록시 설정을 자동으로 관리하는 함수
async fn set_system_proxy(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    let proxy_url = "http://127.0.0.1:8080";

    if enable {
        info!("🔧 시스템 프록시 설정: {}", proxy_url);

        // HTTP 프록시 설정
        let _ = std::process::Command::new("networksetup")
            .args(&["-setwebproxy", "Wi-Fi", "127.0.0.1", "8080"])
            .output();

        // HTTPS 프록시 설정
        let _ = std::process::Command::new("networksetup")
            .args(&["-setsecurewebproxy", "Wi-Fi", "127.0.0.1", "8080"])
            .output();

        info!("✅ 시스템 프록시 설정 완료");
    } else {
        info!("🔧 시스템 프록시 해제");

        // HTTP 프록시 해제
        let _ = std::process::Command::new("networksetup")
            .args(&["-setwebproxystate", "Wi-Fi", "off"])
            .output();

        // HTTPS 프록시 해제
        let _ = std::process::Command::new("networksetup")
            .args(&["-setsecurewebproxystate", "Wi-Fi", "off"])
            .output();

        info!("✅ 시스템 프록시 해제 완료");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 자동 프록시 테스트 시작");

    // CA 인증서 생성
    let key_pair = rcgen::KeyPair::generate().expect("Failed to generate key pair");
    let mut ca_cert = rcgen::CertificateParams::new(vec!["localhost".to_string()])
        .expect("Failed to create certificate params");
    ca_cert.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let ca_cert = ca_cert
        .self_signed(&key_pair)
        .expect("Failed to sign CA certificate");

    let ca = RcgenAuthority::new(key_pair, ca_cert, 1000, aws_lc_rs::default_provider());
    info!("✅ CA 인증서 생성 완료");

    // 프록시 서버 시작
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("🌐 프록시 서버 주소: {}", addr);

    let proxy = Proxy::builder()
        .with_addr(addr)
        .with_ca(ca)
        .with_rustls_client(aws_lc_rs::default_provider())
        .with_http_handler(NoopHandler::default())
        .with_websocket_handler(NoopHandler::default())
        .build()
        .expect("프록시 빌드 실패");

    // 시스템 프록시 설정
    set_system_proxy(true).await?;

    // Graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
        info!("SIGINT 수신, 프록시 종료 중...");
    };

    info!("✅ 자동 프록시 테스트가 시작되었습니다");
    info!("📝 gateway.icloud.com:443 연결 테스트 준비 완료");
    info!("🔧 프록시 설정: http://127.0.0.1:8080");
    info!("🧪 테스트 방법:");
    info!("   1. 브라우저에서 gateway.icloud.com 접속");
    info!("   2. TLS 핸드셰이크 로그 확인");
    info!("   3. Ctrl+C로 종료 시 자동으로 프록시 해제");

    // 프록시 시작 (백그라운드)
    let proxy_handle = tokio::spawn(async move {
        if let Err(e) = proxy.start().await {
            eprintln!("프록시 실행 오류: {}", e);
        }
    });

    // 종료 신호 대기
    shutdown_signal.await;

    // 시스템 프록시 해제
    set_system_proxy(false).await?;

    // 프록시 종료
    proxy_handle.abort();

    info!("✅ 테스트 완료, 프록시 해제됨");

    Ok(())
}
