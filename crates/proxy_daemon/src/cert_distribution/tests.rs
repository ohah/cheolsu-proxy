use super::handlers::{der_to_pem, handle_cert_request, is_cert_download_request};
use super::template::build_landing_html;
use super::types::{detect_platform, Platform};
use bytes::Bytes;
use proxyapi_v2::{hyper::Request, Body};

#[test]
fn detect_ios_user_agent() {
    assert_eq!(
        detect_platform("Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)"),
        Platform::Ios
    );
    assert_eq!(
        detect_platform("Mozilla/5.0 (iPad; CPU OS 16_0 like Mac OS X)"),
        Platform::Ios
    );
}

#[test]
fn detect_android_user_agent() {
    assert_eq!(
        detect_platform("Mozilla/5.0 (Linux; Android 13; Pixel 7)"),
        Platform::Android
    );
}

#[test]
fn detect_unknown_user_agent() {
    assert_eq!(
        detect_platform("Mozilla/5.0 (Windows NT 10.0; Win64; x64)"),
        Platform::Unknown
    );
    assert_eq!(detect_platform(""), Platform::Unknown);
}

#[test]
fn is_cert_request_host_header() {
    let req = Request::builder()
        .uri("/anything")
        .header("host", "cheolsu.proxy")
        .body(Body::from(""))
        .unwrap();
    assert!(is_cert_download_request(&req));
}

#[test]
fn is_cert_request_host_header_with_port() {
    let req = Request::builder()
        .uri("/anything")
        .header("host", "cheolsu.proxy:8080")
        .body(Body::from(""))
        .unwrap();
    assert!(is_cert_download_request(&req));
}

#[test]
fn is_cert_request_absolute_uri() {
    let req = Request::builder()
        .uri("http://cheolsu.proxy/ssl")
        .body(Body::from(""))
        .unwrap();
    assert!(is_cert_download_request(&req));
}

#[test]
fn is_cert_request_direct_ssl_path() {
    let req = Request::builder().uri("/ssl").body(Body::from("")).unwrap();
    assert!(is_cert_download_request(&req));
}

#[test]
fn is_cert_request_direct_cert_path() {
    let req = Request::builder()
        .uri("/cert")
        .body(Body::from(""))
        .unwrap();
    assert!(is_cert_download_request(&req));
}

#[test]
fn is_not_cert_request_other_host() {
    let req = Request::builder()
        .uri("/api")
        .header("host", "example.com")
        .body(Body::from(""))
        .unwrap();
    assert!(!is_cert_download_request(&req));
}

#[test]
fn is_not_cert_request_other_path() {
    let req = Request::builder()
        .uri("/api/data")
        .body(Body::from(""))
        .unwrap();
    assert!(!is_cert_download_request(&req));
}

#[test]
fn handle_pem_download() {
    let der = Bytes::from(vec![0x30, 0x82, 0x01, 0x00]);
    let req = Request::builder()
        .uri("/ssl/pem")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, Some(&der));
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("Content-Type").unwrap(),
        "application/x-pem-file"
    );
    assert!(resp
        .headers()
        .get("Content-Disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains(".pem"));
}

#[test]
fn handle_der_download() {
    let der = Bytes::from(vec![0x30, 0x82]);
    let req = Request::builder()
        .uri("/ssl/der")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, Some(&der));
    assert_eq!(resp.status(), 200);
    assert!(resp
        .headers()
        .get("Content-Disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains(".der"));
}

#[test]
fn handle_crt_download() {
    let der = Bytes::from(vec![0x30]);
    let req = Request::builder()
        .uri("/ssl/ca.crt")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, Some(&der));
    assert_eq!(resp.status(), 200);
    assert!(resp
        .headers()
        .get("Content-Disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains(".crt"));
}

#[test]
fn handle_ssl_auto_ios() {
    let der = Bytes::from(vec![0x30]);
    let req = Request::builder()
        .uri("/ssl")
        .header("user-agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0)")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, Some(&der));
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("Content-Type").unwrap(),
        "application/x-pem-file"
    );
}

#[test]
fn handle_ssl_auto_android() {
    let der = Bytes::from(vec![0x30]);
    let req = Request::builder()
        .uri("/ssl")
        .header("user-agent", "Mozilla/5.0 (Linux; Android 13)")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, Some(&der));
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("Content-Type").unwrap(),
        "application/x-x509-ca-cert"
    );
}

#[test]
fn handle_ssl_auto_unknown() {
    let der = Bytes::from(vec![0x30]);
    let req = Request::builder().uri("/ssl").body(Body::from("")).unwrap();
    let resp = handle_cert_request(&req, Some(&der));
    assert_eq!(resp.status(), 200);
    assert!(resp
        .headers()
        .get("Content-Disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains(".crt"));
}

#[test]
fn handle_landing_page() {
    let der = Bytes::from(vec![0x30]);
    let req = Request::builder()
        .uri("/")
        .header("host", "cheolsu.proxy")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, Some(&der));
    assert_eq!(resp.status(), 200);
    assert!(resp
        .headers()
        .get("Content-Type")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("text/html"));
}

#[test]
fn handle_no_cert_available() {
    let req = Request::builder()
        .uri("/ssl/pem")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, None);
    assert_eq!(resp.status(), 404);
}

#[test]
fn der_to_pem_format() {
    let der = vec![0x30, 0x82, 0x01, 0x00];
    let pem = der_to_pem(&der);
    let pem_str = String::from_utf8(pem).unwrap();
    assert!(pem_str.starts_with("-----BEGIN CERTIFICATE-----\n"));
    assert!(pem_str.ends_with("-----END CERTIFICATE-----\n"));
}

