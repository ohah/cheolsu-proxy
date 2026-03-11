# TLS 인증서 고도화 전략 비교: cheolsu-proxy vs mitmproxy

> 작성일: 2026-03-11
> 목적: mitmproxy 레퍼런스 분석을 통한 인증서/TLS 전략 개선 방향 도출

---

## 1. 현재 cheolsu-proxy가 우위인 부분

| 항목                   | cheolsu-proxy                         | mitmproxy                              |
| ---------------------- | ------------------------------------- | -------------------------------------- |
| TLS Passthrough        | 실패 횟수 기반 자동 바이패스          | `ignore_connection` 플래그 (수동 설정) |
| TLS 엔진               | rustls + OpenSSL 하이브리드 이중 엔진 | OpenSSL 단일 엔진                      |
| Apple 서비스 특수 처리 | 전용 암호화 스위트/타임아웃 적용      | 없음                                   |
| 인증서 캐시            | moka 기반 (TTL 180일, 용량 무제한)    | 인메모리 dict (100개 제한)             |

---

## 2. mitmproxy에는 있지만 우리에게 없는 것

### 2.1 Upstream Certificate Sniffing (상류 인증서 스니핑)

mitmproxy의 핵심 전략. `upstream_cert` 옵션 (기본값: True).

**동작 흐름:**

```
[클라이언트] → ClientHello → [프록시] → 상류 서버에 먼저 연결
                                        → 실제 인증서의 CN, SAN, 조직명 복사
                                        → 이 정보로 위조 인증서 생성
                              ← 위조 인증서로 클라이언트 응답
```

**세부 사항:**

- 상류 인증서에서 CN, 모든 SAN(altnames), 조직명을 복사
- CRL Distribution Points도 추출하여 위조 인증서에 포함
- 최종 인증서의 SAN = 상류 인증서 SAN + 클라이언트 SNI + 로컬 소켓 주소 + 서버 주소 (중복 제거)
- `add_upstream_certs_to_client_chain` 옵션: 상류 서버의 전체 인증서 체인을 클라이언트에게 전달 가능

**기대 효과:** 호스트명 기반 인증서 생성보다 위조 인증서 품질이 크게 향상되어 TLS 실패율 감소.

### 2.2 Eager vs Lazy 연결 전략

| 전략      | 설명                                      | 장점                    | 단점               |
| --------- | ----------------------------------------- | ----------------------- | ------------------ |
| **Eager** | 클라이언트 핸드셰이크 전에 서버 먼저 연결 | 정확한 인증서 복제 가능 | 지연시간 증가      |
| **Lazy**  | 클라이언트 SNI/ALPN 확보 후 서버 연결     | 빠른 응답               | 인증서 정보 제한적 |

- Eager: `establish_server_tls_first = True` → 상류 인증서를 먼저 가져와서 위조 인증서 생성
- Lazy: `wait_for_clienthello = True` → 클라이언트의 ClientHello에서 SNI/ALPN 정보를 먼저 확보

### 2.3 ALPN 미러링 (5단계 우선순위)

mitmproxy의 ALPN 프로토콜 선택 로직:

1. 클라이언트가 ALPN 전송 시 → 서버 옵션과 호환되면 사용
2. 서버가 ALPN 광고 시 → 서버 옵션에 있으면 반환
3. 서버가 명시적 거부 시 → 클라이언트에게도 거부 미러링
4. h2 비활성 시 → HTTP/1만 사용, 클라이언트 선호 순서 존중
5. 매칭 실패 시 → `NO_OVERLAPPING_PROTOCOLS` 반환

### 2.4 mTLS (상호 인증서) 지원

- `client_certs`: 프록시 → 서버 간 클라이언트 인증서 전달 (파일 또는 디렉토리) — **✅ 기본 구현 완료**
- `request_client_cert`: 클라이언트 → 프록시 간 mTLS 요청 — **✅ 구현 완료**

> **cheolsu-proxy 현황:** 클라이언트 인증서 전달, 도메인별 인증서, 인증서 정보 UI 표시, `request_client_cert` 모두 구현 완료.

### 2.5 도메인별 커스텀 인증서

- `--certs [domain=]path`로 특정 도메인에 사용자 제공 리프 인증서 사용 가능
- 와일드카드 패턴 매칭 지원
- 기업 내부 서버 등 특수 인증서가 필요한 환경에서 유용

### 2.6 TLS 이벤트 훅 아키텍처

mitmproxy는 7개의 TLS 이벤트 훅을 제공:

