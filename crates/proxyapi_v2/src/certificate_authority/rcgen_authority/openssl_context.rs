use super::RcgenAuthority;
use crate::certificate_authority::{LEAF_TTL_SECS, NOT_BEFORE_OFFSET, truncate_cn};
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use rand::{Rng, rng};
use rcgen::{DistinguishedName, DnType, Ia5String, SanType};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tokio_rustls::rustls::pki_types::CertificateDer;
use tracing::{debug, info, warn};

pub(super) async fn gen_openssl_context(
    this: &RcgenAuthority,
    authority: &Authority,
    upstream_cert: Option<&UpstreamCertInfo>,
) -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
    // 캐시에서 조회
    if let Some(ctx) = this.openssl_ctx_cache.get(authority).await {
        debug!("[OPENSSL-CONTEXT] 캐시된 컨텍스트 사용: {}", authority);
        return Ok((*ctx).clone());
    }

    info!(
        "[OPENSSL-CONTEXT] OpenSSL 컨텍스트 생성 시작: {}",
        authority
    );

    // spawn_blocking에 전달할 데이터를 미리 준비 (Send 가능한 형태)
    let ca_cert_pem = this.ca_cert.pem();
    let ca_key_pem = this.key_pair.serialize_pem();
    let host = authority.host().to_string();
    let upstream_cert = upstream_cert.cloned();

    // 인증서 생성 + OpenSSL 컨텍스트 빌드를 모두 spawn_blocking으로 오프로드
    let ctx = tokio::task::spawn_blocking(
        move || -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
            // rcgen 인증서 생성 (CPU 집약적 작업)
            let ca_key_pair = rcgen::KeyPair::from_pem(&ca_key_pem)?;
            let ca_cert_params = rcgen::CertificateParams::from_ca_cert_pem(&ca_cert_pem)?;
            let ca_cert_rcgen = ca_cert_params.self_signed(&ca_key_pair)?;

            // upstream 키 타입에 맞는 leaf 키페어 생성
            let leaf_key_pair =
                crate::certificate_authority::generate_rcgen_leaf_key_pair(upstream_cert.as_ref())
                    .unwrap_or_else(|e| {
                        warn!("[OPENSSL-CONTEXT] leaf 키 생성 실패, ECDSA P-256: {:?}", e);
                        crate::certificate_authority::generate_rcgen_leaf_key_pair(None)
                            .expect("ECDSA P-256 키 생성 실패 불가")
                    });

            let mut params = rcgen::CertificateParams::default();
            params.serial_number = Some(rng().random::<u64>().into());

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
            let server_cert: CertificateDer<'static> = params
                .signed_by(&leaf_key_pair, &ca_cert_rcgen, &ca_key_pair)?
                .into();
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
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        format!("spawn_blocking failed: {}", e).into()
    })??;

    // 캐시에 저장
    this.openssl_ctx_cache
        .insert(authority.clone(), Arc::new(ctx.clone()))
        .await;

    info!(
        "[OPENSSL-CONTEXT] OpenSSL 컨텍스트 생성 완료: {}",
        authority
    );
    Ok(ctx)
}
