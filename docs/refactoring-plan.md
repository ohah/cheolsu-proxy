# Rust 코드베이스 리팩토링 계획

> 작성일: 2026-03-09
> 대상: `crates/` 디렉토리 (총 ~27,000줄, 7개 크레이트)

---

## 목차

1. [현황 요약](#1-현황-요약)
2. [Phase 1: unwrap/expect 정리](#2-phase-1-unwrapexpect-정리)
3. [Phase 2: 테스트 커버리지 확대](#3-phase-2-테스트-커버리지-확대)
4. [Phase 3: 거대 함수 분해](#4-phase-3-거대-함수-분해)
5. [Phase 4: 구조체 책임 분리](#5-phase-4-구조체-책임-분리)
6. [Phase 5: 동시성 개선](#6-phase-5-동시성-개선)
7. [작업 순서 및 의존 관계](#7-작업-순서-및-의존-관계)

---

## 1. 현황 요약

### 크레이트 구조

```
proxy_v2_models (핵심 데이터 모델)
    ↑
proxyapi_v2 (MITM 프록시 엔진)
    ↑
proxy_daemon (데몬 프로세스) ← scripting (TS/JS 엔진)
    ↑
├── tui (Terminal UI)
├── mcp_server (MCP 서버)
└── desktop/src-tauri (Tauri 데스크톱)
```

순환 의존성 없음. 계층 구조 양호.

### 테스트 현황

| 크레이트 | 단위 테스트 | 통합 테스트 | 합계 |
|---------|-----------|-----------|------|
| proxy_v2_models | 114 | 0 | 114 |
| proxyapi_v2 | 73 | 81 | 154 |
| proxy_daemon | 50 | 0 | **50** |
| mcp_server | 31 | 0 | 31 |
| tui | 29 | 0 | 29 |
| scripting | 19 | 0 | 19 |
| **합계** | **316** | **81** | **397** |

### 주요 문제 파일 (줄 수 기준)

| 파일 | 줄 수 | 핵심 문제 |
|------|-------|----------|
| `mcp_server/src/main.rs` | 1,251 | 모든 MCP 도구가 단일 파일 |
| `proxyapi_v2/src/hybrid_tls_handler.rs` | 1,149 | 370줄짜리 함수 |
| `proxy_daemon/src/handler.rs` | 965 | God Object (11개 필드) |
| `proxy_v2_models/src/data_type.rs` | 897 | 하나의 impl에 60+개 메서드 |
| `proxyapi_v2/src/proxy/internal.rs` | 832 | 프록시 내부 로직 혼재 |
| `proxy_v2_models/src/har.rs` | 801 | HAR 변환 + 테스트 |
| `tui/src/app/forms.rs` | 788 | TUI 폼 로직 |

---

## 2. Phase 1: unwrap/expect 정리

**목표:** 프로덕션 코드에서 panic 가능성 제거
**난이도:** 낮음 | **위험도:** 낮음 | **효과:** 런타임 안정성 즉시 개선

### 2.1 심각도: 높음 (데몬 크래시 직결)

#### proxy_daemon/src/daemon.rs

```rust
// Line 114 - 런타임 생성 실패 시 데몬 즉사
expect("Failed to create tokio runtime")

// Line 167 - 잘못된 host:port 입력 시 panic
expect("Invalid host:port")

// Line 225 - 시그널 핸들러 등록 실패 시 panic
expect("Failed to register SIGTERM handler")
```

**조치:** `anyhow::Result` 반환으로 변경, 호출부에서 에러 로깅 후 graceful shutdown

#### proxy_daemon/src/handler.rs (10건)

```rust
// Line 668 - 에러 응답 빌드 실패
expect("Failed to build error response")

// Line 134, 143, 162, 267, 273, 460, 631, 806, 813
// 다양한 unwrap() 호출
```

**조치:** `?` 연산자로 전환, `HttpHandler` trait 반환 타입이 `Result`이므로 전파 가능

#### scripting/src/handle.rs

```rust
// Line 69
expect("Failed to create scripting runtime")
```

**조치:** `Result` 반환으로 변경, 스크립트 로드 실패를 사용자에게 알림

### 2.2 심각도: 중간 (Lock Poisoning)

#### mcp_server/src/main.rs (19건)

```rust
// Line 50, 58, 66, 230, 243, 278, 324, 400, 515, 583, 600, 668-671, 682-684, 713
self.store.transactions.lock().unwrap()
self.store.ws_messages.lock().unwrap()
self.store.rules.lock().unwrap()
```

**조치:** 두 가지 선택지
- (A) `parking_lot::Mutex` 사용 — lock poisoning 자체가 발생하지 않음
- (B) `.lock().unwrap_or_else(|e| e.into_inner())` — poisoned lock 복구

**권장:** (A) `parking_lot::Mutex`로 전환. API 동일하고 성능도 더 좋음.

### 2.3 심각도: 중간 (헤더 파싱)

#### proxyapi_v2/src/tunnel_event.rs (16건)

```rust
// Line 104-162
header_value.to_str().unwrap()
value.parse().unwrap()
```

**조치:** `Option`/`Result` 체이닝으로 변경, 파싱 실패 시 기본값 사용

#### proxyapi_v2/src/proxy/middleware.rs (5건)

```rust
// Line 80-94
.parse().unwrap()  // 캐시 컨트롤, 헤더 값 파싱
```

**조치:** `.parse().unwrap_or_default()` 또는 `?` 전파

### 2.4 심각도: 낮음

#### proxy_v2_models/src/file_storage.rs

```rust
// Line 72
let base_cache_dir = cache_dir.parent().unwrap();
```

**조치:** `.parent().ok_or_else(|| anyhow!("cache_dir has no parent"))?`

#### proxy_daemon/src/intercept.rs (9건)

```rust
// Line 121-354
.parse().unwrap()  // 헤더 값 변환
```

**조치:** 실패 시 원본 값 유지 또는 skip 처리

#### tui/src/app/key_handlers.rs, mod.rs (2건)

```rust
// key_handlers.rs Line 314
let form = self.rule_form.as_mut().unwrap();

// mod.rs Line 299
.unwrap()
```

**조치:** `if let Some(form)` 패턴으로 변경

### 2.5 제외 대상

- `tests/` 디렉토리: 테스트 코드에서 unwrap()은 허용 (실패 시 테스트 실패로 처리됨)
- `benches/`, `examples/`: 프로덕션 경로가 아니므로 낮은 우선순위

---

## 3. Phase 2: 테스트 커버리지 확대

**목표:** 리팩토링 안전망 구축
**난이도:** 중간 | **위험도:** 없음 | **효과:** 리팩토링 신뢰도 확보

### 3.1 proxy_daemon 통합 테스트 (최우선)

현재 통합 테스트 0건. 가장 큰 격차.

```
crates/proxy_daemon/tests/
├── daemon_lifecycle_test.rs    # 데몬 시작/종료/재시작
├── request_handler_test.rs     # HTTP 요청 처리 E2E
├── intercept_rules_test.rs     # 인터셉트 규칙 동작
└── websocket_handler_test.rs   # WebSocket 프록시 E2E
```

**테스트 시나리오:**

| 테스트 | 검증 대상 |
|--------|----------|
| 데몬 시작 → HTTP 요청 → 캡처 확인 | 기본 프록시 동작 |
| 인터셉트 규칙 등록 → 요청 차단 확인 | 규칙 엔진 |
| 서버 리플레이 등록 → 매칭 응답 확인 | 리플레이 기능 |
| WS 연결 → 메시지 캡처 확인 | WebSocket 핸들링 |
| 데몬 graceful shutdown | 리소스 정리 |

### 3.2 scripting 통합 테스트

```
crates/scripting/tests/
├── script_lifecycle_test.rs    # 스크립트 로드/언로드/핫리로드
└── hook_execution_test.rs      # onRequest/onResponse 훅 실행
```

### 3.3 mcp_server 통합 테스트

```
crates/mcp_server/tests/
└── mcp_protocol_test.rs        # MCP 도구 호출 및 응답 검증
```

### 3.4 기존 테스트 보강

| 크레이트 | 보강 대상 |
|---------|----------|
| proxy_v2_models | `file_storage.rs` 경로 엣지 케이스 |
| proxyapi_v2 | `hybrid_tls_handler.rs` 핸드셰이크 실패 경로 |
| proxy_daemon | `intercept.rs` 규칙 매칭 엣지 케이스 |

---

## 4. Phase 3: 거대 함수 분해

**목표:** 함수당 80줄 이하, 단일 책임
**난이도:** 중간 | **위험도:** 중간 (Phase 2 테스트 필요) | **효과:** 가독성, 테스트 용이성

### 4.1 `analyze_handshake_failure()` — 370줄 → 4~5개 함수

**파일:** `proxyapi_v2/src/hybrid_tls_handler.rs` (Line 507-876)

**현재:** 진단 수집 + 에러 분류 + 로깅이 한 함수에 혼재

**변경 계획:**

```rust
// Before: 하나의 거대 함수
fn analyze_handshake_failure(&self, stream, error, authority) {
    // 370줄...
}

// After: 책임별 분리
struct HandshakeDiagnostics {
    ssl_state: String,
    error_code: ErrorCode,
    peer_certificate: Option<CertInfo>,
    cipher_info: Option<CipherInfo>,
    protocol_version: Option<String>,
    failure_category: HandshakeFailureCategory,
}

fn diagnose_handshake(stream: &SslStream, error: &ssl::Error) -> HandshakeDiagnostics { ... }
fn categorize_failure(diagnostics: &HandshakeDiagnostics) -> HandshakeFailureCategory { ... }
fn suggest_remediation(category: &HandshakeFailureCategory) -> &str { ... }
fn log_handshake_failure(authority: &Authority, diagnostics: &HandshakeDiagnostics) { ... }
```

### 4.2 `daemon_main()` — 160줄 → 3~4개 함수

**파일:** `proxy_daemon/src/daemon.rs` (Line 120-279)

```rust
// Before
pub fn daemon_main(config: DaemonConfig) {
    // 채널 설정 (20줄)
    // 프록시 빌더 (30줄)
    // 시그널 핸들러 (20줄)
    // 메인 루프 (90줄)
}

// After
fn setup_channels() -> Channels { ... }
fn build_proxy(config: &DaemonConfig, channels: &Channels) -> Result<Proxy> { ... }
fn register_signal_handlers() -> Result<()> { ... }
async fn run_main_loop(proxy: Proxy, channels: Channels) { ... }
```

### 4.3 `handle_message()` — 141줄 → 핸들러 분리

**파일:** `proxy_daemon/src/handler.rs` (Line 706-846)

```rust
// Before
async fn handle_message(&mut self, ctx, msg) -> Option<Message> {
    // WebSocket 타입 분기 (20줄)
    // MQTT 특수 처리 (40줄)
    // 인터셉트 규칙 적용 (30줄)
    // 스크립트 훅 실행 (30줄)
    // 로깅 (20줄)
}

// After
async fn handle_message(&mut self, ctx, msg) -> Option<Message> {
    let processed = self.process_ws_payload(&msg)?;
    let processed = self.apply_ws_intercept_rules(processed)?;
    let processed = self.run_ws_script_hook(processed).await?;
    self.log_ws_message(&processed);
    Some(processed)
}
```

### 4.4 `handle_request()` / `handle_response()` — 각 ~100줄

동일 패턴 적용: 전처리 → 규칙 적용 → 스크립트 → 로깅 단계 분리

---

## 5. Phase 4: 구조체 책임 분리

**목표:** 단일 책임 원칙 적용
**난이도:** 높음 | **위험도:** 높음 (Phase 2, 3 완료 후) | **효과:** 유지보수성 대폭 개선

### 5.1 `LoggingHandler` 분리 (11개 필드 → 3개 구조체)

**파일:** `proxy_daemon/src/handler.rs`

```rust
// Before: God Object
pub struct LoggingHandler {
    sender: Sender<RequestInfo>,
    ws_sender: Option<Sender<WsEvent>>,
    ws_sequence: Arc<AtomicU64>,
    mqtt_versions: Arc<Mutex<HashMap<String, u8>>>,
    req: Option<ProxiedRequest>,
    res: Option<ProxiedResponse>,
    intercept_rules: Arc<Mutex<Vec<InterceptRule>>>,
    server_replay_entries: Arc<Mutex<Vec<ServerReplayEntry>>>,
    cache_dir: Option<PathBuf>,
    script_handle: ScriptHandle,
    ca_cert_der: Option<Bytes>,
}

// After: 책임별 분리
pub struct HttpContext {
    req: Option<ProxiedRequest>,
    res: Option<ProxiedResponse>,
    cache_dir: Option<PathBuf>,
    ca_cert_der: Option<Bytes>,
}

pub struct InterceptEngine {
    intercept_rules: Arc<Mutex<Vec<InterceptRule>>>,
    server_replay_entries: Arc<Mutex<Vec<ServerReplayEntry>>>,
    script_handle: ScriptHandle,
}

pub struct LoggingHandler {
    sender: Sender<RequestInfo>,
    ws_handler: WebSocketHandler,
    http_ctx: HttpContext,
    intercept: InterceptEngine,
}

pub struct WebSocketHandler {
    ws_sender: Option<Sender<WsEvent>>,
    ws_sequence: Arc<AtomicU64>,
    mqtt_versions: Arc<Mutex<HashMap<String, u8>>>,
}
```

### 5.2 `mcp_server/main.rs` 모듈 분리 (1,251줄 → 4개 파일)

```
crates/mcp_server/src/
├── main.rs              # 진입점, 서버 설정 (~50줄)
├── store.rs             # Store 구조체 및 데이터 관리 (~100줄)
├── tools/
│   ├── mod.rs           # 도구 등록
│   ├── traffic.rs       # 트래픽 관련 도구 (list, get, search, clear, export)
│   ├── websocket.rs     # WebSocket 관련 도구
│   ├── intercept.rs     # 인터셉트 규칙 도구
│   └── scripting.rs     # 스크립팅 도구
└── connection.rs        # 데몬 연결 관리
```

### 5.3 `data_type.rs` trait 분리 (897줄 → 3개 모듈)

```rust
// data_type/mod.rs - DataType enum 정의 + 기본 메서드
// data_type/detection.rs - detect_data_type(), 콘텐츠 분석 로직
// data_type/conversion.rs - to_mime_type(), to_monaco_language() 등 변환
```

---

## 6. Phase 5: 동시성 개선

**목표:** Lock 경합 최소화, 성능 향상
**난이도:** 중간 | **위험도:** 중간 | **효과:** 고부하 시 성능 개선

### 6.1 `std::sync::Mutex` → `parking_lot::Mutex`

**대상:** mcp_server의 Store, proxy_daemon의 공유 상태

```toml
# Cargo.toml
[dependencies]
parking_lot = "0.12"
```

**장점:**
- Lock poisoning 없음 (unwrap 불필요)
- std::sync::Mutex 대비 2~3배 빠름
- API 거의 동일 (drop-in replacement)

### 6.2 읽기 빈도 높은 데이터: `RwLock` 전환

**대상:** `intercept_rules`, `server_replay_entries`

```rust
// Before
intercept_rules: Arc<Mutex<Vec<InterceptRule>>>

// After
intercept_rules: Arc<parking_lot::RwLock<Vec<InterceptRule>>>
```

규칙은 읽기가 압도적으로 많고 쓰기는 드묾 → RwLock이 적합

### 6.3 불필요한 clone() 감소

**대상:** mcp_server에서 전체 VecDeque clone 패턴

```rust
// Before
let rules = self.store.rules.lock().unwrap().clone();

// After (필요한 데이터만 복사)
let rules: Vec<_> = self.store.rules.read()
    .iter()
    .filter(|r| r.enabled)
    .cloned()
    .collect();
```

---

## 7. 작업 순서 및 의존 관계

```
Phase 1: unwrap/expect 정리
  │  (독립 작업, 즉시 시작 가능)
  │
  ├── 1-1. proxy_daemon (daemon.rs, handler.rs) ← 최우선
  ├── 1-2. mcp_server (parking_lot 전환 포함)
  ├── 1-3. proxyapi_v2 (tunnel_event.rs, middleware.rs)
  └── 1-4. 나머지 (scripting, tui, proxy_v2_models)
  │
  ▼
Phase 2: 테스트 추가
  │  (Phase 1과 병행 가능)
  │
  ├── 2-1. proxy_daemon 통합 테스트 ← 최우선
  ├── 2-2. scripting 통합 테스트
  └── 2-3. mcp_server 통합 테스트
  │
  ▼
Phase 3: 거대 함수 분해 (Phase 2 완료 후)
  │
  ├── 3-1. analyze_handshake_failure() 분해
  ├── 3-2. daemon_main() 분해
  └── 3-3. handle_message/request/response 분해
  │
  ▼
Phase 4: 구조체 분리 (Phase 3 완료 후)
  │
  ├── 4-1. LoggingHandler 책임 분리
  ├── 4-2. mcp_server 모듈 분리
  └── 4-3. data_type.rs 모듈 분리
  │
  ▼
Phase 5: 동시성 개선 (Phase 4와 병행 가능)
  │
  ├── 5-1. parking_lot::Mutex 전환
  ├── 5-2. RwLock 전환
  └── 5-3. clone() 최적화
```

### 예상 변경 규모

| Phase | 변경 파일 수 | 신규 파일 수 | 삭제 파일 수 |
|-------|------------|------------|------------|
| 1 | ~15 | 0 | 0 |
| 2 | ~3 | ~8 | 0 |
| 3 | ~5 | 0 | 0 |
| 4 | ~6 | ~8 | 0 |
| 5 | ~5 | 0 | 0 |

### 브랜치 전략

```
main
 └── refactor/phase-1-error-handling
 └── refactor/phase-2-test-coverage
 └── refactor/phase-3-function-decomposition
 └── refactor/phase-4-struct-separation
 └── refactor/phase-5-concurrency
```

Phase별 독립 PR로 리뷰 부담 분산. Phase 1, 2는 병행 가능.
