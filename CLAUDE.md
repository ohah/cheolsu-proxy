# Claude Code Rules

## 코드 규칙

- 커밋 메시지는 한국어로 작성
- 응답은 항상 한국어로

## PR 규칙

- PR 생성 시 라벨(`enhancement`, `bug` 등 적절한 라벨) 추가
- PR 생성 시 assignee를 `ohah`로 설정
- PR 제목은 한국어로 작성
- GUI 코드 변경 시 `bun run extract`로 i18n 카탈로그 업데이트 필수
- Rust 코드 변경 시 `cargo fmt --all`로 포맷 확인 필수
- CI 체크 항목: Rust Format Check, JavaScript/TypeScript Lint Check, Frontend Unit Tests

## CLI 서브커맨드 가이드라인

MCP 도구와 동일한 기능을 CLI 서브커맨드로 제공한다. 새 CLI 커맨드를 추가할 때 아래 규칙을 따른다.

### 아키텍처 원칙

- **핵심 로직 재사용**: MCP 도구와 CLI 커맨드는 `cheolsu_ops` 크레이트의 동일한 비즈니스 로직을 공유한다.
- **로직 중복 금지**: MCP `tools/*.rs`와 CLI `main.rs` 양쪽에 로직을 복사하지 않는다. 비즈니스 로직은 반드시 `cheolsu_ops`에 위치한다.
- **daemon 통신**: CLI도 MCP와 동일하게 UDS를 통해 `proxy_daemon`과 통신한다. `OpsContext`를 그대로 사용한다.

### 커맨드 구조

- clap derive 모드를 사용한다.
- MCP 도구 카테고리에 맞춰 서브커맨드를 그룹화한다:
  ```
  cheolsu-cli traffic search --host example.com --method GET
  cheolsu-cli rule list
  cheolsu-cli analyze performance
  ```
- 서브커맨드 이름은 MCP 도구 이름과 일관성을 유지한다.

### 파라미터 매핑

- `cheolsu_ops/src/params.rs`의 파라미터 struct를 CLI에서 직접 생성하여 ops 함수에 전달한다.
- Optional 필드는 CLI에서도 optional flag로 유지한다.

### 출력 규칙

- 에러는 stderr로, 결과는 stdout으로 출력한다.
- exit code: 성공=0, 에러=1.
