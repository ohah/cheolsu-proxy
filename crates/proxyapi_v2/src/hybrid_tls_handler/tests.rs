use super::analysis::is_openssl_required_domain;
use super::*;
use crate::tls_version_detector::TlsVersion;

/// 최소 유효한 TLS 1.2 ClientHello를 생성하는 헬퍼
/// 구조: record_hdr(5) + handshake_hdr(4) + client_version(2) + random(32) +
///       session_id_len(1) + cipher_suites_len(2) + cipher_suites(N*2) +
///       compression_len(1) + compression(1) + extensions_len(2) + extensions(...)
fn build_client_hello(
    client_version: [u8; 2],
    cipher_suites: &[u16],
    extensions: &[(u16, &[u8])], // (type, data)
) -> Vec<u8> {
    let mut body = Vec::new();

    // client_version
    body.extend_from_slice(&client_version);
    // random (32 bytes)
    body.extend_from_slice(&[0u8; 32]);
    // session_id_length = 0
    body.push(0);
    // cipher suites
    let cs_len = (cipher_suites.len() * 2) as u16;
    body.extend_from_slice(&cs_len.to_be_bytes());
    for &cs in cipher_suites {
        body.extend_from_slice(&cs.to_be_bytes());
    }
    // compression methods: 1 method (null)
    body.push(1);
    body.push(0);

    // extensions
    let mut ext_buf = Vec::new();
    for &(ext_type, ext_data) in extensions {
        ext_buf.extend_from_slice(&ext_type.to_be_bytes());
        ext_buf.extend_from_slice(&(ext_data.len() as u16).to_be_bytes());
        ext_buf.extend_from_slice(ext_data);
    }
    let ext_len = ext_buf.len() as u16;
    body.extend_from_slice(&ext_len.to_be_bytes());
    body.extend_from_slice(&ext_buf);

    // Handshake header: type=ClientHello(0x01), length=body.len() (3 bytes)
    let hs_len = body.len() as u32;
    let mut handshake = vec![0x01];
    handshake.push((hs_len >> 16) as u8);
    handshake.push((hs_len >> 8) as u8);
    handshake.push(hs_len as u8);
    handshake.extend_from_slice(&body);

    // TLS record header: type=Handshake(0x16), version=TLS 1.0(0x03,0x01), length
    let record_len = handshake.len() as u16;
    let mut record = vec![0x16, 0x03, 0x01];
    record.extend_from_slice(&record_len.to_be_bytes());
    record.extend_from_slice(&handshake);

    record
}

// ─── TLS 전략 단위 테스트 ───

#[test]
fn test_tls10_routes_to_openssl_only() {
    let buf = build_client_hello([0x03, 0x01], &[0x002f], &[]);
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(info.version, TlsVersion::Tls10);

    let authority: Authority = "example.com:443".parse().unwrap();
    let strategy = determine_tls_strategy(&authority, &info, None);
    assert_eq!(strategy, TlsStrategy::OpenSslOnly);
}

#[test]
fn test_tls11_routes_to_openssl_only() {
    let buf = build_client_hello([0x03, 0x02], &[0x002f], &[]);
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(info.version, TlsVersion::Tls11);

    let authority: Authority = "example.com:443".parse().unwrap();
    let strategy = determine_tls_strategy(&authority, &info, None);
    assert_eq!(strategy, TlsStrategy::OpenSslOnly);
}

#[test]
fn test_ssl30_routes_to_openssl_only() {
    let buf = build_client_hello([0x03, 0x00], &[0x002f], &[]);
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(info.version, TlsVersion::Ssl30);

    let authority: Authority = "example.com:443".parse().unwrap();
    let strategy = determine_tls_strategy(&authority, &info, None);
    assert_eq!(strategy, TlsStrategy::OpenSslOnly);
}

