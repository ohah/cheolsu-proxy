use super::OpensslAuthority;
use crate::certificate_authority::{NOT_BEFORE_OFFSET, TTL_SECS, truncate_cn};
use crate::upstream_cert::{EcCurve, UpstreamCertInfo, UpstreamKeyType};
use http::uri::Authority;
use openssl::{
    asn1::{Asn1Integer, Asn1Time},
    bn::BigNum,
    ec::{EcGroup, EcKey},
    error::ErrorStack,
    nid::Nid,
    pkey::{PKey, Private},
    rand,
    x509::{
        X509Builder, X509NameBuilder,
        extension::{AuthorityKeyIdentifier, ExtendedKeyUsage, SubjectAlternativeName},
    },
};
use std::collections::HashSet;
use std::time::SystemTime;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tracing::{info, warn};

impl OpensslAuthority {
    /// upstream 키 타입에 맞는 leaf 키페어를 동적 생성합니다.
    fn generate_leaf_pkey(
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<PKey<Private>, ErrorStack> {
        let key_type = upstream_cert.map(|u| &u.key_type);

        match key_type {
            Some(UpstreamKeyType::Rsa(bits)) => {
                let bits = match *bits {
                    b if b >= 4096 => 4096,
                    b if b >= 2048 => 2048,
                    _ => 2048,
                };
                info!("[CERT-GEN] RSA-{} leaf 키페어 생성", bits);
                let rsa = openssl::rsa::Rsa::generate(bits)?;
                PKey::from_rsa(rsa)
            }
            Some(UpstreamKeyType::Ecdsa(curve)) => {
                let nid = match curve {
                    EcCurve::P256 => Nid::X9_62_PRIME256V1,
                    EcCurve::P384 => Nid::SECP384R1,
                    EcCurve::P521 => Nid::SECP521R1,
                };
                info!("[CERT-GEN] ECDSA {:?} leaf 키페어 생성", curve);
                let group = EcGroup::from_curve_name(nid)?;
                let ec_key = EcKey::generate(&group)?;
                PKey::from_ec_key(ec_key)
            }
            Some(UpstreamKeyType::Ed25519) => {
                info!("[CERT-GEN] Ed25519 leaf 키페어 생성");
                PKey::generate_ed25519()
            }
            Some(UpstreamKeyType::Unknown) | None => {
                // 기본값: ECDSA P-256
                info!("[CERT-GEN] ECDSA P-256 leaf 키페어 생성 (기본값)");
                let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
                let ec_key = EcKey::generate(&group)?;
                PKey::from_ec_key(ec_key)
            }
        }
    }

    /// 인증서 서명에 사용할 해시 알고리즘을 키 타입에 맞게 결정합니다.
    fn hash_for_key(
        pkey: &PKey<Private>,
        default_hash: openssl::hash::MessageDigest,
    ) -> openssl::hash::MessageDigest {
        // Ed25519는 자체 해시를 사용하므로 None 전달 필요 (sign에서 처리)
        // RSA/ECDSA는 SHA-256 사용
        if pkey.id() == openssl::pkey::Id::ED25519 {
            // Ed25519는 sign()에 None을 전달해야 하지만, openssl crate에서는
            // MessageDigest를 무시하므로 기본값을 전달해도 안전
            default_hash
        } else {
            default_hash
        }
    }

    pub(super) fn gen_cert(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>), ErrorStack> {
        let host = authority.host();

        // upstream 키 타입에 맞는 leaf 키페어 생성
        let leaf_pkey = Self::generate_leaf_pkey(upstream_cert).unwrap_or_else(|e| {
            warn!(
                "[CERT-GEN] leaf 키페어 생성 실패, 기본 ECDSA P-256으로 폴백: {:?}",
                e
            );
            let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
            let ec_key = EcKey::generate(&group).unwrap();
            PKey::from_ec_key(ec_key).unwrap()
        });
        let leaf_private_key_der = leaf_pkey.private_key_to_pkcs8()?;
        let leaf_private_key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(leaf_private_key_der));

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

        x509_builder.set_pubkey(&leaf_pkey)?;
        x509_builder.set_issuer_name(self.ca_cert.subject_name())?;

        // Authority Key Identifier (AKI) 추가 — CA 공개키 해시로 발급자 식별
        // keyid(false): CA에 SKI가 없어도 에러를 발생시키지 않음
        if let Ok(aki) = AuthorityKeyIdentifier::new()
            .keyid(false)
            .build(&x509_builder.x509v3_context(Some(&self.ca_cert), None))
        {
            let _ = x509_builder.append_extension(aki);
        }

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

        // CA 키로 서명 (leaf 키가 아닌 CA 키로 서명해야 체인이 성립)
        let sign_hash = Self::hash_for_key(&self.pkey, self.hash);
        x509_builder.sign(&self.pkey, sign_hash)?;
        let x509 = x509_builder.build();
        Ok((CertificateDer::from(x509.to_der()?), leaf_private_key))
    }
}