| 훅                       | 설명                                                                                                      |
| ------------------------ | --------------------------------------------------------------------------------------------------------- |
| `tls_clienthello`        | ClientHello 수신 시. `ignore_connection`으로 패스스루, `establish_server_tls_first`로 서버 우선 연결 결정 |
| `tls_start_client`       | 클라이언트 TLS 협상 시작 전                                                                               |
| `tls_start_server`       | 서버 TLS 협상 시작 전                                                                                     |
| `tls_established_client` | 클라이언트 핸드셰이크 성공                                                                                |
| `tls_established_server` | 서버 핸드셰이크 성공                                                                                      |
| `tls_failed_client`      | 클라이언트 핸드셰이크 실패                                                                                |
| `tls_failed_server`      | 서버 핸드셰이크 실패                                                                                      |

**ClientHelloData 속성:** `sni`, `cipher_suites`, `alpn_protocols`, `extensions` (type + raw_bytes)

### 2.7 TLS 버전 및 암호화 스위트 세분화

- 클라이언트/서버 방향별 TLS 버전 범위 독립 설정 (`tls_version_client_min/max`, `tls_version_server_min/max`)
- 클라이언트/서버 방향별 암호화 스위트 독립 설정 (`ciphers_client`, `ciphers_server`)
- 기본 27개 암호 스위트 (Mozilla 설정 가이드라인 기반, GCM과 ChaCha20 우선)
- ECDH 곡선 독립 설정 (`tls_ecdh_curve_client/server`)

### 2.8 인증서 생성 세부 사항

