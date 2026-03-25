#[cfg(test)]
mod tests {
    use crate::certificate_authority::CertificateAuthority;
    use crate::certificate_authority::rcgen_authority::RcgenAuthority;
    use crate::upstream_cert::UpstreamCertInfo;
    use http::uri::Authority;
    use std::sync::Arc;
    use tokio_rustls::rustls::crypto::aws_lc_rs;

    fn build_ca(cache_size: u64) -> RcgenAuthority {
        let key_pem = include_str!("../cheolsu-proxy.key");
        let ca_cert_pem = include_str!("../cheolsu-proxy.cer");

        RcgenAuthority::from_pem(
            ca_cert_pem,
            key_pem,
            cache_size,
            aws_lc_rs::default_provider(),
        )
        .expect("Failed to create RcgenAuthority from PEM")
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

        let c1 = ca.gen_cert(&authority1, None).unwrap().0;
        let c2 = ca.gen_cert(&authority2, None).unwrap().0;
        let c3 = ca.gen_cert(&authority1, None).unwrap().0;
        let c4 = ca.gen_cert(&authority2, None).unwrap().0;

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
            ..Default::default()
        };

        let cert_der = ca.gen_cert(&authority, Some(&upstream)).unwrap().0;
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

        let cert_der = ca.gen_cert(&authority, None).unwrap().0;
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

