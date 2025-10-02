# Rustls MITM 프록시 개선사항

이 문서는 rustls를 사용한 MITM 프록시의 인증서 핸드셰이크 문제를 해결하기 위한 개선사항을 설명합니다.

## 🚀 주요 개선사항

### 1. SAN(Subject Alternative Name) 처리 개선

**문제**: 기존 코드는 단일 도메인만 SAN에 추가하여 와일드카드 도메인이나 서브도메인 처리에 문제가 있었습니다.

**해결책**:

- 기본 도메인 추가
- 와일드카드 도메인 자동 생성 (`*.example.com`)
- IP 주소 처리
- localhost 및 127.0.0.1 특별 처리

```rust
fn add_san_entries(&self, params: &mut CertificateParams, host: &str) {
    // 기본 도메인 추가
    if let Ok(dns_name) = Ia5String::try_from(host) {
        params.subject_alt_names.push(SanType::DnsName(dns_name));
    }

    // 와일드카드 도메인 처리
    if !host.starts_with("*.") {
        let wildcard = format!("*.{}", host);
        if let Ok(wildcard_name) = Ia5String::try_from(wildcard.as_str()) {
            params.subject_alt_names.push(SanType::DnsName(wildcard_name));
        }
    }

    // IP 주소 및 localhost 처리
    // ...
}
```

### 2. 사설 CA 인증서 자동 추가

**문제**: 클라이언트 설정에 사설 CA가 추가되지 않아 MITM 인증서가 신뢰되지 않았습니다.

**해결책**:

- `CertificateAuthority` 트레이트에 `get_ca_cert_der()` 메서드 추가
- 클라이언트 설정 생성 시 자동으로 사설 CA 추가
- webpki_roots와 함께 사용하여 호환성 보장

```rust
// 사설 CA 인증서 추가
if let Some(ca_cert_der) = self.0.ca.get_ca_cert_der() {
    debug!("Adding custom CA certificate ({} bytes)", ca_cert_der.len());
    if let Err(e) = client_config.root_store.add(CertificateDer::from(ca_cert_der)) {
        warn!("Failed to add custom CA to root store: {}", e);
    } else {
        info!("Successfully added custom CA to root store");
    }
}
```

### 3. 상세한 디버깅 로그

**개선사항**:

- 인증서 생성 과정 로깅
- SAN 엔트리 생성 로깅
- TLS 설정 로깅
- 에러 발생 시 상세 정보 제공

```rust
info!("Generating certificate for authority: {}", authority);
debug!("Certificate host: {}", host);
info!("Generated {} SAN entries for host '{}'", params.subject_alt_names.len(), host);
```

### 4. TLS 버전 및 ALPN 최적화

**개선사항**:

- HTTP/2 우선, HTTP/1.1 fallback ALPN 설정
- 클라이언트와 서버 모두에 일관된 ALPN 설정
- TLS 버전 호환성 개선

```rust
// ALPN 프로토콜 설정 - HTTP/2 우선, HTTP/1.1 fallback
client_config.alpn_protocols = vec![
    #[cfg(feature = "http2")]
    b"h2".to_vec(),
    b"http/1.1".to_vec(),
];
```

## 🔧 사용법

### 기본 사용법

```rust
use proxyapi_v2::{
    certificate_authority::build_ca,
    Proxy,
};
use tokio_rustls::rustls::crypto::aws_lc_rs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CA 인증서 로드 또는 생성
    let ca = build_ca()?;

    // 프록시 빌드
    let proxy = Proxy::builder()
        .with_addr("127.0.0.1:8080".parse()?)
        .with_ca(ca)
        .with_rustls_client(aws_lc_rs::default_provider())
        .build()?;

    // 프록시 시작
    proxy.start().await?;
    Ok(())
}
```

### 디버깅 모드 실행

```bash
# 상세한 로그와 함께 실행
RUST_LOG=debug cargo run --example improved_rustls_proxy

# 특정 모듈만 로그 출력
RUST_LOG=proxyapi_v2::certificate_authority=debug cargo run
```

## 🐛 문제 해결

### 1. 인증서 핸드셰이크 실패

**증상**: `InvalidCertificate(CertNotValidForName)` 오류

**해결책**:

- SAN 엔트리가 올바르게 생성되었는지 확인
- 로그에서 "Generated X SAN entries" 메시지 확인
- 와일드카드 도메인이 필요한 경우 자동 생성됨

### 2. 사설 CA 신뢰 문제

**증상**: `InvalidCertificate(UnknownIssuer)` 오류

**해결책**:

- 로그에서 "Successfully added custom CA to root store" 메시지 확인
- CA 인증서가 올바른 DER 형식인지 확인
- 브라우저에 CA 인증서를 신뢰할 수 있는 인증서로 추가

### 3. TLS 버전 호환성 문제

**증상**: `PeerIncompatibleError` 오류

**해결책**:

- ALPN 프로토콜 설정 확인
- HTTP/2와 HTTP/1.1 모두 지원하는지 확인
- 서버의 TLS 버전 요구사항 확인

## 📊 성능 개선

### 인증서 캐싱

- 생성된 인증서는 메모리에 캐시됨
- 동일한 도메인에 대한 반복 요청 시 빠른 응답
- 캐시 TTL: 6개월 (기본값)

### 연결 풀링

- hyper-rustls의 연결 풀링 활용
- HTTP/2 멀티플렉싱 지원
- Keep-alive 연결 최적화

## 🔒 보안 고려사항

### 인증서 보안

- 강력한 키 알고리즘 사용 (ECDSA P-256)
- 적절한 인증서 유효기간 설정
- SAN 필드 검증

### TLS 보안

- 안전한 TLS 버전만 사용 (TLS 1.2+)
- 강력한 암호화 스위트 사용
- Perfect Forward Secrecy 지원

## 📝 로그 예시

```
INFO proxyapi_v2::certificate_authority: Generating certificate for authority: example.com:443
DEBUG proxyapi_v2::certificate_authority: Certificate host: example.com
DEBUG proxyapi_v2::certificate_authority: Added DNS SAN: example.com
DEBUG proxyapi_v2::certificate_authority: Added wildcard SAN: *.example.com
INFO proxyapi_v2::certificate_authority: Generated 2 SAN entries for host 'example.com'
INFO proxyapi_v2::certificate_authority: Successfully generated certificate for 'example.com:443'
INFO proxyapi_v2::proxy::builder: Building rustls client configuration
DEBUG proxyapi_v2::proxy::builder: Adding custom CA certificate (1234 bytes)
INFO proxyapi_v2::proxy::builder: Successfully added custom CA to root store
DEBUG proxyapi_v2::proxy::builder: Client config ALPN protocols: ["h2", "http/1.1"]
```

## 🚀 향후 개선 계획

1. **OCSP Stapling 지원**: 인증서 상태 확인 최적화
2. **Certificate Transparency**: 인증서 투명성 로깅
3. **HTTP/3 지원**: QUIC 프로토콜 지원
4. **동적 인증서 관리**: 런타임 인증서 업데이트
5. **메트릭 수집**: Prometheus 메트릭 지원

## 📚 참고 자료

- [Rustls 공식 문서](https://docs.rs/rustls/)
- [Hyper-rustls 문서](https://docs.rs/hyper-rustls/)
- [RCGen 문서](https://docs.rs/rcgen/)
- [TLS 1.3 RFC](https://tools.ietf.org/html/rfc8446)