#[test]
fn landing_page_without_cert() {
    let req = Request::builder()
        .uri("/about")
        .header("host", "cheolsu.proxy")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, None);
    assert_eq!(resp.status(), 200);
}

#[test]
fn landing_page_has_download_buttons() {
    let html = build_landing_html(true, Platform::Unknown);
    assert!(html.contains("/ssl/pem"));
    assert!(html.contains("/ssl/der"));
    assert!(html.contains("/ssl/ca.crt"));
}

#[test]
fn landing_page_shows_alert_without_cert() {
    let html = build_landing_html(false, Platform::Unknown);
    assert!(html.contains("not available"));
    assert!(!html.contains("/ssl/pem"));
}

#[test]
fn landing_page_ios_tab_active() {
    let html = build_landing_html(true, Platform::Ios);
    assert!(html.contains("id=\"tab-ios\">iOS</button>"));
    assert!(html.contains("id=\"content-ios\">"));
}

#[test]
fn landing_page_android_tab_active() {
    let html = build_landing_html(true, Platform::Android);
    assert!(html.contains("id=\"tab-android\">Android</button>"));
}

// --- 추가 엣지 케이스 테스트 ---

#[test]
fn der_to_pem_empty_input() {
    let pem = der_to_pem(&[]);
    let pem_str = String::from_utf8(pem).unwrap();
    assert!(pem_str.starts_with("-----BEGIN CERTIFICATE-----\n"));
    assert!(pem_str.ends_with("-----END CERTIFICATE-----\n"));
    // 빈 DER이면 BEGIN/END 사이에 빈 base64만 있어야 함
    let inner = pem_str
        .strip_prefix("-----BEGIN CERTIFICATE-----\n")
        .unwrap()
        .strip_suffix("-----END CERTIFICATE-----\n")
        .unwrap();
    assert_eq!(inner.trim(), "");
}

#[test]
fn detect_platform_very_long_user_agent() {
    // 매우 긴 User-Agent 문자열이 패닉 없이 처리되는지 확인
    let long_ua = "Mozilla/5.0 ".to_string() + &"x".repeat(10_000);
    assert_eq!(detect_platform(&long_ua), Platform::Unknown);

    // 긴 문자열 안에 플랫폼 키워드가 있으면 정상 감지
    let long_ios = "x".repeat(5_000) + "iPhone" + &"x".repeat(5_000);
    assert_eq!(detect_platform(&long_ios), Platform::Ios);

    let long_android = "x".repeat(5_000) + "Android" + &"x".repeat(5_000);
    assert_eq!(detect_platform(&long_android), Platform::Android);
}

#[test]
fn is_cert_request_host_with_port_443() {
    let req = Request::builder()
        .uri("/anything")
        .header("host", "cheolsu.proxy:443")
        .body(Body::from(""))
        .unwrap();
    assert!(is_cert_download_request(&req));
}

#[test]
fn is_cert_request_host_with_port_80() {
    let req = Request::builder()
        .uri("/anything")
        .header("host", "cheolsu.proxy:80")
        .body(Body::from(""))
        .unwrap();
    assert!(is_cert_download_request(&req));
}

#[test]
fn handle_cert_path_auto_download() {
    // /cert 경로도 /ssl과 동일하게 auto-download 동작해야 함
    let der = Bytes::from(vec![0x30, 0x82]);

    // Unknown platform -> .crt 다운로드
    let req = Request::builder()
        .uri("/cert")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, Some(&der));
    assert_eq!(resp.status(), 200);
    assert!(resp
        .headers()
        .get("Content-Disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains(".crt"));

    // iOS -> .pem 다운로드
    let req = Request::builder()
        .uri("/cert")
        .header("user-agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0)")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, Some(&der));
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("Content-Type").unwrap(),
        "application/x-pem-file"
    );

    // Android -> .der 다운로드
    let req = Request::builder()
        .uri("/cert")
        .header("user-agent", "Mozilla/5.0 (Linux; Android 14)")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, Some(&der));
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("Content-Type").unwrap(),
        "application/x-x509-ca-cert"
    );
}

#[test]
fn handle_cert_path_no_cert() {
    let req = Request::builder()
        .uri("/cert")
        .body(Body::from(""))
        .unwrap();
    let resp = handle_cert_request(&req, None);
    assert_eq!(resp.status(), 404);
}

#[test]
fn landing_page_contains_install_guide_text() {
    let html = build_landing_html(true, Platform::Unknown);
    // 페이지 제목
    assert!(html.contains("Cheolsu Proxy"));
    assert!(html.contains("CA Certificate"));
    // 설치 안내 텍스트
    assert!(html.contains("Install the CA certificate"));
    assert!(html.contains("Download Certificate"));
    // iOS 설치 가이드 단계
    assert!(html.contains("Download the certificate"));
    assert!(html.contains("Install the profile"));
    assert!(html.contains("Trust the certificate"));
    // Android 설치 가이드 단계
    assert!(html.contains("Open security settings"));
    assert!(html.contains("Install the certificate"));
    assert!(html.contains("Verify installation"));
    // 푸터
    assert!(html.contains("cheolsu.proxy"));
}

#[test]
fn landing_page_contains_install_guide_without_cert() {
    let html = build_landing_html(false, Platform::Ios);
    // 인증서가 없어도 설치 가이드 텍스트는 표시되어야 함
    assert!(html.contains("Install the CA certificate"));
    assert!(html.contains("Download the certificate"));
    // 다운로드 버튼 대신 경고 메시지
    assert!(html.contains("not available"));
}