#[test]
fn test_tls12_normal_domain_routes_to_rustls() {
    // SNI extension present (type 0x0000)
    let sni_data = b"\x00\x00\x0eexample.com";
    let buf = build_client_hello(
        [0x03, 0x03],
        &[0x1301, 0x1302, 0xc02c],
        &[(0x0000, sni_data)],
    );
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(info.version, TlsVersion::Tls12);
    assert!(info.has_sni);

    let authority: Authority = "example.com:443".parse().unwrap();
    let strategy = determine_tls_strategy(&authority, &info, None);
    assert_eq!(strategy, TlsStrategy::RustlsOnly);
}

#[test]
fn test_apple_domain_routes_normally() {
    // 터널 모드 제거 후 Apple 도메인도 일반 전략으로 처리
    let sni_data = b"\x00\x00\x15gateway.icloud.com";
    let buf = build_client_hello([0x03, 0x03], &[0x1301], &[(0x0000, sni_data)]);
    let info = analyze_tls_connection(&buf).unwrap();

    let authority: Authority = "gateway.icloud.com:443".parse().unwrap();
    let strategy = determine_tls_strategy(&authority, &info, None);
    // TLS 자동 바이패스가 터널 역할을 대체
    assert!(matches!(
        strategy,
        TlsStrategy::OpenSslOnly | TlsStrategy::RustlsOnly
    ));
}

#[test]
fn test_apple_cipher_routes_to_openssl_only() {
    // 0xcaca is Apple's GREASE cipher suite marker
    let sni_data = b"\x00\x00\x0eexample.com";
    let buf = build_client_hello(
        [0x03, 0x03],
        &[0xcaca, 0x1301, 0xc02c],
        &[(0x0000, sni_data)],
    );
    let info = analyze_tls_connection(&buf).unwrap();
    assert!(info.has_apple_cipher);

    let authority: Authority = "example.com:443".parse().unwrap();
    let strategy = determine_tls_strategy(&authority, &info, None);
    assert_eq!(strategy, TlsStrategy::OpenSslOnly);
}

#[test]
fn test_no_sni_high_complexity_routes_to_openssl() {
    // Build a buffer with many cipher suites, many extensions, large size, no SNI
    // This should give a high complexity score
    let many_ciphers: Vec<u16> = (0x0001..=0x0019).collect(); // 25 ciphers → +3
    let mut dummy_extensions = Vec::new();
    for i in 1u16..=12 {
        // 12 extensions → +3, no SNI → +2
        dummy_extensions.push((i, &[][..]));
    }
    let buf = build_client_hello([0x03, 0x03], &many_ciphers, &dummy_extensions);
    let info = analyze_tls_connection(&buf).unwrap();
    assert!(!info.has_sni);
    assert!(info.complexity_score >= 6);

    let authority: Authority = "some-server.com:443".parse().unwrap();
    let strategy = determine_tls_strategy(&authority, &info, None);
    assert_eq!(strategy, TlsStrategy::OpenSslOnly);
}

// ─── ClientHello 파싱 테스트 ───

#[test]
fn test_valid_tls12_client_hello_parses() {
    let sni_data = b"\x00\x00\x0eexample.com";
    let buf = build_client_hello(
        [0x03, 0x03],
        &[0x1301, 0xc02c],
        &[(0x0000, sni_data), (0x000d, &[0x00, 0x02, 0x04, 0x03])],
    );
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(info.version, TlsVersion::Tls12);
    assert!(info.has_sni);
    assert_eq!(info.cipher_suites.len(), 2);
    assert_eq!(info.extensions.len(), 2);
}

#[test]
fn test_short_buffer_returns_error() {
    let buf = vec![0x16, 0x03, 0x01, 0x00];
    let result = analyze_tls_connection(&buf);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too short"));
}

#[test]
fn test_http_data_returns_error() {
    let buf = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = analyze_tls_connection(buf);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("TLS not detected"));
}

