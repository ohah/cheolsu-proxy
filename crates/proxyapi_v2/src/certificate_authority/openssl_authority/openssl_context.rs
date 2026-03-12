use super::OpensslAuthority;
use crate::certificate_authority::{NOT_BEFORE_OFFSET, TTL_SECS, truncate_cn};
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
use tracing::{debug, info};

pub(super) async fn gen_openssl_context(
    this: &OpensslAuthority,
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

    // 생성자에서 미리 캐시해둔 DER 바이트 사용 (async context에서 OpenSSL 호출 방지)
    let ca_cert_der = this.ca_cert_der.clone();
    let pkey_der = this.pkey_der.clone();
    let host = authority.host().to_string();
    let hash = this.hash;
    let upstream_cert = upstream_cert.cloned();

    // 인증서 생성 + OpenSSL 컨텍스트 빌드를 모두 spawn_blocking으로 오프로드
    let ctx = tokio::task::spawn_blocking(
        move || -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
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