        let cert_der = ca.gen_cert(&authority, None).unwrap().0;
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        // AKI extension (OID 2.5.29.35) 이 존재하는지 확인
        let aki = cert.extensions().iter().find(|ext| {
            ext.oid == x509_parser::oid_registry::OID_X509_EXT_AUTHORITY_KEY_IDENTIFIER
        });
        // Windows SChannel 호환성을 위해 AKI를 추가함
        assert!(
            aki.is_some(),
            "AKI가 존재해야 합니다 (Windows SChannel 호환성)"
        );
    }

    #[test]
    fn gen_cert_mirrors_rsa_key_type() {
        use crate::upstream_cert::UpstreamKeyType;

        let ca = build_ca(0);
        let authority = Authority::from_static("rsa-test.example.com");
        let upstream = UpstreamCertInfo {
            key_type: UpstreamKeyType::Rsa(2048),
            ..Default::default()
        };

        let (cert_der, _private_key) = ca.gen_cert(&authority, Some(&upstream)).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        // RSA 키 타입 확인
        let pk = cert.public_key();
        let alg_oid = pk.algorithm.algorithm.to_id_string();
        // RSA OID: 1.2.840.113549.1.1.1
        assert_eq!(alg_oid, "1.2.840.113549.1.1.1");
    }

    #[test]
    fn gen_cert_mirrors_ecdsa_p256_key_type() {
        use crate::upstream_cert::{EcCurve, UpstreamKeyType};

        let ca = build_ca(0);
        let authority = Authority::from_static("ecdsa-test.example.com");
        let upstream = UpstreamCertInfo {
            key_type: UpstreamKeyType::Ecdsa(EcCurve::P256),
            ..Default::default()
        };

        let (cert_der, _private_key) = ca.gen_cert(&authority, Some(&upstream)).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        // ECDSA 키 타입 확인
        let pk = cert.public_key();
        let alg_oid = pk.algorithm.algorithm.to_id_string();
        // EC public key OID: 1.2.840.10045.2.1
        assert_eq!(alg_oid, "1.2.840.10045.2.1");
    }

    #[test]
    fn gen_cert_mirrors_ecdsa_p384_key_type() {
        use crate::upstream_cert::{EcCurve, UpstreamKeyType};

        let ca = build_ca(0);
        let authority = Authority::from_static("ecdsa384-test.example.com");
        let upstream = UpstreamCertInfo {
            key_type: UpstreamKeyType::Ecdsa(EcCurve::P384),
            ..Default::default()
        };

        let (cert_der, _private_key) = ca.gen_cert(&authority, Some(&upstream)).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        let pk = cert.public_key();
        let alg_oid = pk.algorithm.algorithm.to_id_string();
        assert_eq!(alg_oid, "1.2.840.10045.2.1");

        // P-384 곡선 확인
        let curve_oid = pk.algorithm.parameters.as_ref().unwrap().as_oid().unwrap();
        assert_eq!(curve_oid.to_id_string(), "1.3.132.0.34");
    }

    #[test]
    fn gen_cert_mirrors_ed25519_key_type() {
        use crate::upstream_cert::UpstreamKeyType;

        let ca = build_ca(0);
        let authority = Authority::from_static("ed25519-test.example.com");
        let upstream = UpstreamCertInfo {
            key_type: UpstreamKeyType::Ed25519,
            ..Default::default()
        };

        let (cert_der, _private_key) = ca.gen_cert(&authority, Some(&upstream)).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        let pk = cert.public_key();
        let alg_oid = pk.algorithm.algorithm.to_id_string();
        // Ed25519 OID: 1.3.101.112
        assert_eq!(alg_oid, "1.3.101.112");
    }

    #[test]
    fn gen_cert_default_key_type_is_ecdsa_p256() {
        let ca = build_ca(0);
        let authority = Authority::from_static("default-key.example.com");

        let (cert_der, _private_key) = ca.gen_cert(&authority, None).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        let pk = cert.public_key();
        let alg_oid = pk.algorithm.algorithm.to_id_string();
        // 기본값은 ECDSA
        assert_eq!(alg_oid, "1.2.840.10045.2.1");
        // P-256 곡선
        let curve_oid = pk.algorithm.parameters.as_ref().unwrap().as_oid().unwrap();
        assert_eq!(curve_oid.to_id_string(), "1.2.840.10045.3.1.7");
    }

    #[test]
    fn gen_cert_same_key_type_produces_same_algorithm() {
        let ca = build_ca(0);
        let authority = Authority::from_static("cache-test.example.com");

        let (cert1_der, _) = ca.gen_cert(&authority, None).unwrap();
        let (cert2_der, _) = ca.gen_cert(&authority, None).unwrap();

        let (_, cert1) = x509_parser::parse_x509_certificate(&cert1_der).unwrap();
        let (_, cert2) = x509_parser::parse_x509_certificate(&cert2_der).unwrap();

        // 같은 키 타입이면 동일 알고리즘 사용
        assert_eq!(
            cert1.public_key().algorithm.algorithm,
            cert2.public_key().algorithm.algorithm
        );
    }

    #[test]
    fn gen_cert_cn_truncated_to_64_chars() {
        let ca = build_ca(0);
        let long_host = format!("{}.example.com", "a".repeat(80));
        let authority = Authority::try_from(long_host).unwrap();

        let cert_der = ca.gen_cert(&authority, None).unwrap().0;
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

    /// Thundering herd 방지 테스트: 동일 도메인에 동시 요청 시 인증서가 한 번만 생성되는지 확인
    #[tokio::test]
    async fn gen_server_config_thundering_herd() {
        let ca = Arc::new(build_ca(1_000));
        let authority = Authority::from_static("thundering-herd.example.com");

        // 10개 동시 요청 발행
        let mut handles = Vec::new();
        for _ in 0..10 {
            let ca_clone = ca.clone();
            let auth_clone = authority.clone();
            handles.push(tokio::spawn(async move {
                ca_clone
                    .gen_server_config(&auth_clone, None)
                    .await
                    .expect("ServerConfig 생성 실패")
            }));
        }

        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // 모든 결과가 동일한 Arc를 공유해야 함 (동일 캐시 엔트리)
        // Arc::ptr_eq로 같은 인스턴스인지 확인
        let first = &results[0];
        for (i, result) in results.iter().enumerate().skip(1) {
            assert!(
                Arc::ptr_eq(first, result),
                "요청 {}이 다른 ServerConfig 인스턴스를 반환했습니다 (thundering herd 발생)",
                i
            );
        }
    }

    /// 캐시 히트 시 동일한 ServerConfig가 반환되는지 확인
    #[tokio::test]
    async fn gen_server_config_cache_returns_same_instance() {
        let ca = build_ca(1_000);
        let authority = Authority::from_static("cache-instance.example.com");

        let cfg1 = ca.gen_server_config(&authority, None).await.unwrap();
        let cfg2 = ca.gen_server_config(&authority, None).await.unwrap();

        assert!(
            Arc::ptr_eq(&cfg1, &cfg2),
            "캐시에서 반환된 ServerConfig는 동일 인스턴스여야 합니다"
        );
    }

    /// Leaf 인증서 유효기간이 90일인지 확인
    #[test]
    fn gen_cert_leaf_validity_is_90_days() {
        let ca = build_ca(0);
        let authority = Authority::from_static("leaf-ttl.example.com");

        let cert_der = ca.gen_cert(&authority, None).unwrap().0;
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der).unwrap();

        let not_before = cert.validity().not_before.timestamp();
        let not_after = cert.validity().not_after.timestamp();
        let validity_secs = not_after - not_before;

        // 90일 = 7,776,000초 (NOT_BEFORE_OFFSET 포함 보정)
        let expected = crate::certificate_authority::LEAF_TTL_SECS;
        assert_eq!(
            validity_secs, expected,
            "Leaf 인증서 유효기간이 {}초(90일)여야 하지만 {}초입니다",
            expected, validity_secs
        );
    }

    /// CertEvent enum이 올바르게 동작하는지 확인
    #[test]
    fn cert_event_variants() {
        use crate::certificate_authority::CertEvent;

        let e1 = CertEvent::CaGenerated;
        let e2 = CertEvent::CaRegeneratedExpiringSoon(15);
        let e3 = CertEvent::CaRegeneratedExpired;
        let e4 = CertEvent::CaRegeneratedCorrupted("키 불일치".to_string());
        let e5 = CertEvent::CaLoaded;

        // Debug, Clone, PartialEq 동작 확인
        assert_eq!(e1.clone(), CertEvent::CaGenerated);
        assert_eq!(e2.clone(), CertEvent::CaRegeneratedExpiringSoon(15));
        assert_ne!(e3, e5);
        assert_eq!(
            format!("{:?}", e4),
            r#"CaRegeneratedCorrupted("키 불일치")"#
        );
    }
}
