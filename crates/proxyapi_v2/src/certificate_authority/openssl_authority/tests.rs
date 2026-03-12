#[cfg(test)]
mod tests {
    use crate::certificate_authority::CertificateAuthority;
    use crate::certificate_authority::openssl_authority::OpensslAuthority;
    use crate::upstream_cert::UpstreamCertInfo;
    use http::uri::Authority;
    use openssl::{hash::MessageDigest, pkey::PKey, x509::X509};
    use std::sync::Arc;
    use tokio_rustls::rustls::crypto::aws_lc_rs;

    fn build_ca(cache_size: u64) -> OpensslAuthority {
        let private_key_bytes: &[u8] = include_bytes!("../cheolsu-proxy.key");
        let ca_cert_bytes: &[u8] = include_bytes!("../cheolsu-proxy.cer");
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
