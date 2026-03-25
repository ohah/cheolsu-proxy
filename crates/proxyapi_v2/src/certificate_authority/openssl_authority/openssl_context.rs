use super::OpensslAuthority;
use crate::certificate_authority::{LEAF_TTL_SECS, NOT_BEFORE_OFFSET, truncate_cn};
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use openssl::{
    asn1::{Asn1Integer, Asn1Time},
    bn::BigNum,
    rand,
    x509::{
        X509Builder, X509NameBuilder,
        extension::{AuthorityKeyIdentifier, SubjectAlternativeName},
    },
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{info, warn};

pub(super) async fn gen_openssl_context(
    this: &OpensslAuthority,
    authority: &Authority,
    upstream_cert: Option<&UpstreamCertInfo>,
) -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
    // 생성자에서 미리 캐시해둔 DER 바이트 사용 (async context에서 OpenSSL 호출 방지)
    let ca_cert_der = this.ca_cert_der.clone();
    let pkey_der = this.pkey_der.clone();
    let host = authority.host().to_string();
    let hash = this.hash;
    let upstream_cert = upstream_cert.cloned();

    // Thundering herd 방지: openssl_ctx_cache에 try_get_with 적용
    let ctx = this
        .openssl_ctx_cache
        .try_get_with(authority.clone(), async {
            build_openssl_context_inner(ca_cert_der, pkey_der, host, hash, upstream_cert).await
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::from(e.to_string()) })?;

    Ok((*ctx).clone())
}

async fn build_openssl_context_inner(
    ca_cert_der: Vec<u8>,
    pkey_der: Vec<u8>,
    host: String,
    hash: openssl::hash::MessageDigest,
    upstream_cert: Option<UpstreamCertInfo>,
) -> Result<Arc<openssl::ssl::SslContext>, String> {
    info!("[OPENSSL-CONTEXT] OpenSSL 컨텍스트 생성 시작: {}", host);

    let ctx = tokio::task::spawn_blocking(
        move || -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
            // CA 키 및 인증서 로드
            let ca_pkey = openssl::pkey::PKey::private_key_from_der(&pkey_der)?;
            let ca_cert = openssl::x509::X509::from_der(&ca_cert_der)?;

            // upstream 키 타입에 맞는 leaf 키페어 생성
            let leaf_pkey =
                crate::certificate_authority::generate_openssl_leaf_pkey(upstream_cert.as_ref())
                    .unwrap_or_else(|e| {
                        warn!(
                            "[OPENSSL-CONTEXT] leaf 키 생성 실패, 기본 ECDSA P-256: {:?}",
                            e
                        );
                        crate::certificate_authority::generate_openssl_leaf_pkey(None)
                            .expect("기본 ECDSA P-256 키 생성은 실패할 수 없음")
                    });

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
                if let Some(ref country) = upstream.country {
                    name_builder.append_entry_by_text("C", country)?;
                }
                if let Some(ref state) = upstream.state {
                    name_builder.append_entry_by_text("ST", state)?;
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
            x509_builder
                .set_not_after(Asn1Time::from_unix(not_before + LEAF_TTL_SECS)?.as_ref())?;

            x509_builder.set_pubkey(&leaf_pkey)?;
            x509_builder.set_issuer_name(ca_cert.subject_name())?;

            // Authority Key Identifier (AKI) 추가
            // keyid(false): CA에 SKI가 없어도 에러를 발생시키지 않음
            if let Ok(aki) = AuthorityKeyIdentifier::new()
                .keyid(false)
                .build(&x509_builder.x509v3_context(Some(&ca_cert), None))
            {
                let _ = x509_builder.append_extension(aki);
            }

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

            x509_builder.sign(&ca_pkey, hash)?;
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
            ctx.set_private_key(&leaf_pkey)?;

            Ok(ctx.build())
        },
    )
    .await
    .map_err(|e| format!("spawn_blocking failed: {}", e))?
    .map_err(|e| e.to_string())?;

    info!("[OPENSSL-CONTEXT] OpenSSL 컨텍스트 생성 완료");

    Ok(Arc::new(ctx))
}
