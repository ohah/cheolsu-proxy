use super::OpensslAuthority;
use crate::certificate_authority::{NOT_BEFORE_OFFSET, TTL_SECS, truncate_cn};
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use openssl::{
    asn1::{Asn1Integer, Asn1Time},
    bn::BigNum,
    error::ErrorStack,
    rand,
    x509::{
        X509Builder, X509NameBuilder,
        extension::{AuthorityKeyIdentifier, ExtendedKeyUsage, SubjectAlternativeName},
    },
};
use std::collections::HashSet;
use std::time::SystemTime;
use tokio_rustls::rustls::pki_types::CertificateDer;

impl OpensslAuthority {
    pub(super) fn gen_cert(
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

        // 실질적으로 발생할 수 없는 경로이지만, expect 대신 안전하게 처리
        let not_before = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| openssl::error::ErrorStack::get())?
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

        // Extended Key Usage (EKU) — macOS Security.framework 등에서 serverAuth 필수
        let eku = ExtendedKeyUsage::new().server_auth().build()?;
        x509_builder.append_extension(eku)?;

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
