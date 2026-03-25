use super::RcgenAuthority;
use crate::certificate_authority::{LEAF_TTL_SECS, NOT_BEFORE_OFFSET, truncate_cn};
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use rcgen::{DistinguishedName, DnType, SanType, string::Ia5String};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tokio_rustls::rustls::pki_types::CertificateDer;
use tracing::{info, warn};

pub(super) async fn gen_openssl_context(
    this: &RcgenAuthority,
    authority: &Authority,
    upstream_cert: Option<&UpstreamCertInfo>,
) -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
    // 캐시 히트 시 불필요한 clone을 피하기 위해 먼저 조회
    if let Some(ctx) = this.openssl_ctx_cache.get(authority).await {
        return Ok((*ctx).clone());
    }

    // 캐시 미스 시에만 데이터 clone (spawn_blocking에 전달할 Send 가능한 형태)
    let ca_cert_pem = this.ca_cert_pem.clone();
    let ca_key_pem = this.ca_key_pem.clone();
    let host = authority.host().to_string();
    let upstream_cert = upstream_cert.cloned();

    // Thundering herd 방지: try_get_with로 동일 authority에 대한 중복 생성 방지
    let ctx = this
        .openssl_ctx_cache
        .try_get_with(authority.clone(), async {
            build_openssl_context(ca_cert_pem, ca_key_pem, host, upstream_cert).await
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::from(e.to_string()) })?;

    Ok((*ctx).clone())
}

async fn build_openssl_context(
    ca_cert_pem: String,
    ca_key_pem: String,
    host: String,
    upstream_cert: Option<UpstreamCertInfo>,
) -> Result<Arc<openssl::ssl::SslContext>, String> {
    info!("[OPENSSL-CONTEXT] OpenSSL 컨텍스트 생성 시작: {}", host);

    let ctx = tokio::task::spawn_blocking(
        move || -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
            // rcgen 인증서 생성 (CPU 집약적 작업)
            let ca_key_pair = rcgen::KeyPair::from_pem(&ca_key_pem)?;
            let ca_issuer = rcgen::Issuer::from_ca_cert_pem(&ca_cert_pem, ca_key_pair)?;

            // upstream 키 타입에 맞는 leaf 키페어 생성
            let leaf_key_pair =
                crate::certificate_authority::generate_rcgen_leaf_key_pair(upstream_cert.as_ref())
                    .unwrap_or_else(|e| {
                        warn!("[OPENSSL-CONTEXT] leaf 키 생성 실패, ECDSA P-256: {:?}", e);
                        crate::certificate_authority::generate_rcgen_leaf_key_pair(None)
                            .expect("ECDSA P-256 키 생성 실패 불가")
                    });

            let mut params = rcgen::CertificateParams::default();
            params.serial_number = Some(crate::certificate_authority::generate_serial_number());

            let not_before = OffsetDateTime::now_utc() - Duration::seconds(NOT_BEFORE_OFFSET);
            params.not_before = not_before;
            params.not_after = not_before + Duration::seconds(LEAF_TTL_SECS);

            let mut distinguished_name = DistinguishedName::new();

            if let Some(ref upstream) = upstream_cert {
                if let Some(ref cn) = upstream.common_name {
                    distinguished_name.push(DnType::CommonName, &truncate_cn(cn));
                } else {
                    distinguished_name.push(DnType::CommonName, &truncate_cn(&host));
                }
                if let Some(ref org) = upstream.organization {
                    distinguished_name.push(DnType::OrganizationName, org);
                }
                if let Some(ref country) = upstream.country {
                    distinguished_name.push(DnType::CountryName, country);
                }
                if let Some(ref state) = upstream.state {
                    distinguished_name.push(DnType::StateOrProvinceName, state);
                }
            } else {
                distinguished_name.push(DnType::CommonName, &truncate_cn(&host));
            }
            params.distinguished_name = distinguished_name;
            // AuthorityKeyIdentifier 추가: Windows SChannel 호환성 개선
            params.use_authority_key_identifier_extension = true;
            params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];

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
                        params
                            .subject_alt_names
                            .push(SanType::DnsName(wildcard_name));
                    }
                }
                if let Ok(ip_addr) = host.parse::<IpAddr>() {
                    params.subject_alt_names.push(SanType::IpAddress(ip_addr));
                }
            }

            // leaf 키로 서명 (CA 키가 서명자)
            let server_cert: CertificateDer<'static> =
                params.signed_by(&leaf_key_pair, &ca_issuer)?.into();
            let server_cert_der = server_cert.to_vec();
            let leaf_key_pem = leaf_key_pair.serialize_pem();

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
            let leaf_key = openssl::pkey::PKey::private_key_from_pem(leaf_key_pem.as_bytes())?;

            ctx.set_certificate(&server_cert)?;
            ctx.add_extra_chain_cert(ca_cert)?;
            ctx.set_private_key(&leaf_key)?;

            Ok(ctx.build())
        },
    )
    .await
    .map_err(|e| format!("spawn_blocking failed: {}", e))?
    .map_err(|e| e.to_string())?;

    info!("[OPENSSL-CONTEXT] OpenSSL 컨텍스트 생성 완료");

    Ok(Arc::new(ctx))
}
