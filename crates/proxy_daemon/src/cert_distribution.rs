use bytes::Bytes;
use proxyapi_v2::{
    hyper::{Request, Response},
    Body,
};
use tracing::info;

const CERT_DOWNLOAD_HOST: &str = "cheolsu.proxy";
const CERT_DOWNLOAD_HOST_COLON: &str = "cheolsu.proxy:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Platform {
    Ios,
    Android,
    Unknown,
}

fn detect_platform(user_agent: &str) -> Platform {
    let ua = user_agent.to_lowercase();
    if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ipod") || ua.contains("mac os")
    {
        Platform::Ios
    } else if ua.contains("android") {
        Platform::Android
    } else {
        Platform::Unknown
    }
}

pub(crate) fn is_cert_download_request(req: &Request<Body>) -> bool {
    if let Some(host) = req.headers().get("host").and_then(|v| v.to_str().ok()) {
        if host == CERT_DOWNLOAD_HOST || host.starts_with(CERT_DOWNLOAD_HOST_COLON) {
            return true;
        }
    }
    if let Some(host) = req.uri().host() {
        if host == CERT_DOWNLOAD_HOST {
            return true;
        }
    }
    if req.uri().host().is_none() {
        let path = req.uri().path();
        if path == "/ssl" || path == "/cert" {
            return true;
        }
    }
    false
}

pub(crate) fn handle_cert_request(
    req: &Request<Body>,
    ca_cert_der: Option<&Bytes>,
) -> Response<Body> {
    let path = req.uri().path();
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let platform = detect_platform(user_agent);

    match path {
        "/ssl/pem" => serve_pem(ca_cert_der),
        "/ssl/der" => serve_der(ca_cert_der),
        "/ssl/ca.crt" => serve_crt(ca_cert_der),
        "/ssl" | "/cert" => serve_auto_download(ca_cert_der, platform),
        _ => serve_landing_page(ca_cert_der.is_some(), platform),
    }
}

fn der_to_pem(der: &[u8]) -> Vec<u8> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(der);
    let mut pem = String::with_capacity(b64.len() + 60);
    pem.push_str("-----BEGIN CERTIFICATE-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem.into_bytes()
}

