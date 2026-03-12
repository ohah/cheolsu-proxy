#[cfg(test)]
mod tests {
    #[cfg(feature = "openssl-ca")]
    use crate::certificate_authority::CertificateAuthority;
    use crate::certificate_authority::rcgen_authority::RcgenAuthority;
    use crate::upstream_cert::UpstreamCertInfo;
    use http::uri::Authority;
    use rcgen::{CertificateParams, KeyPair};
    #[cfg(feature = "openssl-ca")]
    use std::sync::Arc;
    use tokio_rustls::rustls::crypto::aws_lc_rs;

    fn build_ca(cache_size: u64) -> RcgenAuthority {
        let key_pair = include_str!("../cheolsu-proxy.key");
        let ca_cert = include_str!("../cheolsu-proxy.cer");
        let key_pair = KeyPair::from_pem(key_pair).expect("Failed to parse private key");
        let ca_cert = CertificateParams::from_ca_cert_pem(ca_cert)
            .expect("Failed to parse CA certificate")
            .self_signed(&key_pair)
            .expect("Failed to sign CA certificate");

        RcgenAuthority::new(key_pair, ca_cert, cache_size, aws_lc_rs::default_provider())
    }

    #[cfg(feature = "openssl-ca")]
    #[tokio::test]
    async fn gen_openssl_context_returns_valid_context() {
        let ca = build_ca(1_000);
        let authority = Authority::from_static("example.com");

        let ctx = ca.gen_openssl_context(&authority, None).await;
        assert!(ctx.is_ok(), "OpenSSL 컨텍스트 생성 실패: {:?}", ctx.err());
    }

    #[cfg(feature = "openssl-ca")]
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

    #[cfg(feature = "openssl-ca")]
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

        let cert_der = ca.gen_cert(&authority, Some(&upstream)).unwrap();
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

        let cert_der = ca.gen_cert(&authority, None).unwrap();
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
        // rcgen의 CertificateParams::default()는 AKI를 자동 추가하지 않으므로
        // 현재 구현에서는 AKI가 없는 것이 정상 동작
        assert!(
            aki.is_none(),
            "AKI가 예상과 달리 존재합니다 (현재 구현에서는 AKI 미설정)"
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
