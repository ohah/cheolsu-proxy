use super::analysis::is_openssl_required_domain;
use super::*;

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

#[test]
fn test_openssl_required_domain() {
    let auth: Authority = "api2.cursor.sh:443".parse().unwrap();
    assert!(is_openssl_required_domain(&auth));

    let auth: Authority = "google.com:443".parse().unwrap();
    assert!(!is_openssl_required_domain(&auth));
}
