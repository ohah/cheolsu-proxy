# Claude Code Rules

## PR 규칙
- PR 생성 시 항상 assignee를 `ohah`로 지정
- PR 생성 시 적절한 라벨 추가 (enhancement, bug, refactor, documentation 등)
- PR 올리기 전 반드시 린트 검사 (cargo fmt --check, cargo check, cargo test) 수행
- UI 관련 변경이 포함된 PR은 필요 시 `run-e2e` 라벨을 추가하여 e2e 테스트 실행

## 코드 규칙
- 커밋 메시지는 한국어로 작성
- 응답은 항상 한국어로
