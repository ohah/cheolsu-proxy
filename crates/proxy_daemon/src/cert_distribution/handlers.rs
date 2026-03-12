use bytes::Bytes;
use proxyapi_v2::{
    hyper::{Request, Response},
    Body,
};
use tracing::info;

use super::template::build_landing_html;
use super::types::{detect_platform, Platform, CERT_DOWNLOAD_HOST, CERT_DOWNLOAD_HOST_COLON};

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
    if req.uri().host().is_none() && req.headers().get("host").is_none() {
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

pub(super) fn der_to_pem(der: &[u8]) -> Vec<u8> {
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
    match platform {
        Platform::Ios => serve_pem(ca_cert_der),
        Platform::Android => serve_der(ca_cert_der),
        Platform::Unknown => serve_crt(ca_cert_der),
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
