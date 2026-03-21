use crate::tls_version_detector::TlsVersion;
use http::uri::Authority;
use tracing::info;

use super::{TlsConnectionInfo, TlsExtension, TlsStrategy};

/// 특정 도메인이 openssl을 필요로 하는지 확인합니다
pub(crate) fn is_openssl_required_domain(authority: &Authority) -> bool {
    let host = authority.host();
    let openssl_required_domains = [
        "api2.cursor.sh",
        "wps.apple.com",
        "gdmf.apple.com",
        "fbs.smoot.apple.com",
        "gateway.icloud.com",
    ];
    openssl_required_domains
        .iter()
        .any(|&domain| host == domain)
}

/// Extension 타입을 이름으로 변환
pub(crate) fn get_extension_name(extension_type: u16) -> String {
    match extension_type {
        0x0000 => "SNI".to_string(),
        0x0001 => "max_fragment_length".to_string(),
        0x0002 => "client_certificate_url".to_string(),
        0x0003 => "trusted_ca_keys".to_string(),
        0x0004 => "truncated_hmac".to_string(),
        0x0005 => "status_request".to_string(),
        0x0006 => "user_mapping".to_string(),
        0x0007 => "client_authz".to_string(),
        0x0008 => "server_authz".to_string(),
        0x0009 => "cert_type".to_string(),
        0x000a => "supported_groups".to_string(),
        0x000b => "ec_point_formats".to_string(),
        0x000c => "srp".to_string(),
        0x000d => "signature_algorithms".to_string(),
        0x000e => "use_srtp".to_string(),
        0x000f => "heartbeat".to_string(),
        0x0010 => "application_layer_protocol_negotiation".to_string(),
        0x0011 => "status_request_v2".to_string(),
        0x0012 => "signed_certificate_timestamp".to_string(),
        0x0013 => "client_certificate_type".to_string(),
        0x0014 => "server_certificate_type".to_string(),
        0x0015 => "padding".to_string(),
        0x0016 => "encrypt_then_mac".to_string(),
        0x0017 => "extended_master_secret".to_string(),
        0x0018 => "token_binding".to_string(),
        0x0019 => "cached_info".to_string(),
        0x001a => "tls_lts".to_string(),
        0x001b => "compress_certificate".to_string(),
        0x001c => "record_size_limit".to_string(),
        0x001d => "pwd_protect".to_string(),
        0x001e => "pwd_clear".to_string(),
        0x001f => "password_salt".to_string(),
        0x0020 => "ticket_pinning".to_string(),
        0x0021 => "tls_cert_with_extern_psk".to_string(),
        0x0022 => "delegated_credentials".to_string(),
        0x0023 => "session_ticket".to_string(),
        0x0024 => "TLMSP".to_string(),
        0x0025 => "TLMSP_proxying".to_string(),
        0x0026 => "TLMSP_delegate".to_string(),
        0x0027 => "supported_ekt_ciphers".to_string(),
        0x0029 => "pre_shared_key".to_string(),
        0x002a => "early_data".to_string(),
        0x002b => "supported_versions".to_string(),
        0x002c => "cookie".to_string(),
        0x002d => "psk_key_exchange_modes".to_string(),
        0x002f => "certificate_authorities".to_string(),
        0x0030 => "oid_filters".to_string(),
        0x0031 => "post_handshake_auth".to_string(),
        0x0032 => "signature_algorithms_cert".to_string(),
        0x0033 => "key_share".to_string(),
        _ => format!("unknown_0x{:04x}", extension_type),
    }
}

/// 연결 복잡도 점수를 계산합니다
pub(crate) fn calculate_complexity_score(
    cipher_suites: &[u16],
    extensions: &[TlsExtension],
    message_size: usize,
    has_apple_cipher: bool,
) -> u8 {
    let mut score = 0u8;

    // 암호화 스위트 개수에 따른 점수
    if cipher_suites.len() > 20 {
        score += 3;
    } else if cipher_suites.len() > 10 {
        score += 2;
    } else if cipher_suites.len() > 5 {
        score += 1;
    }

    // Extensions 개수에 따른 점수
    if extensions.len() > 10 {
        score += 3;
    } else if extensions.len() > 5 {
        score += 2;
    } else if extensions.len() > 2 {
        score += 1;
    }

    // 메시지 크기에 따른 점수
    if message_size > 1000 {
        score += 3;
    } else if message_size > 500 {
        score += 2;
    } else if message_size > 200 {
        score += 1;
    }

    // Apple 특별 암호화 스위트
    if has_apple_cipher {
        score += 2;
    }

    // SNI가 없는 경우 복잡도 증가
    let has_sni = extensions.iter().any(|ext| ext.extension_type == 0x0000);
    if !has_sni {
        score += 2;
    }

    score.min(10) // 최대 10점
}

