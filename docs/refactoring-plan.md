# 리팩토링 계획서

> 작성일: 2026-03-08
> 대상: cheolsu-proxy 코드베이스 (Rust + TypeScript)
> 진행 상태: Phase 1 완료, Phase 2 완료, Phase 3 완료 (2026-03-08)

## 1. 목적

대형 파일(1,000줄 이상)을 관심사별로 분리하여 유지보수성과 가독성을 높인다.
기존 공개 API(import 경로)를 `pub use` re-export로 유지하여 외부 영향을 최소화한다.

## 2. 현황

| 파일                                 | 라인  | 문제                                                      |
| ------------------------------------ | ----- | --------------------------------------------------------- |
| `crates/tui/src/app.rs`              | 1,633 | 폼 타입, 키 핸들링, CA 인증서, 유틸이 한 파일에 혼재      |
| `crates/proxy_daemon/src/handler.rs` | 1,605 | TLS 클라이언트, 인터셉트, 스크립트 연동, curl 폴백이 혼재 |
| `crates/proxy_daemon/src/daemon.rs`  | 1,151 | 데몬 생명주기, 클라이언트 처리, 프록시 실행이 혼재        |

### 부수적 이슈

- `tui/src/app.rs`에서 `next()`/`prev()` 순환 로직이 3곳에서 중복
- 에러 타입이 `Box<dyn Error>`, `hyper::Error`, `String`, `let _ =` 등 4가지 패턴 혼재
- `proxy_v2_models/` 전체에 테스트 부재 (순수 데이터 모델)

## 3. 분리 원칙

1. **단계별 커밋** — 각 Step을 독립 커밋으로 만들어 실패 시 revert 가능
2. **re-export 유지** — `pub use` 로 기존 import 경로 보존
3. **단계별 검증** — 매 Step 후 `cargo build` + `cargo test` 통과 확인
4. **독립 코드부터** — 의존성 없는 타입/함수를 먼저 추출

## 4. Phase 1 — 위험도 낮음 (독립 타입/함수 추출)

> 각 Step이 독립적이므로 병렬 진행 가능

### Step 1-A: `app.rs` → 폼 타입 추출

- **대상**: `ScriptLogEntry`, `WsConnection`, `UpstreamProxyForm/Field`, `RuleForm/Field`, `ActionType` 및 impl 블록 (~300줄)
- **이동**: `crates/tui/src/app/forms.rs`
- **방법**:
  1. `app.rs` → `app/mod.rs`로 이동
  2. `app/forms.rs` 생성, 타입 이동
  3. `mod.rs`에서 `pub use forms::*;` re-export
- **검증**: `ui/settings.rs`의 `use crate::app::{App, UpstreamProxyField}` 경로 유지 확인
- **테스트**: `UpstreamProxyForm` 관련 기존 테스트 12개 통과 확인

### Step 1-B: `app.rs` → 유틸리티 함수 추출

- **대상**: `copy_to_clipboard()`, `format_ws_messages()`, `format_curl_command()` (~80줄)
- **이동**: `crates/tui/src/app/utils.rs`
- **방법**: 자유 함수이므로 그대로 이동, `mod.rs`에서 `use utils::*;`

### Step 1-C: `handler.rs` → TLS 클라이언트 추출

- **대상**: `DangerousCertificateVerifier` + `create_hybrid_client()` (~120줄)
- **이동**: `crates/proxy_daemon/src/tls_client.rs`
- **방법**: `lib.rs`에서 `pub mod tls_client;` 추가, re-export 유지
- **주의**: `daemon.rs`의 `use crate::handler::create_hybrid_client` 경로 변경 또는 `handler.rs`에서 re-export

### Step 1-D: `handler.rs` → curl 폴백 추출

- **대상**: `fallback_with_curl()` + `parse_curl_response()` (~110줄)
- **이동**: `crates/proxy_daemon/src/curl_fallback.rs`
- **방법**: 자유 함수이므로 그대로 이동, `handler.rs`에서 `use crate::curl_fallback::*;`

## 5. Phase 2 — 위험도 중간 (impl 블록 분산)

> Rust에서 같은 crate 내 여러 파일에 `impl Struct` 블록을 나눠 쓸 수 있음
> 분리된 파일에서 `self.필드` 접근 시 해당 필드를 `pub(crate)`로 변경 필요

### Step 2-A: `app.rs` → CA 인증서 메서드 추출

