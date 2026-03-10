use crate::certificate_authority::{CACHE_TTL, CertificateAuthority, NOT_BEFORE_OFFSET, TTL_SECS};
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use moka::future::Cache;
use rand::{Rng, rng};
use rcgen::{
    Certificate, CertificateParams, DistinguishedName, DnType, Ia5String, KeyPair, SanType,
};
use std::collections::HashSet;
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
/// use proxyapi_v2::{certificate_authority::RcgenAuthority, rustls::crypto::aws_lc_rs};
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
    #[cfg(feature = "openssl-ca")]
    openssl_ctx_cache: Cache<Authority, Arc<openssl::ssl::SslContext>>,
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
            #[cfg(feature = "openssl-ca")]
            openssl_ctx_cache: Cache::builder()
                .max_capacity(cache_size)
                .time_to_live(std::time::Duration::from_secs(CACHE_TTL))
                .build(),
            provider: Arc::new(provider),
        }
    }

    fn gen_cert(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> CertificateDer<'static> {
        info!("Generating certificate for authority: {}", authority);

        let mut params = CertificateParams::default();
        params.serial_number = Some(rng().random::<u64>().into());

        let not_before = OffsetDateTime::now_utc() - Duration::seconds(NOT_BEFORE_OFFSET);
        params.not_before = not_before;
        params.not_after = not_before + Duration::seconds(TTL_SECS);

        let host = authority.host();
        debug!("Certificate host: {}", host);

        let mut distinguished_name = DistinguishedName::new();

        if let Some(upstream) = upstream_cert {
            // upstream 인증서의 CN 사용
            if let Some(ref cn) = upstream.common_name {
                distinguished_name.push(DnType::CommonName, cn);
            } else {
                distinguished_name.push(DnType::CommonName, host);
            }
            // upstream 인증서의 Organization 복제
            if let Some(ref org) = upstream.organization {
                distinguished_name.push(DnType::OrganizationName, org);
            }
        } else {
            distinguished_name.push(DnType::CommonName, host);
        }

        params.distinguished_name = distinguished_name;

        // SAN 엔트리 설정
        if let Some(upstream) = upstream_cert {
            self.add_upstream_san_entries(&mut params, host, upstream);
        } else {
            self.add_san_entries(&mut params, host);
        }

        let cert = params
            .signed_by(&self.key_pair, &self.ca_cert, &self.key_pair)
            .map_err(|e| {
                error!(authority = %authority, error = ?e, "Failed to sign certificate");
                e
            })
            .expect("Failed to sign certificate");

        info!("Successfully generated certificate for '{}'", authority);
        cert.into()
    }

    /// 상류 인증서의 SAN 정보를 복제하여 위조 인증서에 추가
    fn add_upstream_san_entries(
        &self,
        params: &mut CertificateParams,
        host: &str,
        upstream: &UpstreamCertInfo,
    ) {
        debug!(
            "Adding upstream SAN entries for host: {} (upstream DNS SANs: {}, IP SANs: {})",
            host,
            upstream.sans_dns.len(),
            upstream.sans_ip.len()
        );

        let mut added_dns: HashSet<String> = HashSet::new();

        // upstream 인증서의 DNS SAN 복제
        for dns in &upstream.sans_dns {
            if added_dns.insert(dns.clone()) {
                if let Ok(dns_name) = Ia5String::try_from(dns.as_str()) {
                    params.subject_alt_names.push(SanType::DnsName(dns_name));
                }
            }
        }

        // 호스트 도메인이 SAN에 없으면 추가
        if added_dns.insert(host.to_string()) {
            if let Ok(dns_name) = Ia5String::try_from(host) {
                params.subject_alt_names.push(SanType::DnsName(dns_name));
            }
        }

        // upstream 인증서의 IP SAN 복제
        let mut added_ip: HashSet<IpAddr> = HashSet::new();
        for ip in &upstream.sans_ip {
            if added_ip.insert(*ip) {
                params.subject_alt_names.push(SanType::IpAddress(*ip));
            }
        }

        // host가 IP 주소인 경우 추가
        if let Ok(ip_addr) = host.parse::<std::net::IpAddr>() {
            if added_ip.insert(ip_addr) {
                params.subject_alt_names.push(SanType::IpAddress(ip_addr));
            }
        }

        info!(
            "Generated {} SAN entries for host '{}' (upstream cert sniffing)",
            params.subject_alt_names.len(),
            host
        );
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

        // IP 주소인 경우 SAN에 추가
        if let Ok(ip_addr) = host.parse::<std::net::IpAddr>() {
            params.subject_alt_names.push(SanType::IpAddress(ip_addr));
            debug!("Added IP SAN: {}", ip_addr);
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

use std::net::IpAddr;

impl CertificateAuthority for RcgenAuthority {
    async fn gen_server_config(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Arc<ServerConfig> {
        if let Some(server_cfg) = self.cache.get(authority).await {
            debug!("Using cached server config for {}", authority);
            return server_cfg;
        }

        info!("🔧 [SERVER-CONFIG] 서버 설정 생성 시작: {}", authority);
        let start_time = std::time::Instant::now();

        info!("🔧 [SERVER-CONFIG] 인증서 생성 중: {}", authority);
        let certs = vec![self.gen_cert(authority, upstream_cert)];
        info!(
            "🔧 [SERVER-CONFIG] 인증서 생성 완료: {} bytes",
            certs[0].len()
        );

        info!("🔧 [SERVER-CONFIG] ServerConfig 빌더 생성 중");

        // TLS 버전 설정: TLS 1.2부터 TLS 1.3까지 허용 (rustls 지원 범위)
        let supported_versions = vec![
            &tokio_rustls::rustls::version::TLS12,
            &tokio_rustls::rustls::version::TLS13,
        ];

        let mut server_cfg = ServerConfig::builder_with_provider(Arc::clone(&self.provider))
            .with_protocol_versions(&supported_versions)
            .expect("Failed to specify protocol versions")
            .with_no_client_auth()
            .with_single_cert(certs, self.private_key.clone_key())
            .expect("Failed to build ServerConfig");

        info!("🔧 [SERVER-CONFIG] ServerConfig 빌더 생성 완료");

        // ALPN 미러링: 상류 서버의 ALPN 협상 결과를 반영
        server_cfg.alpn_protocols = if let Some(ref upstream) = upstream_cert {
            if let Some(ref negotiated) = upstream.negotiated_alpn {
                // 상류 서버가 협상한 프로토콜 우선, 나머지도 포함
                let mut protocols = vec![negotiated.clone()];
                #[cfg(feature = "http2")]
                if negotiated != b"h2" {
                    protocols.push(b"h2".to_vec());
                }
                if negotiated != b"http/1.1" {
                    protocols.push(b"http/1.1".to_vec());
                }
                info!(
                    "🔧 [SERVER-CONFIG] ALPN 미러링 적용: {:?}",
                    protocols
                        .iter()
                        .map(|p| String::from_utf8_lossy(p).to_string())
                        .collect::<Vec<_>>()
                );
                protocols
            } else {
                vec![
                    #[cfg(feature = "http2")]
                    b"h2".to_vec(),
                    b"http/1.1".to_vec(),
                ]
            }
        } else {
            vec![
                #[cfg(feature = "http2")]
                b"h2".to_vec(),
                b"http/1.1".to_vec(),
            ]
        };

        info!(
            "🔧 [SERVER-CONFIG] ALPN 프로토콜 설정: {:?}",
            server_cfg.alpn_protocols
        );

        // 지원되는 TLS 버전 로깅 (rustls 0.23+에서는 다른 방법으로 확인)
        info!("🔧 [SERVER-CONFIG] rustls ServerConfig 생성 완료");

        let server_cfg = Arc::new(server_cfg);
        let duration = start_time.elapsed();

        info!(
            "✅ [SERVER-CONFIG] 서버 설정 생성 완료: {} (소요시간: {:?})",
            authority, duration
        );

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

    async fn is_config_cached(&self, authority: &Authority) -> bool {
        self.cache.get(authority).await.is_some()
    }

    #[cfg(feature = "openssl-ca")]
    async fn gen_openssl_context(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
        // 캐시에서 조회
        if let Some(ctx) = self.openssl_ctx_cache.get(authority).await {
            debug!("[OPENSSL-CONTEXT] 캐시된 컨텍스트 사용: {}", authority);
            return Ok((*ctx).clone());
        }

        info!(
            "[OPENSSL-CONTEXT] OpenSSL 컨텍스트 생성 시작: {}",
            authority
        );

        // spawn_blocking에 전달할 데이터를 미리 준비 (Send 가능한 형태)
        let ca_cert_pem = self.ca_cert.pem();
        let ca_key_pem = self.key_pair.serialize_pem();
        let host = authority.host().to_string();
        let upstream_cert = upstream_cert.cloned();

        // 인증서 생성 + OpenSSL 컨텍스트 빌드를 모두 spawn_blocking으로 오프로드
        let ctx = tokio::task::spawn_blocking(move || -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
            // rcgen 인증서 생성 (CPU 집약적 작업)
            let key_pair = rcgen::KeyPair::from_pem(&ca_key_pem)?;
            let ca_cert_params = rcgen::CertificateParams::from_ca_cert_pem(&ca_cert_pem)?;
            let ca_cert_rcgen = ca_cert_params.self_signed(&key_pair)?;

            let mut params = rcgen::CertificateParams::default();
            params.serial_number = Some(rng().random::<u64>().into());

            let not_before = OffsetDateTime::now_utc() - Duration::seconds(NOT_BEFORE_OFFSET);
            params.not_before = not_before;
            params.not_after = not_before + Duration::seconds(TTL_SECS);

            let mut distinguished_name = DistinguishedName::new();

            if let Some(ref upstream) = upstream_cert {
                if let Some(ref cn) = upstream.common_name {
                    distinguished_name.push(DnType::CommonName, cn);
                } else {
                    distinguished_name.push(DnType::CommonName, &host);
                }
                if let Some(ref org) = upstream.organization {
                    distinguished_name.push(DnType::OrganizationName, org);
                }
            } else {
                distinguished_name.push(DnType::CommonName, &host);
            }
            params.distinguished_name = distinguished_name;

            // SAN 엔트리 추가
            if let Some(ref upstream) = upstream_cert {
                let mut added_dns: HashSet<String> = HashSet::new();
                for dns in &upstream.sans_dns {
                    if added_dns.insert(dns.clone()) {
                        if let Ok(dns_name) = Ia5String::try_from(dns.as_str()) {
                            params.subject_alt_names.push(SanType::DnsName(dns_name));
                        }
                    }
                }
                if added_dns.insert(host.clone()) {
                    if let Ok(dns_name) = Ia5String::try_from(host.as_str()) {
                        params.subject_alt_names.push(SanType::DnsName(dns_name));
                    }
                }
                for ip in &upstream.sans_ip {
                    params.subject_alt_names.push(SanType::IpAddress(*ip));
                }
                if let Ok(ip_addr) = host.parse::<IpAddr>() {
                    if !upstream.sans_ip.contains(&ip_addr) {
                        params.subject_alt_names.push(SanType::IpAddress(ip_addr));
                    }
                }
            } else {
                if let Ok(dns_name) = Ia5String::try_from(host.as_str()) {
                    params.subject_alt_names.push(SanType::DnsName(dns_name));
                }
                if !host.starts_with("*.") {
                    let wildcard = format!("*.{}", host);
                    if let Ok(wildcard_name) = Ia5String::try_from(wildcard.as_str()) {
                        params.subject_alt_names.push(SanType::DnsName(wildcard_name));
                    }
                }
                if let Ok(ip_addr) = host.parse::<IpAddr>() {
                    params.subject_alt_names.push(SanType::IpAddress(ip_addr));
                }
            }

            let server_cert: CertificateDer<'static> = params
                .signed_by(&key_pair, &ca_cert_rcgen, &key_pair)?
                .into();
            let server_cert_der = server_cert.to_vec();

            // OpenSSL 컨텍스트 빌드
            let mut ctx = openssl::ssl::SslContext::builder(openssl::ssl::SslMethod::tls_server())?;

            ctx.set_min_proto_version(Some(openssl::ssl::SslVersion::TLS1))?;
            ctx.set_max_proto_version(Some(openssl::ssl::SslVersion::TLS1_3))?;
            ctx.set_cipher_list("@SECLEVEL=0:ALL:!aNULL:!eNULL")?;

            ctx.set_verify(openssl::ssl::SslVerifyMode::NONE);
            ctx.set_verify_depth(10);

            ctx.set_options(openssl::ssl::SslOptions::NO_COMPRESSION);
            ctx.set_options(openssl::ssl::SslOptions::SINGLE_DH_USE);
            ctx.set_options(openssl::ssl::SslOptions::SINGLE_ECDH_USE);

            let server_cert = openssl::x509::X509::from_der(&server_cert_der)?;
            let ca_cert = openssl::x509::X509::from_pem(ca_cert_pem.as_bytes())?;
            let ca_key = openssl::pkey::PKey::private_key_from_pem(ca_key_pem.as_bytes())?;

            ctx.set_certificate(&server_cert)?;
            ctx.add_extra_chain_cert(ca_cert)?;
            ctx.set_private_key(&ca_key)?;

            Ok(ctx.build())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("spawn_blocking failed: {}", e).into()
        })??;

        // 캐시에 저장
        self.openssl_ctx_cache
            .insert(authority.clone(), Arc::new(ctx.clone()))
            .await;

        info!(
            "[OPENSSL-CONTEXT] OpenSSL 컨텍스트 생성 완료: {}",
            authority
        );
        Ok(ctx)
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

    #[tokio::test]
    async fn gen_openssl_context_returns_valid_context() {
        let ca = build_ca(1_000);
        let authority = Authority::from_static("example.com");

        let ctx = ca.gen_openssl_context(&authority, None).await;
        assert!(ctx.is_ok(), "OpenSSL 컨텍스트 생성 실패: {:?}", ctx.err());
    }

    #[tokio::test]
    async fn gen_openssl_context_cache_hit() {
        let ca = build_ca(1_000);
        let authority = Authority::from_static("cache-test.com");

        let ctx1 = ca.gen_openssl_context(&authority, None).await.unwrap();
        let ctx2 = ca.gen_openssl_context(&authority, None).await.unwrap();

        // 캐시된 컨텍스트의 인증서가 동일한지 확인
        let cert1 = ctx1.certificate().unwrap().to_der().unwrap();
        let cert2 = ctx2.certificate().unwrap().to_der().unwrap();
        assert_eq!(cert1, cert2);
    }

    #[tokio::test]
    async fn gen_openssl_context_concurrent_no_deadlock() {
        let ca = Arc::new(build_ca(1_000));
        let mut handles = Vec::new();

        for i in 0..20 {
            let ca_clone = ca.clone();
            handles.push(tokio::spawn(async move {
                let authority =
                    Authority::try_from(format!("rcgen-concurrent-{}.example.com", i)).unwrap();
                ca_clone
                    .gen_openssl_context(&authority, None)
                    .await
                    .expect("컨텍스트 생성 실패");
            }));
        }

        let result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            for handle in handles {
                handle.await.unwrap();
            }
        })
        .await;

        assert!(result.is_ok(), "데드락 감지: 10초 타임아웃 초과");
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

        let c1 = ca.gen_cert(&authority1, None);
        let c2 = ca.gen_cert(&authority2, None);
        let c3 = ca.gen_cert(&authority1, None);
        let c4 = ca.gen_cert(&authority2, None);

        let (_, cert1) = x509_parser::parse_x509_certificate(&c1).unwrap();
        let (_, cert2) = x509_parser::parse_x509_certificate(&c2).unwrap();

        assert_ne!(cert1.raw_serial(), cert2.raw_serial());

        let (_, cert3) = x509_parser::parse_x509_certificate(&c3).unwrap();
        let (_, cert4) = x509_parser::parse_x509_certificate(&c4).unwrap();

        assert_ne!(cert3.raw_serial(), cert4.raw_serial());

        assert_ne!(cert1.raw_serial(), cert3.raw_serial());
        assert_ne!(cert2.raw_serial(), cert4.raw_serial());
    }

    #[test]
    fn gen_cert_with_upstream_info_uses_upstream_sans() {
        let ca = build_ca(0);
        let authority = Authority::from_static("example.com");

        let upstream = UpstreamCertInfo {
            common_name: Some("Real Example".to_string()),
            organization: Some("Example Inc.".to_string()),
            sans_dns: vec![
                "example.com".to_string(),
                "www.example.com".to_string(),
                "api.example.com".to_string(),
            ],
            sans_ip: vec!["93.184.216.34".parse().unwrap()],
            negotiated_alpn: Some(b"h2".to_vec()),
        };

        let cert_der = ca.gen_cert(&authority, Some(&upstream));
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        // CN이 upstream의 CN인지 확인
        let cn = cert
            .subject()
            .iter()
            .flat_map(|rdn| rdn.iter())
            .find(|attr| *attr.attr_type() == x509_parser::oid_registry::OID_X509_COMMON_NAME)
            .and_then(|attr| attr.as_str().ok())
            .unwrap();
        assert_eq!(cn, "Real Example");

        // SAN에 upstream의 DNS SAN이 포함되는지 확인
        let san = cert.subject_alternative_name().unwrap().unwrap();
        let dns_sans: Vec<&str> = san
            .value
            .general_names
            .iter()
            .filter_map(|name| match name {
                x509_parser::extensions::GeneralName::DNSName(dns) => Some(*dns),
                _ => None,
            })
            .collect();

        assert!(dns_sans.contains(&"example.com"));
        assert!(dns_sans.contains(&"www.example.com"));
        assert!(dns_sans.contains(&"api.example.com"));
    }

    #[test]
    fn gen_cert_without_upstream_falls_back_to_host() {
        let ca = build_ca(0);
        let authority = Authority::from_static("fallback.example.com");

        let cert_der = ca.gen_cert(&authority, None);
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        let cn = cert
            .subject()
            .iter()
            .flat_map(|rdn| rdn.iter())
            .find(|attr| *attr.attr_type() == x509_parser::oid_registry::OID_X509_COMMON_NAME)
            .and_then(|attr| attr.as_str().ok())
            .unwrap();
        assert_eq!(cn, "fallback.example.com");
    }
}
