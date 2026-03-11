use crate::certificate_authority::{
    CACHE_TTL, CertificateAuthority, NOT_BEFORE_OFFSET, TTL_SECS, truncate_cn,
};
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use moka::future::Cache;
use openssl::{
    asn1::{Asn1Integer, Asn1Time},
    bn::BigNum,
    error::ErrorStack,
    hash::MessageDigest,
    pkey::{PKey, Private},
    rand,
    x509::{
        X509, X509Builder, X509NameBuilder,
        extension::{AuthorityKeyIdentifier, SubjectAlternativeName},
    },
};
use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio_rustls::rustls::{
    ServerConfig,
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
};
use tracing::{debug, info};

/// Issues certificates for use when communicating with clients.
///
/// Issues certificates for communicating with clients over TLS. Certificates are cached in memory
/// up to a max size that is provided when creating the authority. Certificates are generated using
/// the `openssl` crate.
///
/// # Examples
///
/// ```rust
/// use proxyapi_v2::{
///     certificate_authority::OpensslAuthority,
///     openssl::{hash::MessageDigest, pkey::PKey, x509::X509},
///     rustls::crypto::aws_lc_rs,
/// };
///
/// let private_key_bytes: &[u8] = include_bytes!("../../examples/ca/hudsucker.key");
/// let ca_cert_bytes: &[u8] = include_bytes!("../../examples/ca/hudsucker.cer");
/// let private_key = PKey::private_key_from_pem(private_key_bytes).unwrap();
/// let ca_cert = X509::from_pem(ca_cert_bytes).unwrap();
///
/// let ca = OpensslAuthority::new(
///     private_key,
///     ca_cert,
///     MessageDigest::sha256(),
///     1_000,
///     aws_lc_rs::default_provider(),
/// );
/// ```
pub struct OpensslAuthority {
    pkey: PKey<Private>,
    private_key: PrivateKeyDer<'static>,
    ca_cert: X509,
    /// DER 형태의 CA 인증서 (spawn_blocking 전달용, 생성자에서 미리 캐시)
    ca_cert_der: Vec<u8>,
    /// DER 형태의 개인키 (spawn_blocking 전달용, 생성자에서 미리 캐시)
    pkey_der: Vec<u8>,
    hash: MessageDigest,
    cache: Cache<Authority, Arc<ServerConfig>>,
    #[cfg(feature = "openssl-ca")]
    openssl_ctx_cache: Cache<Authority, Arc<openssl::ssl::SslContext>>,
    provider: Arc<CryptoProvider>,
}

impl OpensslAuthority {
    /// Creates a new openssl authority.
    pub fn new(
        pkey: PKey<Private>,
        ca_cert: X509,
        hash: MessageDigest,
        cache_size: u64,
        provider: CryptoProvider,
    ) -> Self {
        let private_key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(
            pkey.private_key_to_pkcs8()
                .expect("Failed to encode private key"),
        ));
        let ca_cert_der = ca_cert.to_der().expect("Failed to encode CA cert to DER");
        let pkey_der = pkey
            .private_key_to_der()
            .expect("Failed to encode private key to DER");