- **대상**: `get_ca_storage_dir()`, `check_ca_status()`, `install_ca_cert()`, `uninstall_ca_cert()` (~190줄)
- **이동**: `crates/tui/src/app/ca_cert.rs`
- **방법**: `impl App { ... }` 블록으로 배치
- **전제**: Step 1-A 완료 (app/ 디렉토리 존재)

### Step 2-B: `app.rs` → 키 핸들러 추출

- **대상**: `handle_key()`, `handle_network_key()`, `handle_ws_key()`, `handle_rules_key()`, `handle_rule_form_key()`, `handle_settings_key()`, `handle_script_key()` (~575줄)
- **이동**: `crates/tui/src/app/key_handlers.rs`
- **방법**: `impl App { ... }` 블록으로 배치
- **전제**: Step 2-A 완료 (호출하는 `set_status`, `send_*` 메서드가 `pub(crate)` 이상)

### Step 2-C: `handler.rs` → 인터셉트 엔진 추출

- **대상**: `wildcard_matches()`, `rule_matches()`, `guess_content_type()`, `find_matching_intercept_rules()`, `apply_request_intercept()`, `apply_response_intercept()`, `find_server_replay_match()` (~340줄)
- **이동**: `crates/proxy_daemon/src/intercept.rs`
- **방법**: `impl LoggingHandler { ... }` 블록으로 배치
- **테스트**: `wildcard_matches`, `rule_matches` 테스트 14개 이동

### Step 2-D: `handler.rs` → 스크립트 연동 추출

- **대상**: `to_script_request()`, `to_script_response_from_hyper()`, `apply_script_request_modify()`, `build_script_response()`, `apply_script_response_modify()` (~105줄)
- **이동**: `crates/proxy_daemon/src/script_bridge.rs`
- **방법**: `impl LoggingHandler { ... }` 블록으로 배치

## 6. Phase 3 — 위험도 높음 (핵심 로직 분할)

### Step 3-A: `daemon.rs` → 클라이언트 핸들링 추출

- **대상**: `handle_client()` (267줄, 매개변수 9개) + `start_file_watcher()` (~370줄)
- **이동**: `crates/proxy_daemon/src/client_handler.rs`
- **선행 리팩토링**: 매개변수 9개를 `ClientContext` 구조체로 묶기
- **위험**: async 시그니처와 채널 타입의 복잡한 의존성

### Step 3-B: `daemon.rs` → 프록시 실행 추출

- **대상**: `run_proxy()` (~140줄)
- **이동**: `crates/proxy_daemon/src/proxy_runner.rs`
- **위험**: 내부 의존성 가장 많음, 마지막 단계에서 진행

## 7. 목표 파일 구조

```
crates/tui/src/
  app/
    mod.rs            ~400줄  (App 구조체 + 핵심 메서드)
    forms.rs          ~300줄  (폼 타입, 열거형)
    ca_cert.rs        ~190줄  (CA 인증서 관리)
    key_handlers.rs   ~575줄  (탭별 키 핸들링)
    utils.rs          ~80줄   (유틸리티 함수)

crates/proxy_daemon/src/
    handler.rs        ~500줄  (LoggingHandler 핵심 + trait impl)
    tls_client.rs     ~120줄  (TLS 인증서 검증, 클라이언트 생성)
    intercept.rs      ~340줄  (인터셉트 규칙 매칭/적용)
    curl_fallback.rs  ~110줄  (curl 폴백 처리)
    script_bridge.rs  ~105줄  (스크립트 요청/응답 변환)
    daemon.rs         ~360줄  (데몬 생명주기, 경로/잠금 유틸)
    client_handler.rs ~370줄  (클라이언트 연결 처리)
    proxy_runner.rs   ~140줄  (프록시 서버 실행)
```

## 8. 각 단계 검증 체크리스트

매 Step 완료 후 아래를 순서대로 수행:

- [ ] `cargo build --workspace` 성공
- [ ] `cargo test --workspace` 전체 통과
- [ ] `cargo clippy --workspace` 경고 없음
- [ ] 기존 import 경로로 사용하는 외부 코드에 변경 없음
- [ ] 단계별 git commit 생성

## 9. 향후 과제 (본 리팩토링 범위 밖)

- `proxy_v2_models/` 유닛 테스트 추가 (순수 데이터 모델, 테스트 작성 용이)
- 데스크톱 TypeScript 테스트 커버리지 확대 (현재 195개 중 6개만 테스트)
- 에러 타입 통일 (`thiserror` 기반 커스텀 에러 정의)
- `next()`/`prev()` 순환 로직 중복 제거 (트레잇 또는 매크로)
- `data_type.rs`의 TODO 3건 해결 (이미지/비디오/오디오 감지 로직)
