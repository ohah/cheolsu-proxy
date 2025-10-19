---
title: TLS 1.0/1.1 지원
description: 레거시 TLS 클라이언트 지원 및 하이브리드 TLS 핸들러
---

# TLS 1.0/1.1 레거시 지원

Cheolsu Proxy는 최신 TLS 버전뿐만 아니라 레거시 TLS 1.0/1.1 클라이언트도 지원합니다. 이를 위해 TLS 버전을 자동 감지하여 적절한 TLS 라이브러리를 선택하는 하이브리드 방식을 구현했습니다.

## 🎯 주요 기능

- **자동 TLS 버전 감지**: ClientHello에서 TLS 버전을 자동으로 감지
- **하이브리드 TLS 처리**: TLS 1.0/1.1은 native-tls, TLS 1.2+는 rustls 사용
- **크로스 플랫폼 호환성**: macOS, Windows에서 모두 동작
- **PKCS12 인증서 지원**: native-tls용 PKCS12 인증서 자동 생성
- **터널 모드**: 특정 호스트의 TLS 문제 해결을 위한 직접 터널링

## 🔧 구현 방식

### TLS 버전별 라이브러리 선택

```
TLS 1.0/1.1 → native-tls (OpenSSL 기반)
TLS 1.2/1.3 → rustls (순수 Rust)
특정 호스트 → 터널 모드 (직접 연결)
```

### 터널 모드 (Tunnel Mode)

일부 호스트(특히 Apple 서비스)는 엄격한 TLS 요구사항으로 인해 프록시를 통한 TLS 핸드셰이크가 실패할 수 있습니다. 이러한 경우 터널 모드를 사용하여 프록시가 TLS 처리를 우회하고 클라이언트와 서버 간의 직접적인 TCP 터널을 생성합니다.

#### 터널 모드 대상 호스트

- `gateway.icloud.com`
- `p112-contacts.icloud.com`
- `p112-caldav.icloud.com`
- `wps.apple.com`
- 기타 Apple 서비스 도메인

#### 터널 모드 동작 방식

1. **CONNECT 요청 수신**: 클라이언트가 특정 호스트로 CONNECT 요청
2. **터널 모드 감지**: 호스트가 터널 모드 대상인지 확인
3. **200 Connection Established 응답**: 즉시 터널 설정 완료 응답 전송
4. **직접 TCP 연결**: 클라이언트와 대상 서버 간 직접 TCP 스트림 생성
5. **양방향 데이터 전송**: `tokio::io::copy_bidirectional`로 데이터 중계
6. **이벤트 로깅**: 터널 시작/완료/오류 이벤트를 Tauri UI로 전송

### 핵심 플로우

1. **CONNECT 요청 수신** → 호스트 확인
2. **터널 모드 확인**: 대상 호스트가 터널 모드 대상인지 확인
3. **처리 방식 선택**:
   - 터널 모드: 직접 TCP 터널 생성
   - 일반 모드: TLS 버전 감지 후 적절한 핸들러 선택
4. **TLS 처리** (일반 모드):
   - TLS 1.0/1.1: `HybridTlsHandler::handle_with_native_tls_upgraded()`
   - TLS 1.2+: `HybridTlsHandler::handle_with_rustls_upgraded()`
5. **인증서 생성**: PKCS12 형식으로 native-tls용 인증서 생성
6. **TLS 핸드셰이크**: 선택된 라이브러리로 핸드셰이크 수행

## 📊 아키텍처

### TLS 핸드셰이크 플로우

```mermaid
sequenceDiagram
    participant Client as TLS Client
    participant Proxy as Cheolsu Proxy
    participant Detector as TLS Version Detector
    participant Hybrid as HybridTlsHandler
    participant Rustls as rustls
    participant Native as native-tls
    participant Server as Target Server

    Client->>Proxy: CONNECT request
    Proxy->>Proxy: Check if tunnel mode required

    alt Tunnel Mode (Apple services)
        Proxy->>Client: 200 Connection Established
        Proxy->>Server: Direct TCP connection
        Proxy->>Proxy: Spawn tunnel task
        Note over Proxy: copy_bidirectional()
        Note over Client,Server: Direct data flow through tunnel
    else Normal TLS Mode
        Proxy->>Client: 200 Connection Established
        Client->>Proxy: ClientHello (TLS handshake)

        Proxy->>Detector: detect_tls_version(buffer)
        Detector-->>Proxy: TLS version (1.0/1.1/1.2/1.3)

        alt TLS 1.0 or 1.1
            Proxy->>Hybrid: handle_with_native_tls_upgraded()
            Hybrid->>Native: Generate PKCS12 certificate
            Native-->>Hybrid: PKCS12 identity
            Hybrid->>Native: TlsAcceptor.accept()
            Native-->>Hybrid: TLS stream
            Hybrid-->>Proxy: NativeTls stream
        else TLS 1.2 or 1.3
            Proxy->>Hybrid: handle_with_rustls_upgraded()
            Hybrid->>Rustls: Generate rustls certificate
            Rustls-->>Hybrid: ServerConfig
            Hybrid->>Rustls: TlsAcceptor.accept()
            Rustls-->>Hybrid: TLS stream
            Hybrid-->>Proxy: Rustls stream
        end

        Proxy->>Client: TLS handshake complete
        Note over Client,Proxy: Secure communication established
    end
```

### PKCS12 인증서 생성 플로우

```mermaid
flowchart TD
    A[rcgen Certificate] --> B[DER format]
    B --> C[OpenSSL X509]
    C --> D[OpenSSL PKey]
    D --> E[PKCS12 Builder]
    E --> F[PKCS12 DER]
    F --> G[native-tls Identity]
    G --> H[TlsAcceptor]

    style A fill:#e1f5fe
    style G fill:#c8e6c9
    style H fill:#c8e6c9
```

