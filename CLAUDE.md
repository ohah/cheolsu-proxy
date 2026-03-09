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