fn not_found_response() -> Response<Body> {
    Response::builder()
        .status(404)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Body::from(
            "CA certificate is not available. Please start the proxy first.",
        ))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn serve_pem(ca_cert_der: Option<&Bytes>) -> Response<Body> {
    let Some(der) = ca_cert_der else {
        return not_found_response();
    };
    let pem = der_to_pem(der);
    info!(
        "[CertDistribution] PEM certificate download ({} bytes)",
        pem.len()
    );
    Response::builder()
        .status(200)
        .header("Content-Type", "application/x-pem-file")
        .header(
            "Content-Disposition",
            "attachment; filename=\"cheolsu-proxy-ca.pem\"",
        )
        .header("Content-Length", pem.len().to_string())
        .body(Body::from(http_body_util::Full::new(Bytes::from(pem))))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn serve_der(ca_cert_der: Option<&Bytes>) -> Response<Body> {
    let Some(der) = ca_cert_der else {
        return not_found_response();
    };
    info!(
        "[CertDistribution] DER certificate download ({} bytes)",
        der.len()
    );
    Response::builder()
        .status(200)
        .header("Content-Type", "application/x-x509-ca-cert")
        .header(
            "Content-Disposition",
            "attachment; filename=\"cheolsu-proxy-ca.der\"",
        )
        .header("Content-Length", der.len().to_string())
        .body(Body::from(http_body_util::Full::new(der.clone())))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn serve_crt(ca_cert_der: Option<&Bytes>) -> Response<Body> {
    let Some(der) = ca_cert_der else {
        return not_found_response();
    };
    let pem = der_to_pem(der);
    info!(
        "[CertDistribution] CRT certificate download ({} bytes)",
        pem.len()
    );
    Response::builder()
        .status(200)
        .header("Content-Type", "application/x-x509-ca-cert")
        .header(
            "Content-Disposition",
            "attachment; filename=\"cheolsu-proxy-ca.crt\"",
        )
        .header("Content-Length", pem.len().to_string())
        .body(Body::from(http_body_util::Full::new(Bytes::from(pem))))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn serve_auto_download(ca_cert_der: Option<&Bytes>, platform: Platform) -> Response<Body> {
    let Some(der) = ca_cert_der else {
        return not_found_response();
    };
    match platform {
        Platform::Ios => {
            let pem = der_to_pem(der);
            info!(
                "[CertDistribution] iOS auto-download PEM ({} bytes)",
                pem.len()
            );
            Response::builder()
                .status(200)
                .header("Content-Type", "application/x-pem-file")
                .header(
                    "Content-Disposition",
                    "attachment; filename=\"cheolsu-proxy-ca.pem\"",
                )
                .header("Content-Length", pem.len().to_string())
                .body(Body::from(http_body_util::Full::new(Bytes::from(pem))))
                .unwrap_or_else(|_| Response::new(Body::empty()))
        }
        Platform::Android => {
            info!(
                "[CertDistribution] Android auto-download DER ({} bytes)",
                der.len()
            );
            Response::builder()
                .status(200)
                .header("Content-Type", "application/x-x509-ca-cert")
                .header(
                    "Content-Disposition",
                    "attachment; filename=\"cheolsu-proxy-ca.der\"",
                )
                .header("Content-Length", der.len().to_string())
                .body(Body::from(http_body_util::Full::new(der.clone())))
                .unwrap_or_else(|_| Response::new(Body::empty()))
        }
        Platform::Unknown => {
            info!(
                "[CertDistribution] Generic auto-download CER ({} bytes)",
                der.len()
            );
            Response::builder()
                .status(200)
                .header("Content-Type", "application/x-x509-ca-cert")
                .header(
                    "Content-Disposition",
                    "attachment; filename=\"cheolsu-proxy-ca.cer\"",
                )
                .header("Content-Length", der.len().to_string())
                .body(Body::from(http_body_util::Full::new(der.clone())))
                .unwrap_or_else(|_| Response::new(Body::empty()))
        }
    }
}

fn serve_landing_page(cert_available: bool, platform: Platform) -> Response<Body> {
    let html = build_landing_html(cert_available, platform);
    Response::builder()
        .status(200)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-cache")
        .body(Body::from(html))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn build_landing_html(cert_available: bool, platform: Platform) -> String {
    let ios_active = matches!(platform, Platform::Ios);
    let android_active = matches!(platform, Platform::Android);

    let download_section = if cert_available {
        format!(
            r#"<div class="download-buttons">
              <a href="/ssl/pem" class="btn btn-ios">Download for iOS (.pem)</a>
              <a href="/ssl/der" class="btn btn-android">Download for Android (.der)</a>
              <a href="/ssl/ca.crt" class="btn btn-generic">Download (.crt)</a>
            </div>"#
        )
    } else {
        r#"<div class="alert">CA certificate is not available yet. Please start the proxy first.</div>"#.to_string()
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,user-scalable=no">
<title>Cheolsu Proxy - CA Certificate</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;
background:#f0f2f5;color:#1a1a2e;min-height:100vh;display:flex;flex-direction:column;align-items:center;
padding:20px}}
.container{{max-width:480px;width:100%;margin:0 auto}}
.header{{text-align:center;padding:32px 0 24px}}
.header h1{{font-size:28px;font-weight:700;color:#1a1a2e;margin-bottom:4px}}
.header .subtitle{{font-size:14px;color:#666;margin-top:8px}}
.logo{{width:64px;height:64px;background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);
border-radius:16px;display:flex;align-items:center;justify-content:center;margin:0 auto 16px;
font-size:28px;color:white;font-weight:bold}}
.card{{background:white;border-radius:16px;padding:24px;margin-bottom:16px;
box-shadow:0 1px 3px rgba(0,0,0,0.08),0 1px 2px rgba(0,0,0,0.06)}}
.tabs{{display:flex;gap:0;margin-bottom:20px;border-radius:10px;overflow:hidden;
background:#f0f2f5;padding:3px}}
.tab{{flex:1;padding:10px 16px;text-align:center;cursor:pointer;font-size:14px;
font-weight:600;border-radius:8px;transition:all 0.2s;color:#666;border:none;background:none}}
.tab.active{{background:white;color:#1a1a2e;box-shadow:0 1px 3px rgba(0,0,0,0.1)}}
.tab-content{{display:none}}
.tab-content.active{{display:block}}
.steps{{list-style:none;counter-reset:step}}
.steps li{{counter-increment:step;padding:12px 0 12px 44px;position:relative;
font-size:14px;line-height:1.5;border-bottom:1px solid #f0f2f5}}
.steps li:last-child{{border-bottom:none}}
.steps li::before{{content:counter(step);position:absolute;left:0;top:12px;
width:28px;height:28px;border-radius:50%;background:#667eea;color:white;
font-size:13px;font-weight:700;display:flex;align-items:center;justify-content:center}}
.steps li strong{{display:block;margin-bottom:2px}}
.download-buttons{{display:flex;flex-direction:column;gap:10px;margin-top:8px}}
.btn{{display:block;padding:14px 24px;border-radius:12px;text-decoration:none;
font-weight:600;font-size:15px;text-align:center;transition:all 0.2s}}
.btn-ios{{background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);color:white}}
.btn-ios:hover{{opacity:0.9;transform:translateY(-1px)}}
.btn-android{{background:linear-gradient(135deg,#43e97b 0%,#38f9d7 100%);color:#1a1a2e}}
.btn-android:hover{{opacity:0.9;transform:translateY(-1px)}}
.btn-generic{{background:#f0f2f5;color:#1a1a2e;border:1px solid #ddd}}
.btn-generic:hover{{background:#e4e6e9;transform:translateY(-1px)}}
.alert{{padding:16px;background:#fff3cd;border-radius:10px;font-size:14px;color:#856404;text-align:center}}
.footer{{text-align:center;padding:16px 0;font-size:12px;color:#999}}
@media(max-width:360px){{
.header h1{{font-size:24px}}
.card{{padding:16px}}
}}
</style>
</head>
<body>
<div class="container">
  <div class="header">
    <div class="logo">CP</div>
    <h1>Cheolsu Proxy</h1>
    <p class="subtitle">Install the CA certificate to enable HTTPS inspection</p>
  </div>

  <div class="card">
    <h2 style="font-size:17px;margin-bottom:16px">Download Certificate</h2>
    {download_section}
  </div>

  <div class="card">
    <div class="tabs">
      <button class="tab{ios_cls}" onclick="showTab('ios')" id="tab-ios">iOS</button>
      <button class="tab{android_cls}" onclick="showTab('android')" id="tab-android">Android</button>
    </div>

    <div class="tab-content{ios_content_cls}" id="content-ios">
      <ol class="steps">
        <li><strong>Download the certificate</strong>Tap the "Download for iOS" button above. Safari will show a profile download prompt.</li>
        <li><strong>Install the profile</strong>Go to <em>Settings &gt; General &gt; VPN &amp; Device Management</em> and tap the downloaded profile to install it.</li>
        <li><strong>Trust the certificate</strong>Go to <em>Settings &gt; General &gt; About &gt; Certificate Trust Settings</em> and enable full trust for "Cheolsu Proxy CA".</li>
      </ol>
    </div>

    <div class="tab-content{android_content_cls}" id="content-android">
      <ol class="steps">
        <li><strong>Download the certificate</strong>Tap the "Download for Android" button above to download the DER file.</li>
        <li><strong>Open security settings</strong>Go to <em>Settings &gt; Security &gt; Encryption &amp; credentials &gt; Install a certificate</em>.</li>
        <li><strong>Install the certificate</strong>Select "CA certificate", then choose the downloaded file. Confirm the security warning.</li>
        <li><strong>Verify installation</strong>Go to <em>Settings &gt; Security &gt; Trusted credentials &gt; User</em> to verify "Cheolsu Proxy CA" appears.</li>
      </ol>
    </div>
  </div>

  <div class="footer">Cheolsu Proxy &middot; Serving from http://cheolsu.proxy/</div>
</div>

<script>
function showTab(name){{
  document.querySelectorAll('.tab').forEach(t=>t.classList.remove('active'));
  document.querySelectorAll('.tab-content').forEach(c=>c.classList.remove('active'));
  document.getElementById('tab-'+name).classList.add('active');
  document.getElementById('content-'+name).classList.add('active');
}}
</script>
</body>
</html>"##,
        download_section = download_section,
        ios_cls = if ios_active || (!ios_active && !android_active) {
            " active"
        } else {
            ""
        },
        android_cls = if android_active { " active" } else { "" },
        ios_content_cls = if ios_active || (!ios_active && !android_active) {
            " active"
        } else {
            ""
        },
        android_content_cls = if android_active { " active" } else { "" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .contains(".cer"));
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
}