        Self {
            pkey,
            private_key,
            ca_cert,
            ca_cert_der,
            pkey_der,
            hash,
            cache: Cache::builder()
                .max_capacity(cache_size)
                .time_to_live(Duration::from_secs(CACHE_TTL))
                .build(),
            #[cfg(feature = "openssl-ca")]
            openssl_ctx_cache: Cache::builder()
                .max_capacity(cache_size)
                .time_to_live(Duration::from_secs(CACHE_TTL))
                .build(),
            provider: Arc::new(provider),
        }
    }

    fn gen_cert(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<CertificateDer<'static>, ErrorStack> {
        let host = authority.host();

        // CN 설정 - upstream 인증서 정보 우선 사용 (RFC 5280: 64자 제한)
        let mut name_builder = X509NameBuilder::new()?;
        if let Some(upstream) = upstream_cert {
            if let Some(ref cn) = upstream.common_name {
                name_builder.append_entry_by_text("CN", &truncate_cn(cn))?;
            } else {
                name_builder.append_entry_by_text("CN", &truncate_cn(host))?;
            }
            if let Some(ref org) = upstream.organization {
                name_builder.append_entry_by_text("O", org)?;
            }
        } else {
            name_builder.append_entry_by_text("CN", &truncate_cn(host))?;
        }
        let name = name_builder.build();

        let mut x509_builder = X509Builder::new()?;
        x509_builder.set_subject_name(&name)?;
        x509_builder.set_version(2)?;

        let not_before = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to determine current UNIX time")
            .as_secs() as i64
            - NOT_BEFORE_OFFSET;
        x509_builder.set_not_before(Asn1Time::from_unix(not_before)?.as_ref())?;
        x509_builder.set_not_after(Asn1Time::from_unix(not_before + TTL_SECS)?.as_ref())?;

        x509_builder.set_pubkey(&self.pkey)?;
        x509_builder.set_issuer_name(self.ca_cert.subject_name())?;

        // Authority Key Identifier (AKI) 추가 — CA 공개키 해시로 발급자 식별
        let aki = AuthorityKeyIdentifier::new()
            .keyid(true)
            .build(&x509_builder.x509v3_context(Some(&self.ca_cert), None))?;
        x509_builder.append_extension(aki)?;

        // SAN 설정 - upstream 인증서 정보 우선 사용
        // RFC 5280 4.2.1.6: subject가 비어있을 때만 SAN을 critical로 설정
        let mut san_builder = SubjectAlternativeName::new();
        if name.entries().count() == 0 {
            san_builder.critical();
        }
        if let Some(upstream) = upstream_cert {
            let mut added_dns: HashSet<String> = HashSet::new();
            for dns in &upstream.sans_dns {
                if added_dns.insert(dns.clone()) {
                    san_builder.dns(dns);
                }
            }
            if added_dns.insert(host.to_string()) {
                san_builder.dns(host);
            }
            for ip in &upstream.sans_ip {
                san_builder.ip(&ip.to_string());
            }
        } else {
            san_builder.dns(host);
        }

        let alternative_name =
            san_builder.build(&x509_builder.x509v3_context(Some(&self.ca_cert), None))?;
        x509_builder.append_extension(alternative_name)?;

        let mut serial_number = [0; 16];
        rand::rand_bytes(&mut serial_number)?;

        let serial_number = BigNum::from_slice(&serial_number)?;
        let serial_number = Asn1Integer::from_bn(&serial_number)?;
        x509_builder.set_serial_number(&serial_number)?;

        x509_builder.sign(&self.pkey, self.hash)?;
        let x509 = x509_builder.build();
        Ok(CertificateDer::from(x509.to_der()?))
    }
}

