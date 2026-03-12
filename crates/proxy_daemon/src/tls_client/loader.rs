use sha2::{Digest, Sha256};
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use x509_parser::extensions::GeneralName;
use x509_parser::oid_registry::{OID_X509_COMMON_NAME, OID_X509_ORGANIZATION_NAME};

use crate::protocol::CertificateInfo;

/// 인증서 체인을 로드합니다. PEM 형식을 먼저 시도하고, 실패 시 DER 형식으로 시도합니다.
pub fn load_certs(
    cert_path: &str,
) -> Result<Vec<CertificateDer<'static>>, Box<dyn std::error::Error>> {
    let cert_data = std::fs::read(cert_path)?;

    // PEM 형식 시도
    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut cert_data.as_slice()).collect::<Result<Vec<_>, _>>()?;

    if certs.is_empty() {
        // DER 형식으로 시도 - 최소한 ASN.1 시퀀스 태그(0x30)인지 확인
        if cert_data.first().copied() != Some(0x30) {
            return Err("인증서 파일이 유효한 PEM 또는 DER 형식이 아닙니다".into());
        }
        return Ok(vec![CertificateDer::from(cert_data)]);
    }

    Ok(certs)
}

/// 인증서 파일에서 상세 정보를 추출합니다.
pub fn parse_certificate_info(
    cert_path: &str,
) -> Result<CertificateInfo, Box<dyn std::error::Error>> {
    let certs = load_certs(cert_path)?;
    if certs.is_empty() {
        return Err("인증서 파일에서 인증서를 찾을 수 없습니다".into());
    }

    let chain_length = certs.len();
    let cert_der = certs[0].as_ref();

    let (_, cert) = x509_parser::parse_x509_certificate(cert_der)
        .map_err(|e| format!("인증서 파싱 실패: {}", e))?;

    Ok(parse_x509_to_certificate_info(
        &cert,
        cert_der,
        chain_length,
    ))
}

/// 인증서 바이트(PEM 또는 DER)에서 상세 정보를 추출합니다.
pub fn parse_certificate_info_from_bytes(
    cert_bytes: &[u8],
) -> Result<CertificateInfo, Box<dyn std::error::Error>> {
    // PEM이면 DER로 변환
    let der_data = if cert_bytes.starts_with(b"-----BEGIN") {
        let certs: Vec<CertificateDer<'static>> =
            rustls_pemfile::certs(&mut cert_bytes.as_ref()).collect::<Result<Vec<_>, _>>()?;
        if certs.is_empty() {
            return Err("PEM 데이터에서 인증서를 찾을 수 없습니다".into());
        }
        certs[0].to_vec()
    } else {
        cert_bytes.to_vec()
    };

    let (_, cert) = x509_parser::parse_x509_certificate(&der_data)
        .map_err(|e| format!("인증서 파싱 실패: {}", e))?;

    Ok(parse_x509_to_certificate_info(&cert, &der_data, 1))
}

/// PEM 파일에서 개인 키를 로드합니다. RSA (PKCS#1), PKCS#8, EC (SEC1) 키를 지원합니다.
/// 현재 DER 형식 키는 지원하지 않습니다.
pub fn load_private_key(
    key_path: &str,
) -> Result<PrivateKeyDer<'static>, Box<dyn std::error::Error>> {
    let key_data = std::fs::read(key_path)?;
    let mut reader = key_data.as_slice();

    // rustls_pemfile를 사용하여 다양한 PEM 키 형식 파싱
    loop {
        match rustls_pemfile::read_one(&mut reader)? {
            Some(rustls_pemfile::Item::Pkcs1Key(key)) => {
                return Ok(PrivateKeyDer::Pkcs1(key));
            }
            Some(rustls_pemfile::Item::Pkcs8Key(key)) => {
                return Ok(PrivateKeyDer::Pkcs8(key));
            }
            Some(rustls_pemfile::Item::Sec1Key(key)) => {
                return Ok(PrivateKeyDer::Sec1(key));
            }
            Some(_) => continue, // 다른 PEM 항목은 건너뜀
            None => break,
        }
    }

    Err("키 파일에서 유효한 개인 키를 찾을 수 없습니다".into())
}

/// x509_parser::X509Certificate에서 CertificateInfo를 추출합니다.
pub(super) fn parse_x509_to_certificate_info(
    cert: &x509_parser::certificate::X509Certificate<'_>,
    cert_der: &[u8],
    chain_length: usize,
) -> CertificateInfo {
    let subject_cn = cert
        .subject()
        .iter()
        .flat_map(|rdn| rdn.iter())
        .find(|attr| *attr.attr_type() == OID_X509_COMMON_NAME)
        .and_then(|attr| attr.as_str().ok())
        .map(|s| s.to_string());

    let issuer_cn = cert
        .issuer()
        .iter()
        .flat_map(|rdn| rdn.iter())
        .find(|attr| *attr.attr_type() == OID_X509_COMMON_NAME)
        .and_then(|attr| attr.as_str().ok())
        .map(|s| s.to_string());

    let organization = cert
        .subject()
        .iter()
        .flat_map(|rdn| rdn.iter())
        .find(|attr| *attr.attr_type() == OID_X509_ORGANIZATION_NAME)
        .and_then(|attr| attr.as_str().ok())
        .map(|s| s.to_string());

    // SAN 추출
    let mut sans_dns = Vec::new();
    let mut sans_ip = Vec::new();
    if let Ok(Some(san)) = cert.subject_alternative_name() {
        for name in &san.value.general_names {
            match name {
                GeneralName::DNSName(dns) => sans_dns.push(dns.to_string()),
                GeneralName::IPAddress(ip_bytes) => {
                    if ip_bytes.len() == 4 {
                        sans_ip.push(format!(
                            "{}.{}.{}.{}",
                            ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]
                        ));
                    } else if ip_bytes.len() == 16 {
                        let mut octets = [0u8; 16];
                        octets.copy_from_slice(ip_bytes);
                        sans_ip.push(std::net::Ipv6Addr::from(octets).to_string());
                    }
                }
                _ => {}
            }
        }
    }

    // CA 여부 (BasicConstraints 확인)
    let is_ca = cert
        .basic_constraints()
        .ok()
        .flatten()
        .map(|bc| bc.value.ca)
        .unwrap_or(false);

    // 유효기간 (ISO 8601 형식)
    let format_asn1_time = |asn1_time: &x509_parser::time::ASN1Time| -> String {
        let s = format!("{}", asn1_time);
        s.replace(" UTC", "Z").replacen(' ', "T", 1)
    };
    let not_before = format_asn1_time(&cert.validity().not_before);
    let not_after = format_asn1_time(&cert.validity().not_after);

    let serial_number = cert.raw_serial_as_string();

    // SHA-256 지문
    let hash = Sha256::digest(cert_der);
    let fingerprint_sha256 = hash
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":");

    CertificateInfo {
        subject_cn,
        issuer_cn,
        organization,
        sans_dns,
        sans_ip,
        not_before,
        not_after,
        serial_number,
        fingerprint_sha256,
        is_ca,
        chain_length,
    }
}
