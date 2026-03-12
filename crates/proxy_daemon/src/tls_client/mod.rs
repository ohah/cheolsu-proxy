//! TLS 클라이언트 모듈: 인증서 로드, 검증, 하이브리드 클라이언트 생성 등

mod custom_ca;
mod loader;
mod resolver;
mod validation;
mod verifier;

#[cfg(test)]
mod tests;

pub use custom_ca::{get_custom_ca_info, parse_pkcs12, remove_custom_ca, save_custom_ca};
pub use loader::{
    load_certs, load_private_key, parse_certificate_info, parse_certificate_info_from_bytes,
};
pub use validation::{
    validate_ca_certificate, validate_ca_certificate_from_bytes, validate_cert_key_pair,
    validate_client_cert_config,
};

use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use proxyapi_v2::{
    upstream_proxy::{ProxyHttpConnector, UpstreamProxyConfig},
    Body,
};
use std::sync::Arc;
use tokio_rustls::rustls::{crypto::aws_lc_rs, ClientConfig};
use tracing::{error, info};

use crate::protocol::ClientCertConfig;
use resolver::{build_certified_key, DefaultCertResolver};
use verifier::DangerousCertificateVerifier;

/// 하이브리드 클라이언트 생성 (모든 인증서 허용, upstream proxy 지원)
///
/// `upstream_rx`를 통해 런타임에 upstream proxy 설정 변경이 즉시 반영됩니다.
pub fn create_hybrid_client(
    upstream_rx: tokio::sync::watch::Receiver<Option<UpstreamProxyConfig>>,
) -> Result<
    Client<hyper_rustls::HttpsConnector<ProxyHttpConnector>, Body>,
    Box<dyn std::error::Error>,
> {
    create_hybrid_client_with_cert(upstream_rx, None)
}

/// 클라이언트 인증서를 포함한 하이브리드 클라이언트 생성
pub fn create_hybrid_client_with_cert(
    upstream_rx: tokio::sync::watch::Receiver<Option<UpstreamProxyConfig>>,
    client_cert_config: Option<&ClientCertConfig>,
) -> Result<
    Client<hyper_rustls::HttpsConnector<ProxyHttpConnector>, Body>,
    Box<dyn std::error::Error>,
> {
    let config_builder =
        ClientConfig::builder_with_provider(std::sync::Arc::new(aws_lc_rs::default_provider()))
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(DangerousCertificateVerifier));

    let rustls_config = if let Some(cert_config) = client_cert_config {
        if cert_config.enabled {
            // 기본 인증서 로드
            let default_cert =
                match build_certified_key(&cert_config.cert_path, &cert_config.key_path) {
                    Ok(key) => {
                        info!(
                            "기본 클라이언트 인증서 로드 성공: cert={}, key={}",
                            cert_config.cert_path, cert_config.key_path
                        );
                        Some(Arc::new(key))
                    }
                    Err(e) => {
                        error!("기본 클라이언트 인증서 로드 실패: {}", e);
                        None
                    }
                };

            // 도메인별 인증서는 로드하여 로그만 남김 (검증 목적)
            // NOTE: 실제 도메인 매칭은 ResolvesClientCert 트레잇 한계로 미지원
            // 향후 per-connection ClientConfig 방식으로 확장 예정
            for dc in &cert_config.domain_certs {
                if !dc.enabled {
                    continue;
                }
                match build_certified_key(&dc.cert_path, &dc.key_path) {
                    Ok(_) => {
                        info!(
                            "도메인 인증서 검증 성공: pattern={}, cert={}",
                            dc.domain_pattern, dc.cert_path
                        );
                    }
                    Err(e) => {
                        error!("도메인 인증서 로드 실패 ({}): {}", dc.domain_pattern, e);
                    }
                }
            }

            let resolver = DefaultCertResolver { default_cert };
            config_builder.with_client_cert_resolver(Arc::new(resolver))
        } else {
            config_builder.with_no_client_auth()
        }
    } else {
        config_builder.with_no_client_auth()
    };

    let proxy_connector = ProxyHttpConnector::new(upstream_rx);

    let https = HttpsConnectorBuilder::new()
        .with_tls_config(rustls_config)
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .wrap_connector(proxy_connector);

    Ok(Client::builder(TokioExecutor::new())
        .http1_title_case_headers(true)
        .http1_preserve_header_case(true)
        .build(https))
}