impl CertificateAuthority for OpensslAuthority {
    async fn gen_server_config(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Arc<ServerConfig> {
        if let Some(server_cfg) = self.cache.get(authority).await {
            debug!("Using cached server config");
            return server_cfg;
        }
        debug!("Generating server config");

        let certs = vec![
            self.gen_cert(authority, upstream_cert)
                .unwrap_or_else(|_| panic!("Failed to generate certificate for {}", authority)),
        ];

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

        // ALPN 미러링: 상류 서버의 ALPN 협상 결과를 반영
        server_cfg.alpn_protocols = if let Some(ref upstream) = upstream_cert {
            if let Some(ref negotiated) = upstream.negotiated_alpn {
                let mut protocols = vec![negotiated.clone()];
                #[cfg(feature = "http2")]
                if negotiated != b"h2" {
                    protocols.push(b"h2".to_vec());
                }
                if negotiated != b"http/1.1" {
                    protocols.push(b"http/1.1".to_vec());
                }
                info!(
                    "[SERVER-CONFIG] ALPN 미러링 적용: {:?}",
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

        let server_cfg = Arc::new(server_cfg);

        self.cache
            .insert(authority.clone(), Arc::clone(&server_cfg))
            .await;

        server_cfg
    }

    fn get_ca_cert_der(&self) -> Option<Vec<u8>> {
        // OpenSSL X509 인증서를 DER 형식으로 변환
        self.ca_cert.to_der().ok()
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

        // 생성자에서 미리 캐시해둔 DER 바이트 사용 (async context에서 OpenSSL 호출 방지)
        let ca_cert_der = self.ca_cert_der.clone();
        let pkey_der = self.pkey_der.clone();
        let host = authority.host().to_string();
        let hash = self.hash;
        let upstream_cert = upstream_cert.cloned();

        // 인증서 생성 + OpenSSL 컨텍스트 빌드를 모두 spawn_blocking으로 오프로드
        let ctx = tokio::task::spawn_blocking(move || -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
            // 인증서 생성
            let pkey = openssl::pkey::PKey::private_key_from_der(&pkey_der)?;
            let ca_cert = openssl::x509::X509::from_der(&ca_cert_der)?;

            let mut name_builder = X509NameBuilder::new()?;
            if let Some(ref upstream) = upstream_cert {
                if let Some(ref cn) = upstream.common_name {
                    name_builder.append_entry_by_text("CN", &truncate_cn(cn))?;
                } else {
                    name_builder.append_entry_by_text("CN", &truncate_cn(&host))?;
                }
                if let Some(ref org) = upstream.organization {
                    name_builder.append_entry_by_text("O", org)?;
                }
            } else {
                name_builder.append_entry_by_text("CN", &truncate_cn(&host))?;
            }
            let name = name_builder.build();

            let mut x509_builder = X509Builder::new()?;
            x509_builder.set_subject_name(&name)?;
            x509_builder.set_version(2)?;

            let not_before = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Failed to determine current UNIX time")
                .as_secs() as i64
                - NOT_BEFORE_OFFSET;
            x509_builder.set_not_before(Asn1Time::from_unix(not_before)?.as_ref())?;
            x509_builder.set_not_after(Asn1Time::from_unix(not_before + TTL_SECS)?.as_ref())?;

            x509_builder.set_pubkey(&pkey)?;
            x509_builder.set_issuer_name(ca_cert.subject_name())?;

            // Authority Key Identifier (AKI) 추가
            let aki = AuthorityKeyIdentifier::new()
                .keyid(true)
                .build(&x509_builder.x509v3_context(Some(&ca_cert), None))?;
            x509_builder.append_extension(aki)?;

            // SAN 설정 (RFC 5280: subject 비어있을 때만 critical)
            let mut san_builder = SubjectAlternativeName::new();
            if name.entries().count() == 0 {
                san_builder.critical();
            }
            if let Some(ref upstream) = upstream_cert {
                let mut added_dns: HashSet<String> = HashSet::new();
                for dns in &upstream.sans_dns {
                    if added_dns.insert(dns.clone()) {
                        san_builder.dns(dns);
                    }
                }
                if added_dns.insert(host.clone()) {
                    san_builder.dns(&host);
                }
                for ip in &upstream.sans_ip {
                    san_builder.ip(&ip.to_string());
                }
            } else {
                san_builder.dns(&host);
            }

            let alternative_name =
                san_builder.build(&x509_builder.x509v3_context(Some(&ca_cert), None))?;
            x509_builder.append_extension(alternative_name)?;

            let mut serial_bytes = [0u8; 16];
            rand::rand_bytes(&mut serial_bytes)?;
            let serial_bn = BigNum::from_slice(&serial_bytes)?;
            let serial_number = Asn1Integer::from_bn(&serial_bn)?;
            x509_builder.set_serial_number(&serial_number)?;

            x509_builder.sign(&pkey, hash)?;
            let server_cert = x509_builder.build();

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

            ctx.set_certificate(&server_cert)?;
            ctx.add_extra_chain_cert(ca_cert)?;
            ctx.set_private_key(&pkey)?;

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

    fn build_ca(cache_size: u64) -> OpensslAuthority {
        let private_key_bytes: &[u8] = include_bytes!("cheolsu-proxy.key");
        let ca_cert_bytes: &[u8] = include_bytes!("cheolsu-proxy.cer");
        let private_key =
            PKey::private_key_from_pem(private_key_bytes).expect("Failed to parse private key");
        let ca_cert = X509::from_pem(ca_cert_bytes).expect("Failed to parse CA certificate");

        OpensslAuthority::new(
            private_key,
            ca_cert,
            MessageDigest::sha256(),
            cache_size,
            aws_lc_rs::default_provider(),
        )
    }

    #[tokio::test]
    async fn gen_openssl_context_returns_valid_context() {
        let ca = build_ca(1_000);
        let authority = Authority::from_static("example.com");

        let ctx = ca.gen_openssl_context(&authority, None).await;
        assert!(ctx.is_ok(), "OpenSSL 컨텍스트 생성 실패: {:?}", ctx.err());
    }

    #[tokio::test]
    async fn gen_openssl_context_cache_returns_same_result() {
        let ca = build_ca(1_000);
        let authority = Authority::from_static("cache-test.com");

        // 첫 번째 호출 - 캐시 미스
        let ctx1 = ca.gen_openssl_context(&authority, None).await.unwrap();
        // 두 번째 호출 - 캐시 히트
        let ctx2 = ca.gen_openssl_context(&authority, None).await.unwrap();

        // 캐시된 컨텍스트의 인증서가 동일한지 확인
        let cert1 = ctx1.certificate().unwrap().to_der().unwrap();
        let cert2 = ctx2.certificate().unwrap().to_der().unwrap();
        assert_eq!(cert1, cert2);
    }

    #[tokio::test]
    async fn gen_openssl_context_different_authorities_have_different_certs() {
        let ca = build_ca(1_000);
        let auth1 = Authority::from_static("example1.com");
        let auth2 = Authority::from_static("example2.com");

        let ctx1 = ca.gen_openssl_context(&auth1, None).await.unwrap();
        let ctx2 = ca.gen_openssl_context(&auth2, None).await.unwrap();

        // 다른 authority에 대해 다른 인증서가 설정되어야 함
        let cert1 = ctx1.certificate().expect("cert1 없음");
        let cert2 = ctx2.certificate().expect("cert2 없음");

        // CN이 다른 도메인이어야 함
        let cn1 = cert1
            .subject_name()
            .entries_by_nid(openssl::nid::Nid::COMMONNAME)
            .next()
            .unwrap()
            .data()
            .as_utf8()
            .unwrap()
            .to_string();
        let cn2 = cert2
            .subject_name()
            .entries_by_nid(openssl::nid::Nid::COMMONNAME)
            .next()
            .unwrap()
            .data()
            .as_utf8()
            .unwrap()
            .to_string();

        assert_eq!(cn1, "example1.com");
        assert_eq!(cn2, "example2.com");
    }

    #[tokio::test]
    async fn gen_openssl_context_concurrent_no_deadlock() {
        let ca = Arc::new(build_ca(1_000));
        let mut handles = Vec::new();

        for i in 0..20 {
            let ca_clone = ca.clone();
            handles.push(tokio::spawn(async move {
                let authority =
                    Authority::try_from(format!("concurrent-{}.example.com", i)).unwrap();
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

    #[tokio::test]
    async fn gen_openssl_context_concurrent_same_authority_uses_cache() {
        let ca = Arc::new(build_ca(1_000));
        let authority = Authority::from_static("shared.example.com");
        let mut handles = Vec::new();

        for _ in 0..10 {
            let ca_clone = ca.clone();
            let auth_clone = authority.clone();
            handles.push(tokio::spawn(async move {
                ca_clone
                    .gen_openssl_context(&auth_clone, None)
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

        let authority1 = Authority::from_static("example.com");
        let authority2 = Authority::from_static("example2.com");

        let c1 = ca.gen_cert(&authority1, None).unwrap();
        let c2 = ca.gen_cert(&authority2, None).unwrap();
        let c3 = ca.gen_cert(&authority1, None).unwrap();
        let c4 = ca.gen_cert(&authority2, None).unwrap();

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
    fn gen_cert_with_upstream_info() {
        let ca = build_ca(0);
        let authority = Authority::from_static("example.com");

        let upstream = UpstreamCertInfo {
            common_name: Some("Real Server".to_string()),
            organization: Some("Real Org".to_string()),
            sans_dns: vec!["example.com".to_string(), "www.example.com".to_string()],
            sans_ip: vec![],
            negotiated_alpn: Some(b"h2".to_vec()),
        };

        let cert_der = ca.gen_cert(&authority, Some(&upstream)).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        // CN 확인
        let cn = cert
            .subject()
            .iter()
            .flat_map(|rdn| rdn.iter())
            .find(|attr| *attr.attr_type() == x509_parser::oid_registry::OID_X509_COMMON_NAME)
            .and_then(|attr| attr.as_str().ok())
            .unwrap();
        assert_eq!(cn, "Real Server");
    }

    #[test]
    fn gen_cert_includes_authority_key_identifier() {
        let ca = build_ca(0);
        let authority = Authority::from_static("aki-test.example.com");

        let cert_der = ca.gen_cert(&authority, None).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        // AKI extension (OID 2.5.29.35) 이 존재하는지 확인
        let aki = cert.extensions().iter().find(|ext| {
            ext.oid == x509_parser::oid_registry::OID_X509_EXT_AUTHORITY_KEY_IDENTIFIER
        });
        assert!(
            aki.is_some(),
            "Authority Key Identifier extension이 없습니다"
        );
    }

    #[test]
    fn gen_cert_cn_truncated_to_64_chars() {
        let ca = build_ca(0);
        let long_host = format!("{}.example.com", "a".repeat(80));
        let authority = Authority::try_from(long_host).unwrap();

        let cert_der = ca.gen_cert(&authority, None).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        let cn = cert
            .subject()
            .iter()
            .flat_map(|rdn| rdn.iter())
            .find(|attr| *attr.attr_type() == x509_parser::oid_registry::OID_X509_COMMON_NAME)
            .and_then(|attr| attr.as_str().ok())
            .unwrap();
        assert!(cn.len() <= 64, "CN이 64자를 초과합니다: {} chars", cn.len());
    }
}
