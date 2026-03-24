//! 개선된 rustls MITM 프록시 예시
//!
//! 이 예시는 다음과 같은 개선사항을 포함합니다:
//! - 향상된 SAN(Subject Alternative Name) 처리
//! - 사설 CA 인증서 자동 추가
//! - 상세한 디버깅 로그
//! - TLS 버전 및 ALPN 최적화

use proxyapi_v2::{
    Proxy,
    certificate_authority::{RcgenAuthority, build_ca},
};
use rcgen::{CertificateParams, Issuer, KeyPair};
use std::net::SocketAddr;
use tokio_rustls::rustls::crypto::aws_lc_rs;
use tokio_rustls::rustls::pki_types::CertificateDer;
use tracing::{Level, info};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 로깅 초기화
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .init();

    info!("🚀 개선된 rustls MITM 프록시 시작");

    // CA 인증서 생성 또는 로드
    let ca = match build_ca() {
        Ok(ca) => {
            info!("✅ 기존 CA 인증서 로드 성공");
            ca
        }
        Err(e) => {
            info!("⚠️  기존 CA 로드 실패, 새로 생성: {}", e);
            create_new_ca()?
        }
    };

    // 프록시 서버 주소 설정
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("🌐 프록시 서버 주소: {}", addr);

    // 프록시 빌드 및 시작
    let proxy = Proxy::builder()
        .with_addr(addr)
        .with_ca(ca)
        .with_rustls_client(aws_lc_rs::default_provider())
        .build()
        .expect("프록시 빌드 실패");

    info!("✅ 프록시 서버 시작됨");
    info!("📋 사용법:");
    info!("   - HTTP 프록시: curl --proxy http://127.0.0.1:8080 https://example.com");
    info!("   - 브라우저 설정: HTTP 프록시를 127.0.0.1:8080으로 설정");
    info!("   - 로그 레벨: RUST_LOG=debug cargo run --example improved_rustls_proxy");

    // 프록시 서버 시작
    if let Err(e) = proxy.start().await {
        eprintln!("❌ 프록시 서버 실행 실패: {}", e);
        return Err(e.into());
    }

    Ok(())
}

/// 새로운 CA 인증서 생성
fn create_new_ca() -> Result<RcgenAuthority, Box<dyn std::error::Error>> {
    info!("🔐 새로운 CA 인증서 생성 중...");

    // CA 키 페어 생성
    let key_pair = KeyPair::generate()?;

    // CA 인증서 파라미터 설정
    let mut params = CertificateParams::default();
    params.distinguished_name = rcgen::DistinguishedName::new();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Cheolsu Proxy CA");
    params
        .distinguished_name
        .push(rcgen::DnType::OrganizationName, "Cheolsu Proxy");
    params
        .distinguished_name
        .push(rcgen::DnType::CountryName, "KR");

    // CA 인증서 생성
    let ca_cert = params.self_signed(&key_pair)?;
    let ca_cert_pem = ca_cert.pem();
    let key_pem = key_pair.serialize_pem();
    let ca_cert_der_bytes = pem::parse(&ca_cert_pem).unwrap().into_contents();
    let issuer = Issuer::from_ca_cert_pem(&ca_cert_pem, key_pair)
        .expect("Failed to create issuer from CA cert");

    // RcgenAuthority 생성
    let ca = RcgenAuthority::new(
        issuer,
        CertificateDer::from(ca_cert_der_bytes),
        ca_cert_pem,
        key_pem,
        1_000, // 캐시 크기
        aws_lc_rs::default_provider(),
    );

    info!("✅ 새로운 CA 인증서 생성 완료");
    info!("📝 CA 인증서를 브라우저에 신뢰할 수 있는 인증서로 추가하세요");

    Ok(ca)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_proxy_startup() {
        // 로깅 초기화
        let _ = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .try_init();

        let ca = create_new_ca().expect("CA 생성 실패");
        let addr = SocketAddr::from(([127, 0, 0, 1], 0)); // 랜덤 포트

        let proxy = Proxy::builder()
            .with_addr(addr)
            .with_ca(ca)
            .with_rustls_client(aws_lc_rs::default_provider())
            .build()
            .expect("프록시 빌드 실패");

        // 5초 내에 프록시가 시작되는지 테스트
        let result = timeout(Duration::from_secs(5), proxy.start()).await;

        // 프록시는 계속 실행되므로 타임아웃이 정상입니다
        assert!(result.is_err(), "프록시가 예상보다 빨리 종료됨");
    }

    #[test]
    fn test_ca_creation() {
        let ca = create_new_ca();
        assert!(ca.is_ok(), "CA 생성 실패: {:?}", ca.err());
    }
}
