use super::RcgenAuthority;
use crate::certificate_authority::{NOT_BEFORE_OFFSET, TTL_SECS, truncate_cn};
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use rand::{Rng, rng};
use rcgen::{
    CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, Ia5String, SanType,
};
use std::collections::HashSet;
use std::net::IpAddr;
use time::{Duration, OffsetDateTime};
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tracing::{debug, error, info, warn};

impl RcgenAuthority {
    /// 인증서와 해당 leaf 키의 PrivateKeyDer를 함께 반환합니다.
    pub(super) fn gen_cert(
        &self,
        authority: &Authority,
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>), rcgen::Error> {
        info!("Generating certificate for authority: {}", authority);

        // upstream 키 타입에 맞는 leaf 키페어 생성
        let leaf_key_pair =
            crate::certificate_authority::generate_rcgen_leaf_key_pair(upstream_cert)
                .unwrap_or_else(|e| {
                    warn!(
                        "[CERT-GEN] leaf 키페어 생성 실패, ECDSA P-256으로 폴백: {:?}",
                        e
                    );
                    crate::certificate_authority::generate_rcgen_leaf_key_pair(None)
                        .expect("ECDSA P-256 키 생성은 실패할 수 없음")
                });

        let leaf_private_key =
            PrivateKeyDer::from(PrivatePkcs8KeyDer::from(leaf_key_pair.serialize_der()));

        let mut params = CertificateParams::default();
        params.serial_number = Some(rng().random::<u64>().into());

        let not_before = OffsetDateTime::now_utc() - Duration::seconds(NOT_BEFORE_OFFSET);
        params.not_before = not_before;
        params.not_after = not_before + Duration::seconds(TTL_SECS);

        let host = authority.host();
        debug!("Certificate host: {}", host);

        let mut distinguished_name = DistinguishedName::new();

        if let Some(upstream) = upstream_cert {
            // upstream 인증서의 CN 사용 (RFC 5280: 64자 제한)
            if let Some(ref cn) = upstream.common_name {
                distinguished_name.push(DnType::CommonName, &truncate_cn(cn));
            } else {
                distinguished_name.push(DnType::CommonName, &truncate_cn(host));
            }
            // upstream 인증서의 Organization 복제
            if let Some(ref org) = upstream.organization {
                distinguished_name.push(DnType::OrganizationName, org);
            }
        } else {
            distinguished_name.push(DnType::CommonName, &truncate_cn(host));
        }

        params.distinguished_name = distinguished_name;
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        // AuthorityKeyIdentifier 추가: Windows SChannel 호환성 개선
        // (mitmproxy #6494: CA와 leaf cert의 SKI가 같으면 SChannel이 오동작)
        params.use_authority_key_identifier_extension = true;

        // SAN 엔트리 설정
        if let Some(upstream) = upstream_cert {
            self.add_upstream_san_entries(&mut params, host, upstream);
        } else {
            self.add_san_entries(&mut params, host);
        }

        // leaf 키페어로 서명 (CA 키페어가 서명자, leaf 키페어가 subject)
        let cert = params
            .signed_by(&leaf_key_pair, &self.ca_cert, &self.key_pair)
            .map_err(|e| {
                error!(authority = %authority, error = ?e, "Failed to sign certificate");
                e
            })?;

        info!(
            "Successfully generated certificate for '{}' (key_type: {:?})",
            authority,
            upstream_cert.map(|u| &u.key_type)
        );
        Ok((cert.into(), leaf_private_key))
    }

    /// 상류 인증서의 SAN 정보를 복제하여 위조 인증서에 추가
    pub(super) fn add_upstream_san_entries(
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
    pub(super) fn add_san_entries(&self, params: &mut CertificateParams, host: &str) {
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