/// TLS 연결을 상세 분석합니다 (순수 함수)
pub(crate) fn analyze_tls_connection(
    initial_buffer: &[u8],
) -> Result<TlsConnectionInfo, Box<dyn std::error::Error + Send + Sync>> {
    info!("🔍 [TLS-ANALYSIS] TLS 연결 분석 시작");

    // 기본 TLS 감지
    if initial_buffer.len() < 2 || initial_buffer[..2] != *b"\x16\x03" {
        return Err("TLS not detected".into());
    }

    if initial_buffer.len() < 11 {
        return Err("TLS handshake data too short".into());
    }

    // TLS 버전 분석
    let version_code = [initial_buffer[9], initial_buffer[10]];
    let version = match version_code {
        [0x03, 0x00] => TlsVersion::Ssl30,
        [0x03, 0x01] => TlsVersion::Tls10,
        [0x03, 0x02] => TlsVersion::Tls11,
        [0x03, 0x03] => TlsVersion::Tls12,
        [0x03, 0x04] => TlsVersion::Tls13,
        _ => {
            return Err(format!(
                "Unknown TLS version: 0x{:02x}{:02x}",
                version_code[0], version_code[1]
            )
            .into());
        }
    };

    info!("📊 [TLS-ANALYSIS] 기본 정보:");
    info!(
        "  - TLS 버전: {} (0x{:02x}{:02x})",
        version, version_code[0], version_code[1]
    );
    info!("  - 메시지 크기: {} bytes", initial_buffer.len());

    // ClientHello 상세 분석
    let mut cipher_suites = Vec::new();
    let mut extensions = Vec::new();
    let mut has_sni = false;
    let mut has_apple_cipher = false;
    let mut supported_versions_max: Option<TlsVersion> = None;
    let mut supported_groups: Vec<u16> = Vec::new();
    let mut signature_algorithms: Vec<u16> = Vec::new();
    let mut ec_point_formats: Vec<u8> = Vec::new();
    let mut alpn_protocols: Vec<Vec<u8>> = Vec::new();
    let mut compression_methods: Vec<u8> = Vec::new();

    if initial_buffer.len() >= 43 {
        let session_id_length = initial_buffer[43] as usize;
        info!("  - 세션 ID 길이: {} bytes", session_id_length);

        if initial_buffer.len() >= 44 + session_id_length + 2 {
            let cipher_suites_start = 44 + session_id_length;
            let cipher_suites_length = u16::from_be_bytes([
                initial_buffer[cipher_suites_start],
                initial_buffer[cipher_suites_start + 1],
            ]) as usize;

            // 암호화 스위트 분석
            if initial_buffer.len() >= cipher_suites_start + 2 + cipher_suites_length {
                let cipher_suites_end = cipher_suites_start + 2 + cipher_suites_length;
                for i in (cipher_suites_start + 2..cipher_suites_end).step_by(2) {
                    if i + 1 < initial_buffer.len() {
                        let suite = u16::from_be_bytes([initial_buffer[i], initial_buffer[i + 1]]);
                        cipher_suites.push(suite);

                        // Apple 특별 암호화 스위트 감지
                        if suite == 0xcaca {
                            has_apple_cipher = true;
                            info!("  - 🍎 Apple 특별 암호화 스위트 감지: 0x{:04x}", suite);
                        }
                    }
                }
            }

            // Compression Methods 분석
            let compression_methods_start = cipher_suites_start + 2 + cipher_suites_length;
            if initial_buffer.len() >= compression_methods_start + 1 {
                let compression_methods_length = initial_buffer[compression_methods_start] as usize;
                // compression methods 데이터 복사
                let cm_data_start = compression_methods_start + 1;
                if initial_buffer.len() >= cm_data_start + compression_methods_length {
                    compression_methods = initial_buffer
                        [cm_data_start..cm_data_start + compression_methods_length]
                        .to_vec();
                }
                let extensions_start = compression_methods_start + 1 + compression_methods_length;

                if initial_buffer.len() >= extensions_start + 2 {
                    let extensions_length = u16::from_be_bytes([
                        initial_buffer[extensions_start],
                        initial_buffer[extensions_start + 1],
                    ]) as usize;

                    let mut pos = extensions_start + 2;
                    let extensions_end = extensions_start + 2 + extensions_length;

                    while pos + 4 <= extensions_end && pos + 4 <= initial_buffer.len() {
                        let extension_type =
                            u16::from_be_bytes([initial_buffer[pos], initial_buffer[pos + 1]]);
                        let extension_length =
                            u16::from_be_bytes([initial_buffer[pos + 2], initial_buffer[pos + 3]])
                                as usize;

                        let extension_name = get_extension_name(extension_type);
                        // extension 원본 데이터 복사
                        let ext_data = if pos + 4 + extension_length <= initial_buffer.len() {
                            initial_buffer[pos + 4..pos + 4 + extension_length].to_vec()
                        } else {
                            Vec::new()
                        };
                        extensions.push(TlsExtension {
                            extension_type,
                            name: extension_name.clone(),
                            length: extension_length as u16,
                            data: ext_data.clone(),
                        });

                        // SNI Extension 감지
                        if extension_type == 0x0000 {
                            has_sni = true;
                            info!("  - ✅ SNI Extension 감지됨");
                        }

                        // Supported Groups (0x000a) 파싱
                        if extension_type == 0x000a && ext_data.len() >= 2 {
                            let list_len = u16::from_be_bytes([ext_data[0], ext_data[1]]) as usize;
                            for i in (2..2 + list_len).step_by(2) {
                                if i + 1 < ext_data.len() {
                                    supported_groups
                                        .push(u16::from_be_bytes([ext_data[i], ext_data[i + 1]]));
                                }
                            }
                        }

                        // EC Point Formats (0x000b) 파싱
                        if extension_type == 0x000b && !ext_data.is_empty() {
                            let fmt_len = ext_data[0] as usize;
                            if ext_data.len() >= 1 + fmt_len {
                                ec_point_formats = ext_data[1..1 + fmt_len].to_vec();
                            }
                        }

                        // Signature Algorithms (0x000d) 파싱
                        if extension_type == 0x000d && ext_data.len() >= 2 {
                            let list_len = u16::from_be_bytes([ext_data[0], ext_data[1]]) as usize;
                            for i in (2..2 + list_len).step_by(2) {
                                if i + 1 < ext_data.len() {
                                    signature_algorithms
                                        .push(u16::from_be_bytes([ext_data[i], ext_data[i + 1]]));
                                }
                            }
                        }

                        // ALPN (0x0010) 파싱
                        if extension_type == 0x0010 && ext_data.len() >= 2 {
                            let list_len = u16::from_be_bytes([ext_data[0], ext_data[1]]) as usize;
                            let mut alpn_pos = 2;
                            while alpn_pos < 2 + list_len && alpn_pos < ext_data.len() {
                                let proto_len = ext_data[alpn_pos] as usize;
                                alpn_pos += 1;
                                if alpn_pos + proto_len <= ext_data.len() {
                                    alpn_protocols
                                        .push(ext_data[alpn_pos..alpn_pos + proto_len].to_vec());
                                }
                                alpn_pos += proto_len;
                            }
                        }

                        // supported_versions Extension 파싱 (0x002b)
                        // TLS 1.3 클라이언트는 client_version을 0x0303(TLS 1.2)으로 설정하고
                        // 실제 지원 버전은 이 확장에 넣음 (RFC 8446)
                        if extension_type == 0x002b && extension_length > 0 {
                            let ext_data_start = pos + 4;
                            if ext_data_start < initial_buffer.len() {
                                let list_len = initial_buffer[ext_data_start] as usize;
                                let list_start = ext_data_start + 1;
                                // list_len은 반드시 짝수 (각 버전이 2바이트)
                                if list_len % 2 == 0
                                    && list_start + list_len <= initial_buffer.len()
                                {
                                    for vi in (0..list_len).step_by(2) {
                                        if list_start + vi + 1 < initial_buffer.len() {
                                            let ver = [
                                                initial_buffer[list_start + vi],
                                                initial_buffer[list_start + vi + 1],
                                            ];
                                            if ver == [0x03, 0x04] {
                                                supported_versions_max = Some(TlsVersion::Tls13);
                                                info!("  - ✅ supported_versions에서 TLS 1.3 감지");
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        pos += 4 + extension_length;
                    }
                }
            }
        }
    }

    // supported_versions 확장에서 감지된 실제 TLS 버전으로 업데이트
    // client_version 필드(바이트 9-10)보다 supported_versions 확장이 우선 (RFC 8446)
    let version = if let Some(sv) = supported_versions_max {
        info!(
            "📊 [TLS-ANALYSIS] supported_versions 확장에 의해 버전 업데이트: {} → {}",
            version, sv
        );
        sv
    } else {
        version
    };

    // 복잡도 점수 계산
    let complexity_score = calculate_complexity_score(
        &cipher_suites,
        &extensions,
        initial_buffer.len(),
        has_apple_cipher,
    );

    info!("📊 [TLS-ANALYSIS] 분석 결과:");
    info!("  - 암호화 스위트 개수: {}", cipher_suites.len());
    info!("  - Extensions 개수: {}", extensions.len());
    info!("  - SNI 지원: {}", has_sni);
    info!("  - Apple 암호화 스위트: {}", has_apple_cipher);
    info!("  - 복잡도 점수: {}", complexity_score);

    Ok(TlsConnectionInfo {
        version,
        version_code,
        cipher_suites,
        extensions,
        has_sni,
        has_apple_cipher,
        message_size: initial_buffer.len(),
        complexity_score,
        raw_client_hello: initial_buffer.to_vec(),
        supported_groups,
        signature_algorithms,
        ec_point_formats,
        alpn_protocols,
        compression_methods,
    })
}

/// TLS 처리 전략을 결정합니다 (순수 함수)
pub(crate) fn determine_tls_strategy(
    authority: &Authority,
    tls_info: &TlsConnectionInfo,
    tls_config: Option<&crate::tls_config::TlsConfigManager>,
) -> TlsStrategy {
    let host = authority.host();

    info!("🎯 [STRATEGY] 전략 결정 분석:");
    info!("  - 도메인: {}", host);
    info!("  - TLS 버전: {}", tls_info.version);
    info!("  - SNI 지원: {}", tls_info.has_sni);
    info!("  - Apple 암호화 스위트: {}", tls_info.has_apple_cipher);
    info!("  - 복잡도 점수: {}", tls_info.complexity_score);

    // 1. TLS 1.0/1.1/SSL 3.0은 OpenSSL 전용
    if matches!(
        tls_info.version,
        TlsVersion::Tls10 | TlsVersion::Tls11 | TlsVersion::Ssl30
    ) {
        info!(
            "🎯 [STRATEGY] 레거시 TLS 버전 감지 ({}) → OpenSSL 전용",
            tls_info.version
        );
        return TlsStrategy::OpenSslOnly;
    }

    // 2. TlsConfigManager 기반 도메인 규칙 확인
    if let Some(config) = tls_config {
        if config.requires_openssl(host) {
            info!("🎯 [STRATEGY] TLS 설정 규칙에 의해 OpenSSL 필수 → OpenSSL 전용");
            return TlsStrategy::OpenSslOnly;
        }
    } else {
        // TlsConfigManager 미설정 시 기존 하드코딩 동작
        if is_openssl_required_domain(authority) {
            info!("🎯 [STRATEGY] 특별한 도메인 감지 → OpenSSL 전용");
            return TlsStrategy::OpenSslOnly;
        }
    }

    // 3. Apple 특별 암호화 스위트가 있으면 OpenSSL 전용
    if tls_info.has_apple_cipher {
        info!("🎯 [STRATEGY] Apple 암호화 스위트 감지 → OpenSSL 전용");
        return TlsStrategy::OpenSslOnly;
    }

    // 4. SNI가 없고 복잡도가 높으면 OpenSSL 전용
    if !tls_info.has_sni && tls_info.complexity_score >= 6 {
        info!("🎯 [STRATEGY] SNI 없음 + 높은 복잡도 → OpenSSL 전용");
        return TlsStrategy::OpenSslOnly;
    }

    // 5. 기본적으로는 rustls 전용
    info!("🎯 [STRATEGY] 기본 전략 → Rustls 전용");
    TlsStrategy::RustlsOnly
}
