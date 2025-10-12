use crate::certificate_authority::{CACHE_TTL, CertificateAuthority, NOT_BEFORE_OFFSET, TTL_SECS};
use http::uri::Authority;
use moka::future::Cache;
use rand::{Rng, rng};
use rcgen::{
    Certificate, CertificateParams, DistinguishedName, DnType, Ia5String, KeyPair, SanType,
};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tokio_rustls::rustls::{
    ServerConfig,
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
};
use tracing::{debug, error, info, warn};

/// Issues certificates for use when communicating with clients.
///
/// Issues certificates for communicating with clients over TLS. Certificates are cached in memory
/// up to a max size that is provided when creating the authority. Certificates are generated using
/// the `rcgen` crate.
///
/// # Examples
///
/// ```rust
/// use hudsucker::{certificate_authority::RcgenAuthority, rustls::crypto::aws_lc_rs};
/// use rcgen::{CertificateParams, KeyPair};
///
/// let key_pair = include_str!("../../examples/ca/hudsucker.key");
/// let ca_cert = include_str!("../../examples/ca/hudsucker.cer");
/// let key_pair = KeyPair::from_pem(key_pair).expect("Failed to parse private key");
/// let ca_cert = CertificateParams::from_ca_cert_pem(ca_cert)
///     .expect("Failed to parse CA certificate")
///     .self_signed(&key_pair)
///     .expect("Failed to sign CA certificate");
///
/// let ca = RcgenAuthority::new(key_pair, ca_cert, 1_000, aws_lc_rs::default_provider());
/// ```
pub struct RcgenAuthority {
    key_pair: KeyPair,
    ca_cert: Certificate,
    private_key: PrivateKeyDer<'static>,
    cache: Cache<Authority, Arc<ServerConfig>>,
    provider: Arc<CryptoProvider>,
}

impl RcgenAuthority {
    /// Creates a new rcgen authority.
    pub fn new(
        key_pair: KeyPair,
        ca_cert: Certificate,
        cache_size: u64,
        provider: CryptoProvider,
    ) -> Self {
        let private_key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

        Self {
            key_pair,
            ca_cert,
            private_key,
            cache: Cache::builder()
                .max_capacity(cache_size)
                .time_to_live(std::time::Duration::from_secs(CACHE_TTL))
                .build(),
            provider: Arc::new(provider),
        }
    }

    fn gen_cert(&self, authority: &Authority) -> CertificateDer<'static> {
        info!("Generating certificate for authority: {}", authority);

        let mut params = CertificateParams::default();
        params.serial_number = Some(rng().random::<u64>().into());

        let not_before = OffsetDateTime::now_utc() - Duration::seconds(NOT_BEFORE_OFFSET);
        params.not_before = not_before;
        params.not_after = not_before + Duration::seconds(TTL_SECS);

        let host = authority.host();
        debug!("Certificate host: {}", host);

        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, host);
        params.distinguished_name = distinguished_name;

        // SAN에 여러 형태의 도메인 추가로 호환성 향상
        self.add_san_entries(&mut params, host);

        // 에러 발생 시 더 자세한 정보 제공
        let cert = params
            .signed_by(&self.key_pair, &self.ca_cert, &self.key_pair)
            .map_err(|e| {
                eprintln!("Failed to sign certificate for '{}': {:?}", authority, e);
                e
            })
            .expect("Failed to sign certificate");

        info!("Successfully generated certificate for '{}'", authority);
        cert.into()
    }

    /// SAN(Subject Alternative Name) 엔트리를 추가하여 호환성 향상
    fn add_san_entries(&self, params: &mut CertificateParams, host: &str) {
        debug!("Adding SAN entries for host: {}", host);

        // 기본 도메인 추가
        if let Ok(dns_name) = Ia5String::try_from(host) {
            params.subject_alt_names.push(SanType::DnsName(dns_name));
            debug!("Added DNS SAN: {}", host);
        } else {
            warn!("Failed to create DNS SAN for host: {}", host);
        }

        // 와일드카드 도메인 처리
        if !host.starts_with("*.") {
            // 서브도메인을 위한 와일드카드 추가
            let wildcard = format!("*.{}", host);
            if let Ok(wildcard_name) = Ia5String::try_from(wildcard.as_str()) {
                params
                    .subject_alt_names
                    .push(SanType::DnsName(wildcard_name));
                debug!("Added wildcard SAN: {}", wildcard);
            } else {
                warn!("Failed to create wildcard SAN for: {}", wildcard);
            }
        }

        // IP 주소인 경우 처리
        match host.parse::<std::net::IpAddr>() {
            Ok(ip_addr) => {
                params.subject_alt_names.push(SanType::IpAddress(ip_addr));
                debug!("Added IP SAN: {}", ip_addr);
            }
            Err(e) => {
                warn!(
                    "Failed to parse IP address for SAN from host '{}': {}",
                    host, e
                );
            }
        }

        // localhost 및 127.0.0.1 처리
        if host == "localhost" {
            if let Ok(localhost_ip) = "127.0.0.1".parse::<std::net::IpAddr>() {
                params
                    .subject_alt_names
                    .push(SanType::IpAddress(localhost_ip));
                debug!("Added localhost IP SAN: {}", localhost_ip);
            }
        }

        info!(
            "Generated {} SAN entries for host '{}'",
            params.subject_alt_names.len(),
            host
        );
        debug!("SAN entries: {:?}", params.subject_alt_names);
    }
}

