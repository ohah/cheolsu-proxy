# 안정화 감사 보고서

> 작성일: 2026-03-10
> 대상: Tokio 런타임, Tauri-Rust IPC, 데몬 통신

---

## 🔴 CRITICAL — 즉시 수정 필요

### 1. UDS 연결 끊김 시 재연결 없음

- **파일:** `crates/proxy_daemon/src/client.rs:220-246`
- **문제:** event_task에서 EOF/에러 시 `break`로 종료. 재연결 시도 없음, 호출자에게 에러 전파 없음
- **영향:** 클라이언트가 연결 끊긴 줄 모르고 명령을 계속 보냄
- **해결:** 재연결 로직 + exponential backoff, 연결 상태 콜백 전파

### 2. 프록시 태스크 abort() 강제 종료

- **파일:** `crates/proxy_daemon/src/daemon.rs:410`
- **문제:** graceful shutdown 없이 `proxy_handle.abort()` 호출
- **영향:** 진행 중인 HTTP 요청, WebSocket, TLS 세션 불완전 종료
- **해결:** shutdown 시그널 → 타임아웃 대기 → abort 패턴

### 3. 이벤트 채널 unbounded로 OOM 위험

- **파일:** `desktop/src-tauri/src/proxy_v2.rs:65`
- **문제:** `tokio::sync::mpsc::unbounded_channel` 사용
- **영향:** 고트래픽 시 메모리 무한 증가
- **해결:** bounded channel(1024~4096) + backpressure

### 4. client_count 레이스 컨디션

- **파일:** `crates/proxy_daemon/src/daemon.rs:257, 282`
- **문제:** 클라이언트 태스크 abort 시 `fetch_sub` 미실행
- **영향:** 데몬 셧다운 로직 트리거 안 됨
- **해결:** Drop guard 패턴으로 카운트 감소 보장

---

## 🟠 HIGH — 빠른 시일 내 수정

### 5. Broadcast 채널 메시지 유실

- **파일:** `crates/proxy_daemon/src/daemon.rs:310` (버퍼 256)
- **파일:** `crates/proxy_daemon/src/client_handler.rs:133-135`
- **문제:** Lagged 에러 시 경고만 로깅, 이벤트 조용히 유실
- **해결:** 버퍼 증가(1024+) + Lagged 누적 시 클라이언트 강제 재연결

### 6. 12곳 이상 send() 에러 무시

- **파일:** `crates/proxy_daemon/src/client_handler.rs` — 162, 164, 241, 243, 254, 256, 267, 269, 342, 377, 398, 400, 546번 줄
- **문제:** broadcast/watch 전송 실패를 `let _ =`로 전부 무시
- **해결:** 최소한 `warn!` 로깅 추가

### 7. 비원자적 상태 업데이트

- **파일:** `crates/proxy_daemon/src/client_handler.rs:156-165`
- **문제:** event_tx.send()와 intercept_tx.send()가 원자적이지 않음
- **영향:** 클라이언트는 업데이트 수신했으나 프록시는 미적용 상태 가능
- **해결:** 트랜잭션 패턴 또는 순서 보장 (watch 먼저 → broadcast)

### 8. signal handler JoinHandle 미보관

- **파일:** `crates/proxy_daemon/src/daemon.rs:224-242`
- **문제:** Ctrl+C, SIGTERM 핸들러 spawn 후 JoinHandle 버림
- **해결:** JoinHandle 보관 후 shutdown 시 await

### 9. Tauri 동기 함수에서 blocking I/O

- **파일:** `desktop/src-tauri/src/proxy_v2.rs:965-1030`
- **문제:** `install_cli()`, `uninstall_cli()`가 sync 함수에서 `std::process::Command` 실행
- **영향:** Tauri 메인 스레드 블로킹 → UI 프리즈
- **해결:** async 변환 또는 spawn_blocking 래핑

### 10. 자동저장 3초 하드코딩 타임아웃

- **파일:** `desktop/src-tauri/src/tray.rs:35-41`
- **문제:** 큰 세션에서 3초 안에 저장 완료 못 하면 데이터 유실
- **해결:** 세션 크기 기반 동적 타임아웃 또는 저장 완료 콜백 대기

---

## 🟡 MEDIUM — 안정화 단계에서 개선

### 11. tunnel 이벤트 버퍼 부족

- **파일:** `crates/proxy_daemon/src/proxy_runner.rs:50`
- **문제:** tunnel 이벤트 버퍼 100개로 작음 (다른 채널은 256)
- **해결:** 256으로 통일

### 12. parking_lot::Mutex가 async 컨텍스트에 존재

- **파일:** `crates/proxy_daemon/src/daemon.rs:175`, `handler.rs:82`
- **문제:** await 사이에 잡히면 패닉 위험 (현재 코드는 안전하나 리팩토링 시 위험)
- **해결:** tokio::sync::Mutex로 교체 또는 주석으로 경고 명시

### 13. event_task/log_task abort() 정리 없음

- **파일:** `crates/proxy_daemon/src/client_handler.rs:454-455`
- **문제:** 클라이언트 핸들러 종료 시 하위 태스크 강제 abort
- **해결:** cancellation token 사용

### 14. 스크립트 엔진 스레드 JoinHandle 미보관

- **파일:** `crates/scripting/src/handle.rs:65-77`
- **문제:** 전용 OS 스레드 생성 후 JoinHandle 버림 → 셧다운 시 leak
- **해결:** JoinHandle 보관 후 shutdown 시 join

### 15. 데몬 시작 대기 시간 고정

- **파일:** `crates/proxy_daemon/src/client.rs:142-176`
- **문제:** 5초 고정 대기 — 느린 시스템에서 부족할 수 있음
- **해결:** exponential backoff + 최대 대기 시간 증가

### 16. 파일 와처 레이스 컨디션

- **파일:** `crates/proxy_daemon/src/client_handler.rs:496-500`
- **문제:** watched_path 체크와 reload 사이 경로 변경 가능
- **해결:** CancellationToken으로 와처 중단 관리

### 17. app.emit() 실패 무시

- **파일:** `desktop/src-tauri/src/proxy_v2.rs:74-127`
- **문제:** 이벤트 발행 실패 시 로깅 없음
- **해결:** warn! 로깅 추가

### 18. 전체 트랜잭션 JSON 직렬화

- **파일:** `desktop/src/app/App.tsx:184-185`
- **문제:** 대량 캡처 시 UI 스레드 블로킹
- **해결:** Web Worker에서 직렬화 또는 청크 단위 처리

### 19. 클라이언트 인증서 변경 미적용

- **파일:** `crates/proxy_daemon/src/proxy_runner.rs:149-158`
- **문제:** 경고만 출력, 프록시 자동 재시작 미지원
- **해결:** 프록시 자동 재시작 또는 사용자 알림 UI

### 20. advanced_repeat 이벤트 리스너 타이밍

- **파일:** `desktop/src/features/advanced-repeat-dialog.tsx:85-95`
- **문제:** 리스너 설정이 명령 실행 후라 초기 progress 누락 가능
- **해결:** 리스너 설정 후 명령 실행으로 순서 변경

---

## 수정 로드맵

| Phase   | 대상     | 이슈 번호               |
| ------- | -------- | ----------------------- |
| Phase 1 | 크리티컬 | #1, #2, #3, #4          |
| Phase 2 | 안정성   | #5, #6, #7, #8, #9, #10 |
| Phase 3 | 품질     | #11 ~ #20               |