### 하이브리드 TLS 핸들러 구조

```mermaid
graph TB
    subgraph "HybridTlsHandler"
        A[handle_tls_connection_upgraded]
        B[TlsVersionDetector]
        C{Version Check}
        D[handle_with_rustls_upgraded]
        E[handle_with_native_tls_upgraded]
    end

    subgraph "Certificate Authority"
        F[RcgenAuthority]
        G[OpensslAuthority]
    end

    subgraph "TLS Libraries"
        H[rustls]
        I[native-tls]
    end

    A --> B
    B --> C
    C -->|TLS 1.2/1.3| D
    C -->|TLS 1.0/1.1| E
    D --> F
    D --> H
    E --> G
    E --> I

    style A fill:#ffecb3
    style C fill:#f3e5f5
    style H fill:#e8f5e8
    style I fill:#e8f5e8
```

## 📁 주요 구현 파일

### 1. CertificateAuthority 트레이트 확장

- `proxyapi_v2/src/certificate_authority/mod.rs`
- `gen_pkcs12_identity()` 메서드 추가

### 2. PKCS12 인증서 생성

- `proxyapi_v2/src/certificate_authority/rcgen_authority.rs`
- `proxyapi_v2/src/certificate_authority/openssl_authority.rs`
- rcgen/OpenSSL 인증서를 PKCS12로 변환

### 3. 하이브리드 TLS 핸들러

- `proxyapi_v2/src/hybrid_tls_handler.rs`
- TLS 버전 감지 및 적절한 핸들러 선택
- Upgraded 스트림 완벽 지원

### 4. 프록시 통합

- `proxyapi_v2/src/proxy/internal.rs`
- 기존 rustls 로직과 하이브리드 핸들러 통합

## 🚀 사용 방법

### 빌드

```bash
cargo build --package proxyapi_v2 \
  --features "native-tls-client,rcgen-ca,openssl-ca"
```

### 테스트

```bash
cargo run --example tls_hybrid_test \
  --features "native-tls-client,rcgen-ca,openssl-ca" \
  --package proxyapi_v2
```

## 📊 테스트 결과

```
📋 TLS 버전 감지 테스트:
--------------------------
  TLS 1.0 → "TLS 1.0" (native-tls) ✅
  TLS 1.1 → "TLS 1.1" (native-tls) ✅
  TLS 1.2 → "TLS 1.2" (rustls) ✅
  TLS 1.3 → "TLS 1.3" (rustls) ✅
```

## 🔍 기술적 세부사항

### PKCS12 변환 플로우

```
rcgen Certificate (DER)
→ openssl::x509::X509
→ openssl::pkcs12::Pkcs12
→ native_tls::Identity
```

### TLS 버전 감지

```rust
// TLS 레코드 헤더에서 버전 추출
let version_bytes = [buffer[3], buffer[4]];
match version_bytes {
    [0x03, 0x01] => Some(TlsVersion::Tls10),
    [0x03, 0x02] => Some(TlsVersion::Tls11),
    [0x03, 0x03] => Some(TlsVersion::Tls12),
    [0x03, 0x04] => Some(TlsVersion::Tls13),
    _ => None,
}
```

## 🛡️ 보안 고려사항

### 레거시 TLS 지원의 위험성

- **TLS 1.0/1.1은 보안상 취약점**이 있음
- **RC4, MD5, SHA-1** 등 취약한 암호화 알고리즘 사용
- **BEAST, POODLE** 등의 공격에 취약

### 권장사항

1. **최신 TLS 버전 사용**: 가능한 한 TLS 1.2 이상 사용
2. **레거시 클라이언트 업그레이드**: 오래된 클라이언트 업데이트 권장
3. **보안 정책 수립**: 레거시 TLS 사용에 대한 명확한 정책 수립

## 🔧 문제 해결

### 터널 모드 관련 문제

**증상**: Apple 서비스나 특정 호스트에 연결 실패

**해결방법**:

1. 터널 모드 대상 호스트 목록 확인
2. 터널 이벤트 로그 확인 (`🚇 [TUNNEL-EVENT]` 로그)
3. `tunnel_event_sender`가 올바르게 설정되었는지 확인
4. 터널 모드 타임아웃 설정 확인 (기본 5분)

### TLS 핸드셰이크 실패

**증상**: TLS 1.0/1.1 클라이언트가 연결되지 않음

**해결방법**:

1. native-tls 기능이 활성화되었는지 확인
2. OpenSSL 라이브러리가 설치되었는지 확인
3. PKCS12 인증서 생성이 성공했는지 확인

### 인증서 오류

**증상**: "certificate verify failed" 오류

**해결방법**:

1. CA 인증서가 올바르게 설치되었는지 확인
2. 인증서 체인 검증 로직 확인
3. 시스템 시간이 올바른지 확인

### 성능 문제

**증상**: TLS 핸드셰이크가 느림

**해결방법**:

1. 인증서 생성 캐싱 구현
2. 연결 풀링 최적화
3. 메모리 사용량 모니터링

## 📈 향후 개선 계획

### 성능 최적화

- [ ] 인증서 생성 캐싱
- [ ] 연결 풀링 구현
- [ ] 메모리 사용량 최적화

### 보안 강화

- [ ] TLS 1.0/1.1 사용 경고
- [ ] 취약한 암호화 알고리즘 감지
- [ ] 보안 정책 강제 적용

### 모니터링

- [ ] TLS 버전별 통계
- [ ] 보안 이벤트 로깅
- [ ] 성능 메트릭 수집