#[test]
fn test_unknown_tls_version_returns_error() {
    // Use version bytes [0x03, 0x05] which is not a known TLS version
    let buf = build_client_hello([0x03, 0x05], &[0x002f], &[]);
    let result = analyze_tls_connection(&buf);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unknown TLS version")
    );
}

// ─── supported_versions 파싱 테스트 ───

#[test]
fn test_tls13_detected_via_supported_versions() {
    // client_version은 TLS 1.2(0x0303)이지만
    // supported_versions extension(0x002b)에 TLS 1.3(0x0304)이 포함됨
    // → TLS 1.3으로 감지되어야 함
    let sni_data = b"\x00\x00\x0eexample.com";
    // supported_versions extension: list_len(1) + versions
    // [0x03, 0x04, 0x03, 0x03] = TLS 1.3, TLS 1.2
    let sv_data: &[u8] = &[0x04, 0x03, 0x04, 0x03, 0x03];
    let buf = build_client_hello(
        [0x03, 0x03], // client_version = TLS 1.2
        &[0x1301, 0x1302, 0xc02c],
        &[(0x0000, sni_data), (0x002b, sv_data)],
    );
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(
        info.version,
        TlsVersion::Tls13,
        "supported_versions에 0x0304가 있으면 TLS 1.3으로 감지해야 함"
    );
}

#[test]
fn test_tls12_only_in_supported_versions() {
    // supported_versions에 TLS 1.2만 있으면 TLS 1.2로 유지
    let sni_data = b"\x00\x00\x0eexample.com";
    let sv_data: &[u8] = &[0x02, 0x03, 0x03]; // list_len=2, TLS 1.2 only
    let buf = build_client_hello(
        [0x03, 0x03],
        &[0x1301],
        &[(0x0000, sni_data), (0x002b, sv_data)],
    );
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(
        info.version,
        TlsVersion::Tls12,
        "supported_versions에 TLS 1.3이 없으면 client_version 기반으로 유지"
    );
}

#[test]
fn test_malformed_supported_versions_odd_length() {
    // list_len이 홀수(3) → malformed → 무시하고 client_version 사용
    let sni_data = b"\x00\x00\x0eexample.com";
    let sv_data: &[u8] = &[0x03, 0x03, 0x04, 0x03]; // list_len=3 (홀수!)
    let buf = build_client_hello(
        [0x03, 0x03],
        &[0x1301],
        &[(0x0000, sni_data), (0x002b, sv_data)],
    );
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(
        info.version,
        TlsVersion::Tls12,
        "홀수 list_len은 무시하고 client_version(TLS 1.2) 유지"
    );
}

#[test]
fn test_tls13_with_apple_cipher_routes_to_openssl() {
    // TLS 1.3 + Apple cipher → OpenSSL 경로로 가되 버전은 정확히 1.3
    let sni_data = b"\x00\x00\x0eexample.com";
    let sv_data: &[u8] = &[0x04, 0x03, 0x04, 0x03, 0x03];
    let buf = build_client_hello(
        [0x03, 0x03],
        &[0xcaca, 0x1301, 0x1302], // 0xcaca = Apple GREASE cipher
        &[(0x0000, sni_data), (0x002b, sv_data)],
    );
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(info.version, TlsVersion::Tls13);
    assert!(info.has_apple_cipher);

    let authority: Authority = "example.com:443".parse().unwrap();
    let strategy = determine_tls_strategy(&authority, &info, None);
    assert_eq!(strategy, TlsStrategy::OpenSslOnly);
}

// ─── ClientHello 미러링 파싱 테스트 ───

#[test]
fn test_analyze_extracts_supported_groups() {
    // supported_groups extension (0x000a) 데이터: length(2) + groups
    let mut groups_data = Vec::new();
    let groups: &[u16] = &[0x0017, 0x0018, 0x001d]; // P-256, P-384, X25519
    let groups_len = (groups.len() * 2) as u16;
    groups_data.extend_from_slice(&groups_len.to_be_bytes());
    for &g in groups {
        groups_data.extend_from_slice(&g.to_be_bytes());
    }

    let buf = build_client_hello([0x03, 0x03], &[0xc02f, 0xc030], &[(0x000a, &groups_data)]);
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(info.supported_groups, vec![0x0017, 0x0018, 0x001d]);
}