| 항목                     | mitmproxy                    | 비고                    |
| ------------------------ | ---------------------------- | ----------------------- |
| CA 유효기간              | -2일 ~ +10년                 | 시계 오차 대비 -2일     |
| 리프 유효기간            | -2일 ~ +365일                |                         |
| CN 길이 제한             | 64자 미만                    | RFC 준수                |
| SAN critical 설정        | subject 비어있을 때 critical | RFC 5280 4.2.1.6        |
| Authority Key Identifier | 포함                         |                         |
| Subject Key Identifier   | 의도적 생략                  | SChannel 호환성 (#6494) |
| 파일 권한                | `umask_secret()` (모드 0o77) | 보안                    |

---

## 3. 양쪽 모두 미지원인 고급 영역

| 기능                           | 설명                            | 필요 기술                                   |
| ------------------------------ | ------------------------------- | ------------------------------------------- |
| TLS 지문 위장 (JA3/JA4 스푸핑) | ClientHello를 브라우저처럼 위장 | uTLS 등 별도 구현                           |
| ECH (Encrypted Client Hello)   | 암호화된 SNI 처리               | 표준 아직 발전 중                           |
| 인증서 피닝 바이패스           | 앱 레벨 피닝 우회               | 외부 도구 의존 (`apk-mitm`, `objection` 등) |

---

## 4. 구현 우선순위 제안

도입 효과가 가장 큰 순서로 정리:

### P0: Upstream Certificate Sniffing — ✅ 구현 완료

- **효과:** 위조 인증서 품질 향상 → TLS 실패율 감소 → passthrough 전환 감소
- **난이도:** 중간 (서버 먼저 연결 → 인증서 파싱 → SAN/CN 복사)
- **구현 포인트:** `dummy_cert()` 생성 시 상류 인증서 정보 활용
- **구현 커밋:** `2e759ca`, `fbc2307`

### P1: Eager/Lazy 연결 전략

- **효과:** P0과 시너지. 상황에 따라 최적 전략 선택 가능
- **난이도:** 중간~높음 (연결 타이밍 제어 로직 변경)
- **구현 포인트:** ClientHello 수신 후 서버 연결 타이밍 분기

### P2: ALPN 미러링 고도화 — ✅ 구현 완료

- **효과:** HTTP/2 환경에서 호환성 향상
- **난이도:** 낮음
- **구현 포인트:** 클라이언트/서버 ALPN 협상 중계 로직 추가
- **구현 커밋:** `fbc2307`

### P3: 도메인별 커스텀 인증서 — ✅ 구현 완료

- **효과:** 기업 내부 서버, 특수 인증서 환경 대응
- **난이도:** 낮음
- **구현 포인트:** 설정 파일에서 도메인-인증서 매핑 로드
- **현황:** `DomainCertResolver` (rustls `ResolvesClientCert` 구현), GUI 도메인별 인증서 관리 UI

### P4: mTLS 지원 — ✅ 구현 완료

- **효과:** 클라이언트 인증서가 필요한 서버 환경 대응
- **난이도:** 중간
- **현황:** 인증서 정보 UI 표시, 도메인별 인증서, `request_client_cert` 모두 구현 완료

---

## 5. 미구현 기능 상세 분석 및 구현 계획

> 작성일: 2026-03-11

### 5.1 난이도 및 일정 요약

| 기능                   | 난이도 | 예상 일수   | 리스크 | 주요 수정 파일                                                            |
| ---------------------- | ------ | ----------- | ------ | ------------------------------------------------------------------------- | ----------- |
| 인증서 생성 세부 사항  | 낮음   | 1-2일       | 낮음   | `generator.rs`, `rcgen_authority.rs`, `openssl_authority.rs`              | **✅ 완료** |
| TLS 이벤트 훅 (7개)    | 중~상  | 4-5일       | 중~상  | 신규 `tls_event.rs`, `hybrid_tls_handler.rs`, `internal.rs`, `context.rs` | **✅ 완료** |
| TLS 버전/암호화 세분화 | 중간   | 3-4일       | 중간   | 신규 `tls_config.rs`, `hybrid_tls_handler.rs`, `context.rs`               | **✅ 완료** |
| Eager/Lazy 연결 전략   | 중간   | 2-3일       | 중간   | `internal.rs`, `upstream_cert.rs`, `context.rs`                           | 미구현      |
| **합계**               |        | **10-14일** |        |                                                                           |             |

### 5.2 권장 구현 순서

한번에 4개를 동시 구현하면 영향 범위가 겹쳐(특히 `hybrid_tls_handler.rs`, `internal.rs`) 충돌과 테스트 복잡도가 급증하므로, **2라운드로 분할** 권장:

- **라운드 1:** 인증서 생성 세부 사항 + TLS 이벤트 훅 — **✅ 완료**
  - 이유: 인증서 세부 사항은 독립적이고 간단. 이벤트 훅은 인프라성 기능이라 먼저 깔아두면 나머지 기능의 디버깅/관찰이 쉬워짐
- **라운드 2:** TLS 버전/암호화 세분화 + Eager/Lazy 연결 전략
  - TLS 버전/암호화 세분화 — **✅ 완료** (`tls_config.rs`, `hybrid_tls_handler.rs`, `context.rs`)
  - Eager/Lazy 연결 전략 — 미구현

### 5.3 인증서 생성 세부 사항 — ✅ 구현 완료

**구현 항목:**

| 항목                     | 이전          | mitmproxy                      | 변경 내용                                                                   | 상태 |
| ------------------------ | ------------- | ------------------------------ | --------------------------------------------------------------------------- | ---- |
| CA NOT_BEFORE            | -60초         | -2일 (-172800초)               | `mod.rs` NOT_BEFORE_OFFSET 변경                                             | ✅   |
| 리프 NOT_BEFORE          | -60초         | -2일                           | 동일하게 변경                                                               | ✅   |
| CN 길이 제한             | 미적용        | 64자 미만 (RFC 준수)           | `truncate_cn()` 헬퍼로 모든 CN 설정에 적용                                  | ✅   |
| SAN critical 조건        | 항상 critical | subject 비어있을 때만 critical | OpenSSL: `name.entries().count() == 0` 조건부 설정                          | ✅   |
| Authority Key Identifier | 미포함        | 포함                           | OpenSSL: `AuthorityKeyIdentifier` 확장 추가, rcgen: `signed_by()` 자동 생성 | ✅   |
| Subject Key Identifier   | 미포함        | 의도적 생략 (SChannel 호환)    | 생략 유지 (mitmproxy와 동일)                                                | ✅   |
| 파일 권한                | rcgen만 적용  | `umask_secret()` (0o77)        | OpenSSL CA 생성 시에도 키 파일 `0o600` 권한 설정 추가                       | ✅   |

**수정 파일:**

- `crates/proxyapi_v2/src/certificate_authority/mod.rs` — NOT_BEFORE_OFFSET 상수 변경
- `crates/proxyapi_v2/src/certificate_authority/rcgen_authority.rs` — AKI 추가, CN 길이 제한, SAN critical 조건
- `crates/proxyapi_v2/src/certificate_authority/openssl_authority.rs` — AKI 추가, CN 길이 제한, SAN critical 조건
- `crates/proxyapi_v2/src/certificate_authority/generator.rs` — CA 키 파일 권한 설정

### 5.4 TLS 이벤트 훅 아키텍처 — ✅ 구현 완료

mitmproxy의 7개 TLS 이벤트 훅에 대응하는 channel 기반 이벤트 시스템 구현:

**훅 매핑:**

| mitmproxy 훅               | cheolsu-proxy 대응              | 발생 시점                   | 데이터                                                 |
| -------------------------- | ------------------------------- | --------------------------- | ------------------------------------------------------ |
| `tls_clienthello`          | `on_client_hello`               | ClientHello 분석 후         | SNI, cipher suites, ALPN, extensions, complexity_score |
| —                          | `on_strategy_selected`          | 전략 결정 후 (cheolsu 고유) | TlsStrategy (Rustls/OpenSSL), 결정 사유                |
| `tls_start_server`         | `on_server_connection_starting` | 서버 TLS 협상 시작 전       | authority, connection_strategy                         |
| `tls_start_client`         | `on_fake_cert_generating`       | 위조 인증서 생성 전         | authority, upstream_cert_info, cache 여부              |
| `tls_established_server`   | `on_upstream_cert_sniffed`      | 상류 인증서 스니핑 후       | UpstreamCertInfo, 소요시간, 성공여부                   |
| `tls_established_client`   | `on_handshake_completed`        | 클라이언트 핸드셰이크 성공  | authority, TLS 버전, cipher, 소요시간                  |
| `tls_failed_client/server` | `on_handshake_failed`           | 핸드셰이크 실패             | authority, 에러 정보, 방향(client/server)              |

**구현 방식:** channel 기반 (`tokio::sync::mpsc`)

기존 `tunnel_event_sender` 패턴과 일관성을 유지하며, `try_send`로 non-blocking을 보장합니다.

```rust
// crates/proxyapi_v2/src/tls_event.rs
pub enum TlsEvent {
    ClientHelloAnalyzed { authority, tls_info },
    StrategySelected { authority, strategy, tls_info },
    ServerConnectionStarting { authority },
    UpstreamCertSniffed { authority, cert_info },
    FakeCertGenerating { authority, has_upstream_cert },
    HandshakeCompleted { authority, strategy, duration },
    HandshakeFailed { authority, strategy, error, duration },
}

pub type TlsEventSender = tokio::sync::mpsc::Sender<TlsEvent>;
pub fn emit_tls_event(sender: &Option<TlsEventSender>, event: TlsEvent) { ... }
```

**이벤트 emit 위치:**

- `hybrid_tls_handler.rs` — `analyze_tls_connection()` 후 → `ClientHelloAnalyzed`, 전략 결정 후 → `StrategySelected`, 핸드셰이크 성공/실패 → `HandshakeCompleted`/`HandshakeFailed`
- `hybrid_tls_handler.rs` — `gen_server_config()`/`gen_openssl_context()` 호출 전 → `FakeCertGenerating`
- `internal.rs` — `sniff_upstream_cert()` 호출 전/후 → `ServerConnectionStarting`, `UpstreamCertSniffed`

**사용법:** `ProxyContext.tls_event_sender`에 `tls_event_channel(buffer)` 로 생성한 sender를 설정하면 이벤트를 수신할 수 있습니다. 현재는 인프라만 구축된 상태이며, 라운드 2에서 Eager/Lazy 전략 자동 조정 등에 활용 예정.

### 5.5 TLS 버전/암호화 스위트 세분화 — ✅ 구현 완료

클라이언트↔프록시, 프록시↔서버 방향별 독립 TLS 설정:

**설정 구조:**

```rust
// 신규 파일: crates/proxyapi_v2/src/tls_config.rs (~400줄)

pub struct DirectionalTlsConfig {
    pub version_min: TlsVersion,       // 기본: TLS 1.2
    pub version_max: TlsVersion,       // 기본: TLS 1.3
    pub cipher_suites: Option<Vec<u16>>, // None이면 기본값 사용
    pub ecdh_curves: Option<Vec<String>>,
}

pub struct TlsConfigRule {
    pub domain_pattern: String,         // "*.apple.com", "api2.cursor.sh"
    pub client_direction: DirectionalTlsConfig,  // 클라이언트 → 프록시
    pub server_direction: DirectionalTlsConfig,  // 프록시 → 서버
    pub priority: u32,
}

pub struct TlsConfigManager {
    rules: Vec<TlsConfigRule>,         // 우선순위 정렬
    default_client: DirectionalTlsConfig,
    default_server: DirectionalTlsConfig,
}
```

**현재 하드코딩된 부분:**

- `rcgen_authority.rs:301-304` — TLS 1.2/1.3 고정
- `rcgen_authority.rs:546` — OpenSSL cipher 문자열 `"@SECLEVEL=0:ALL:!aNULL:!eNULL"` 고정
- `hybrid_tls_handler.rs:429-436` — Apple 서비스 전용 cipher 하드코딩

**변경 포인트:**

- `gen_server_config()` 호출 시 `TlsConfigManager`에서 도메인 매칭 → 방향별 설정 주입
- OpenSSL `SslContext` 생성 시 cipher 문자열 동적 구성
- Apple 서비스 특수 처리를 규칙 기반으로 전환 (하드코딩 → 설정)

**제약사항:**

- rustls는 `ServerConfig` 레벨에서만 cipher 선택 가능 (연결별 동적 변경 불가 → 도메인별 ServerConfig 캐시 필요)
- 설정 변경 시 캐시 무효화 전략 필요

### 5.6 Eager/Lazy 연결 전략

**현재 동작:** Lazy — 서버 연결은 스니핑 시점 또는 HTTP 요청 처리 시점에 발생

**구현 설계:**

```rust
pub enum ConnectionStrategy {
    Lazy,                // 현재 동작: 필요 시 서버 연결
    Eager,               // ClientHello 직후 서버 먼저 연결
    EagerWithFallback,   // Eager 시도 → 실패 시 Lazy 폴백
}
```

**Eager 흐름:**

```
[클라이언트] → ClientHello → [프록시]
                                ├→ 백그라운드: 서버 TCP+TLS 연결 시작
                                ├→ 상류 인증서 추출 (Eager 연결 재사용)
                                ├→ 위조 인증서 생성
                                └→ 클라이언트 TLS 핸드셰이크
                              → HTTP 요청 시 기존 서버 연결 재사용
```

**수정 파일:**

- `proxy/context.rs` — `ConnectionStrategy` 필드 추가
- `proxy/builder.rs` — `with_connection_strategy()` 빌더 메서드 추가
- `proxy/internal.rs` — `process_connect()` 에서 전략 분기, 백그라운드 서버 연결 스폰
- `upstream_cert.rs` — Eager 연결에서 인증서 스니핑 재사용

**핵심 과제:**

- 연결 수명 관리 (idle timeout, 재연결)
- Eager 연결 실패 시 에러 핸들링
- 미사용 연결의 메모리 오버헤드
- `sniff_upstream_cert()`와 Eager 연결 간 TCP 스트림 공유

---

## 6. 구현 단계 상세 (라운드 1)

### Phase 1: 인증서 생성 세부 개선

충돌 최소화를 위해 Feature 2(TLS 이벤트 훅)보다 먼저 완료한다.

#### Step 1-1: NOT_BEFORE_OFFSET 변경 + truncate_cn 헬퍼 추가

**파일**: `certificate_authority/mod.rs`

- `NOT_BEFORE_OFFSET: i64 = 60` → `NOT_BEFORE_OFFSET: i64 = 172_800` (2일 = 172800초)
- `truncate_cn(cn: &str) -> String` 헬퍼 함수 추가 (char 경계 존중: `cn.chars().take(64).collect()`)

> 기존 사용처(`rcgen_authority.rs`, `openssl_authority.rs`)가 모두 빼기 연산이므로 상수만 변경하면 자동 적용.

#### Step 1-2: OpenSSL CA 키 파일 권한 설정

**파일**: `certificate_authority/generator.rs`

- rcgen 경로(96-109행)에는 이미 `#[cfg(unix)] set_permissions(0o600)` 적용됨
- OpenSSL 경로(`generate_openssl_ca`)에 동일한 권한 설정 추가 (키 파일 write 직후)

#### Step 1-3: openssl_authority.rs 개선

- `gen_cert()`: CN truncation 적용, AKI extension 추가, SAN critical 조건부 설정
- `gen_openssl_context()` 내 spawn_blocking: 동일하게 적용

```rust
// AKI 추가
use openssl::x509::extension::AuthorityKeyIdentifier;
let aki = AuthorityKeyIdentifier::new()
    .keyid(true)
    .build(&x509_builder.x509v3_context(Some(&ca_cert), None))?;
x509_builder.append_extension(aki)?;

// SAN critical 조건부
let mut san_builder = SubjectAlternativeName::new();
if name.entries().count() == 0 {
    san_builder.critical();
}
```

#### Step 1-4: rcgen_authority.rs 개선

- `gen_cert()`: CN truncation 적용
- rcgen은 `signed_by()` 호출 시 AKI를 자동 생성하므로 명시적 설정 불필요 (rcgen 버전 확인 필요)
- `gen_openssl_context()` 내 spawn_blocking: CN truncation + AKI + SAN critical 동일 적용

### Phase 2: TLS 이벤트 훅 아키텍처

#### 설계 결정: channel 기반 (trait object 대신)

기존 `tunnel_event_sender: Option<mpsc::Sender<RequestInfo>>` 패턴과 일관성을 유지하기 위해 channel 기반으로 구현:

- `TlsEventSender = tokio::sync::mpsc::Sender<TlsEvent>` 타입 alias
- non-blocking 보장 (`try_send` 사용)
- 수신자 측에서 별도 태스크로 처리 가능

#### Step 2-1: tls_event.rs 모듈 생성

**신규 파일**: `crates/proxyapi_v2/src/tls_event.rs` (~150줄)

```rust
pub enum TlsEvent {
    ClientHelloAnalyzed { authority, tls_info },
    StrategySelected { authority, strategy, tls_info },
    ServerConnectionStarting { authority, strategy },
    UpstreamCertSniffed { authority, cert_info: Option<UpstreamCertInfo> },
    FakeCertGenerating { authority, upstream_cert },
    HandshakeCompleted { authority, strategy, duration },
    HandshakeFailed { authority, strategy, error, duration },
}

pub type TlsEventSender = tokio::sync::mpsc::Sender<TlsEvent>;

/// non-blocking emit 헬퍼
pub fn emit_tls_event(sender: &Option<TlsEventSender>, event: TlsEvent) {
    if let Some(ref s) = sender {
        let _ = s.try_send(event);
    }
}
```

#### Step 2-2: lib.rs + context.rs + builder.rs 수정

- `lib.rs`: `pub mod tls_event;` 추가
- `context.rs`: `tls_event_sender: Option<TlsEventSender>` 필드 추가
- `builder.rs`: `with_tls_event_sender()` 빌더 메서드 추가 (선택적)

#### Step 2-3: HybridTlsHandler에 sender 필드 추가

- `HybridTlsHandler` 구조체에 `tls_event_sender: Option<TlsEventSender>` 추가
- `new()` 시그니처에 sender 파라미터 추가
- CA 구조체(RcgenAuthority/OpensslAuthority)는 변경하지 않음 — `FakeCertGenerating` 이벤트는 `HybridTlsHandler` 내에서 `gen_server_config()`/`gen_openssl_context()` 호출 전에 emit

#### Step 2-4: hybrid_tls_handler.rs 이벤트 emit 삽입

`HybridTlsHandler`에 `tls_event_sender` 필드 추가 후:

| 위치                            | 이벤트                |
| ------------------------------- | --------------------- |
| `analyze_tls_connection()` 직후 | `ClientHelloAnalyzed` |
| `determine_tls_strategy()` 직후 | `StrategySelected`    |
| 핸드셰이크 성공                 | `HandshakeCompleted`  |
| 핸드셰이크 실패                 | `HandshakeFailed`     |

모든 emit은 `try_send`로 non-blocking.

#### Step 2-5: internal.rs 이벤트 emit 삽입

- `HybridTlsHandler::new()` 호출 시 sender 전달
- `sniff_upstream_cert()` 전: `ServerConnectionStarting` emit
- `sniff_upstream_cert()` 후: `UpstreamCertSniffed` emit

### 위험 요소

1. **rcgen AKI 자동 설정 여부**: rcgen 버전에 따라 `signed_by()`가 AKI를 자동 추가하는지 확인 필요
2. **HybridTlsHandler::new() 시그니처 변경**: `internal.rs` 호출부 동시 수정 필수
3. **spawn_blocking 내 sender 사용**: `tokio::sync::mpsc::Sender`는 `Send`이므로 문제 없음
4. **NOT_BEFORE_OFFSET 변경**: 기존 캐시된 인증서와 새 인증서의 not_before 시점이 달라지지만, 기능에 영향 없음

---

## 7. 참고 자료

- [mitmproxy 인증서 문서](https://docs.mitmproxy.org/stable/concepts-certificates/)
- [mitmproxy TLS 동작 원리](https://docs.mitmproxy.org/stable/concepts-howmitmproxyworks/)
- mitmproxy 소스: `net/tls.py`, `certs.py`, `tls.py` (레이어)