impl CertificateAuthority for RcgenAuthority {
    async fn gen_server_config(&self, authority: &Authority) -> Arc<ServerConfig> {
        if let Some(server_cfg) = self.cache.get(authority).await {
            debug!("Using cached server config");
            return server_cfg;
        }
        debug!("Generating server config");

        let certs = vec![self.gen_cert(authority)];

        let mut server_cfg = ServerConfig::builder_with_provider(Arc::clone(&self.provider))
            .with_safe_default_protocol_versions()
            .expect("Failed to specify protocol versions")
            .with_no_client_auth()
            .with_single_cert(certs, self.private_key.clone_key())
            .expect("Failed to build ServerConfig");

        // ALPN 프로토콜 설정 - HTTP/2 우선, HTTP/1.1 fallback
        server_cfg.alpn_protocols = vec![
            #[cfg(feature = "http2")]
            b"h2".to_vec(),
            b"http/1.1".to_vec(),
        ];

        debug!(
            "Server config ALPN protocols: {:?}",
            server_cfg.alpn_protocols
        );

        let server_cfg = Arc::new(server_cfg);

        self.cache
            .insert(authority.clone(), Arc::clone(&server_cfg))
            .await;

        server_cfg
    }

    fn get_ca_cert_der(&self) -> Option<Vec<u8>> {
        // rcgen::Certificate에서 DER 형식으로 CA 인증서를 추출
        let der_bytes = self.ca_cert.der().to_vec();
        debug!(
            "Successfully extracted CA certificate DER ({} bytes)",
            der_bytes.len()
        );
        Some(der_bytes)
    }

    #[cfg(feature = "native-tls-client")]
    async fn gen_pkcs12_identity(&self, authority: &Authority) -> Option<Vec<u8>> {
        #[cfg(feature = "openssl-ca")]
        {
            use openssl::{pkcs12::Pkcs12, pkey::PKey, x509::X509};

            info!("🔧 PKCS12 인증서 생성 시작: {}", authority);

            // rcgen 인증서를 DER 형식으로 생성
            let cert_der = self.gen_cert(authority);

            // DER 형식의 인증서를 OpenSSL X509 객체로 변환
            let cert = match X509::from_der(&cert_der) {
                Ok(cert) => cert,
                Err(e) => {
                    error!("❌ X509 인증서 변환 실패: {}", e);
                    return None;
                }
            };

            // rcgen 개인키를 DER 형식으로 변환
            let private_key_der = self.key_pair.serialize_der();
            let private_key = match PKey::private_key_from_der(&private_key_der) {
                Ok(key) => key,
                Err(e) => {
                    error!("❌ 개인키 변환 실패: {}", e);
                    return None;
                }
            };

            // PKCS12 생성 (패스워드 없음)
            match Pkcs12::builder()
                .name("")
                .pkey(&private_key)
                .cert(&cert)
                .build2("")
            {
                Ok(pkcs12) => {
                    let pkcs12_der = match pkcs12.to_der() {
                        Ok(der) => der,
                        Err(e) => {
                            error!("❌ PKCS12 DER 변환 실패: {}", e);
                            return None;
                        }
                    };

                    info!("✅ PKCS12 인증서 생성 성공: {} bytes", pkcs12_der.len());
                    Some(pkcs12_der)
                }
                Err(e) => {
                    error!("❌ PKCS12 생성 실패: {}", e);
                    None
                }
            }
        }
        #[cfg(not(feature = "openssl-ca"))]
        {
            warn!("PKCS12 생성은 openssl-ca feature가 필요합니다");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_rustls::rustls::crypto::aws_lc_rs;

    fn build_ca(cache_size: u64) -> RcgenAuthority {
        let key_pair = include_str!("cheolsu-proxy.key");
        let ca_cert = include_str!("cheolsu-proxy.cer");
        let key_pair = KeyPair::from_pem(key_pair).expect("Failed to parse private key");
        let ca_cert = CertificateParams::from_ca_cert_pem(ca_cert)
            .expect("Failed to parse CA certificate")
            .self_signed(&key_pair)
            .expect("Failed to sign CA certificate");

        RcgenAuthority::new(key_pair, ca_cert, cache_size, aws_lc_rs::default_provider())
    }

    #[test]
    fn unique_serial_numbers() {
        let ca = build_ca(0);

        let authority1 = Authority::from_static(
            "https://media.adpnut.com/cgi-bin/PelicanC.dll?impr?pageid=02AZ&lang=utf-8&out=iframe",
        );
        let authority2 = Authority::from_static(
            "https//ad.aceplanet.co.kr/cgi-bin/PelicanC.dll?impr?pageid=06P0&campaignid=01sL&gothrough=nextgrade&out=iframe",
        );

        let c1 = ca.gen_cert(&authority1);
        let c2 = ca.gen_cert(&authority2);
        let c3 = ca.gen_cert(&authority1);
        let c4 = ca.gen_cert(&authority2);

        let (_, cert1) = x509_parser::parse_x509_certificate(&c1).unwrap();
        let (_, cert2) = x509_parser::parse_x509_certificate(&c2).unwrap();

        assert_ne!(cert1.raw_serial(), cert2.raw_serial());

        let (_, cert3) = x509_parser::parse_x509_certificate(&c3).unwrap();
        let (_, cert4) = x509_parser::parse_x509_certificate(&c4).unwrap();

        assert_ne!(cert3.raw_serial(), cert4.raw_serial());

        assert_ne!(cert1.raw_serial(), cert3.raw_serial());
        assert_ne!(cert2.raw_serial(), cert4.raw_serial());
    }
}
