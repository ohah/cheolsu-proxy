# Rust 코드베이스 리팩토링 계획

> 작성일: 2026-03-09
> 최종 업데이트: 2026-03-10
> 대상: `crates/` 디렉토리 (총 ~27,000줄, 7개 크레이트)

---

## 목차

1. [현황 요약](#1-현황-요약)
2. [Phase 1: unwrap/expect 정리](#2-phase-1-unwrapexpect-정리) — 거의 완료
3. [Phase 2: 테스트 커버리지 확대](#3-phase-2-테스트-커버리지-확대) — 거의 완료
4. [Phase 3: 거대 함수 분해](#4-phase-3-거대-함수-분해) — ✅ 완료
5. [Phase 4: 구조체 책임 분리](#5-phase-4-구조체-책임-분리) — ✅ 완료
6. [Phase 5: 동시성 개선](#6-phase-5-동시성-개선) — 부분 완료

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

### 테스트 현황 (2026-03-10 기준)

| 크레이트        | 단위 테스트 | 통합 테스트 | 합계    |
| --------------- | ----------- | ----------- | ------- |
| proxy_v2_models | 114+        | 0           | 114+    |
| proxyapi_v2     | 76          | 98          | 174     |
| proxy_daemon    | 284         | 44          | **328** |
| mcp_server      | 31          | 9           | 40      |
| tui             | 29+         | 0           | 29+     |
| scripting       | 19          | 18          | 37      |
| **합계**        |             |             | **579** |

---

## 2. Phase 1: unwrap/expect 정리 — 거의 완료

**목표:** 프로덕션 코드에서 panic 가능성 제거
**난이도:** 낮음 | **위험도:** 낮음 | **효과:** 런타임 안정성 즉시 개선

### 완료된 항목

- ~~proxy_daemon/src/daemon.rs~~ — ✅ `unwrap_or_else`로 개선
- ~~proxy_daemon/src/handler.rs~~ — ✅ 정적 문자열 2건만 남음 (위험 낮음)
- ~~scripting/src/handle.rs~~ — ✅ expect() 제거
- ~~mcp_server/src/main.rs~~ — ✅ parking_lot 전환으로 lock().unwrap() 해결
- ~~proxyapi_v2/src/proxy/middleware.rs~~ — ✅ 제거
- ~~proxy_v2_models/src/file_storage.rs~~ — ✅ ok_or_else로 개선
- ~~proxy_daemon/src/intercept.rs~~ — ✅ 제거
- ~~tui/src/app/key_handlers.rs~~ — ✅ 제거

### 남은 항목

#### proxyapi_v2/src/tunnel_event.rs (15건) 🔴

```rust
// Line 104-162
header_value.to_str().unwrap()
value.parse().unwrap()
// 외부 입력값(target_addr, client_addr, error_msg) 파싱에 사용
```

**조치:** `Option`/`Result` 체이닝으로 변경, 파싱 실패 시 기본값 사용

---

## 3. Phase 2: 테스트 커버리지 확대 — 거의 완료

**목표:** 리팩토링 안전망 구축
**난이도:** 중간 | **위험도:** 없음 | **효과:** 리팩토링 신뢰도 확보

### 완료된 항목

- ~~proxy_daemon 통합 테스트~~ — ✅ 2개 파일 (intercept_integration_test.rs, protocol_test.rs)
- ~~scripting 통합 테스트~~ — ✅ 1개 파일 (script_lifecycle_test.rs)

### 남은 항목

#### mcp_server 통합 테스트 ⚠️

통합 테스트 0건. MCP 도구 호출 및 응답 검증이 필요.

```
crates/mcp_server/tests/
└── mcp_protocol_test.rs        # MCP 도구 호출 및 응답 검증
```

---

## 4. Phase 3: 거대 함수 분해 — ✅ 완료

모든 대상 함수가 적절한 크기로 리팩토링됨.

| 함수                         | 계획서 줄 수 | 현재 줄 수 |
| ---------------------------- | ------------ | ---------- |
| `analyze_handshake_failure()` | 370줄        | 44줄       |
| `daemon_main()`              | 160줄        | 136줄      |
| `handle_message()`           | 141줄        | 31줄       |
| `handle_response()`          | ~100줄       | 35줄       |
| `handle_request()`           | ~100줄       | 85줄       |

---

## 5. Phase 4: 구조체 책임 분리 — ✅ 완료

- ✅ 4-1. LoggingHandler 책임 분리 (HttpState, InterceptEngine, WebSocketState)
- ✅ 4-2. mcp_server 모듈 분리 (store.rs, params.rs, helpers.rs, connection.rs)
- ✅ 4-3. data_type.rs 모듈 분리 (data_type/mod.rs, detection.rs, decompression.rs)

---

## 6. Phase 5: 동시성 개선 — 부분 완료

**목표:** Lock 경합 최소화, 성능 향상
**난이도:** 중간 | **위험도:** 중간 | **효과:** 고부하 시 성능 개선

### 완료된 항목

- ~~mcp_server: parking_lot::Mutex 전환~~ — ✅
- ~~proxy_daemon: tokio::sync::Mutex 사용~~ — ✅ (비동기 컨텍스트에 적합)

### 남은 항목

#### 6.1 읽기 빈도 높은 데이터: `RwLock` 전환 ⚠️

**대상:** `intercept_rules`, `server_replay_entries` (proxy_daemon의 InterceptEngine)

```rust
// Before
intercept_rules: Arc<tokio::sync::Mutex<Vec<InterceptRule>>>

// After
intercept_rules: Arc<tokio::sync::RwLock<Vec<InterceptRule>>>
```

규칙은 읽기가 압도적으로 많고 쓰기는 드묾 → RwLock이 적합

#### 6.2 불필요한 clone() 감소 🔴

**대상:** mcp_server에서 전체 VecDeque/Vec `.lock().clone()` 패턴 (8곳)

```rust
// Before
let rules = self.store.rules.lock().clone();

// After (필요한 데이터만 복사)
let rules: Vec<_> = self.store.rules.lock()
    .iter()
    .filter(|r| r.enabled)
    .cloned()
    .collect();
```

---

## 7. 남은 작업 요약

```
남은 작업:
  │
  ├── 1. tunnel_event.rs unwrap 15건 제거 (Phase 1 잔여)
  ├── 2. mcp_server 통합 테스트 작성 (Phase 2 잔여)
  ├── 3. InterceptEngine RwLock 전환 (Phase 5)
  └── 4. mcp_server .lock().clone() 8곳 최적화 (Phase 5)
```