#[test]
fn test_analyze_extracts_signature_algorithms() {
    // signature_algorithms extension (0x000d)
    let mut sigalg_data = Vec::new();
    let sigalgs: &[u16] = &[0x0403, 0x0804, 0x0807]; // ECDSA+SHA256, RSA-PSS+SHA256, Ed25519
    let sigalg_len = (sigalgs.len() * 2) as u16;
    sigalg_data.extend_from_slice(&sigalg_len.to_be_bytes());
    for &s in sigalgs {
        sigalg_data.extend_from_slice(&s.to_be_bytes());
    }

    let buf = build_client_hello([0x03, 0x03], &[0xc02f], &[(0x000d, &sigalg_data)]);
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(info.signature_algorithms, vec![0x0403, 0x0804, 0x0807]);
}

#[test]
fn test_analyze_extracts_alpn_protocols() {
    // ALPN extension (0x0010): list_len(2) + [proto_len(1) + proto_data]*
    let mut alpn_data = Vec::new();
    let protos: &[&[u8]] = &[b"h2", b"http/1.1"];
    let mut proto_buf = Vec::new();
    for proto in protos {
        proto_buf.push(proto.len() as u8);
        proto_buf.extend_from_slice(proto);
    }
    alpn_data.extend_from_slice(&(proto_buf.len() as u16).to_be_bytes());
    alpn_data.extend_from_slice(&proto_buf);

    let buf = build_client_hello([0x03, 0x03], &[0xc02f], &[(0x0010, &alpn_data)]);
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(
        info.alpn_protocols,
        vec![b"h2".to_vec(), b"http/1.1".to_vec()]
    );
}

#[test]
fn test_analyze_extension_data_preserved() {
    let dummy_data = vec![0x01, 0x02, 0x03, 0x04];
    let buf = build_client_hello(
        [0x03, 0x03],
        &[0xc02f],
        &[(0x0023, &dummy_data)], // session_ticket extension
    );
    let info = analyze_tls_connection(&buf).unwrap();
    let session_ticket_ext = info.extensions.iter().find(|e| e.extension_type == 0x0023);
    assert!(session_ticket_ext.is_some());
    assert_eq!(session_ticket_ext.unwrap().data, dummy_data);
}

#[test]
fn test_analyze_multiple_extensions_all_parsed() {
    // supported_groups
    let mut groups_data = Vec::new();
    groups_data.extend_from_slice(&4u16.to_be_bytes());
    groups_data.extend_from_slice(&0x0017u16.to_be_bytes());
    groups_data.extend_from_slice(&0x001du16.to_be_bytes());

    // signature_algorithms
    let mut sigalg_data = Vec::new();
    sigalg_data.extend_from_slice(&2u16.to_be_bytes());
    sigalg_data.extend_from_slice(&0x0403u16.to_be_bytes());

    let buf = build_client_hello(
        [0x03, 0x03],
        &[0xc02f, 0xc030, 0x1301],
        &[(0x000a, &groups_data), (0x000d, &sigalg_data)],
    );
    let info = analyze_tls_connection(&buf).unwrap();
    assert_eq!(info.supported_groups, vec![0x0017, 0x001d]);
    assert_eq!(info.signature_algorithms, vec![0x0403]);
    assert_eq!(info.cipher_suites, vec![0xc02f, 0xc030, 0x1301]);
}

#[test]
fn test_openssl_required_domain() {
    let auth: Authority = "api2.cursor.sh:443".parse().unwrap();
    assert!(is_openssl_required_domain(&auth));

    let auth: Authority = "google.com:443".parse().unwrap();
    assert!(!is_openssl_required_domain(&auth));
}
