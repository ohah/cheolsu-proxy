# PR 생성/업데이트 커맨드

변경 사항을 커밋하고, 린트 검사를 수행한 뒤, PR을 생성하거나 기존 PR에 추가한다.

## 1. 변경 사항 분석 및 커밋 분리

### 커밋 분리 원칙

- **하나의 커밋 = 하나의 목적** (기능 추가, 버그 수정, 리팩토링, 스타일 등을 절대 섞지 않는다)
- 관련 없는 변경은 반드시 별도 커밋으로 분리한다
- 각 커밋은 독립적으로 의미가 있어야 한다
- 가능한 작은 논리적 단위로 나눈다

### 커밋 메시지 형식

```
<type>(<scope>): <한국어 설명>

<상세 내용 (선택)>
```

- **type**: `feat` | `fix` | `refactor` | `test` | `docs` | `chore` | `style`
- **scope** (선택): `proxyapi` | `proxyapi_v2` | `proxy_v2_models` | `proxyapi_models` | `desktop` | `document` | `config`
- **설명**: 한국어로 작성, 무엇을 왜 변경했는지 명확히

### 커밋 순서

1. `git status`와 `git diff`로 모든 변경 사항을 파악한다
2. 변경 사항을 논리적 단위로 그룹핑한다
3. 각 그룹별로 관련 파일만 `git add`하여 개별 커밋한다
4. 커밋 간 의존성이 있다면 의존성 순서대로 커밋한다

## 2. 린트 및 포맷 자동 수정 (필수 — 커밋 전에 반드시 실행)

커밋하기 전에 아래 명령어를 실행하여 자동 수정한다. 체크만 하지 말고 **항상 자동 수정**을 실행한다.

### Rust

```bash
cargo fmt --all            # 포맷 자동 수정
cargo check                # 컴파일 검사
cargo test                 # 테스트 (기존 실패만 있으면 통과로 간주, 새 실패는 수정)
```

### TypeScript/Frontend (루트 디렉토리에서 실행)

```bash
bun run format             # oxfmt 포맷 자동 수정
bun run lint:fix           # oxlint 린트 자동 수정
npx tsc --noEmit           # 타입 체크 (desktop 디렉토리에서 실행)
```

### 자동 수정으로 변경된 파일이 있으면

- 해당 변경을 관련 커밋에 포함하거나 별도 `style:` 커밋으로 추가한다

## 3. PR 생성 또는 업데이트

### 기존 PR 확인

```bash
gh pr list --head $(git branch --show-current) --state open
```

- **기존 PR이 있으면**: 해당 PR에 커밋을 push하고, PR 설명을 업데이트한다
- **기존 PR이 없으면**: 새 PR을 생성한다

### 브랜치 관리

- 현재 브랜치가 main이면 새 브랜치를 생성한다
- 브랜치명은 `feat/`, `fix/`, `refactor/` 등 type에 맞는 prefix를 사용한다

### PR 생성

```bash
gh pr create \
  --title "<type>: 한국어 제목" \
  --assignee ohah \
  --label <적절한 라벨> \
  --body "$(cat <<'EOF'
<PR 본문>
EOF
)"
```

### PR 라벨 규칙

- `gh label list`로 사용 가능한 라벨을 확인한다
- 변경 내용에 맞는 라벨을 추가한다 (enhancement, bug, refactor, documentation 등)
- UI 관련 변경이 포함되면 필요 시 `run-e2e` 라벨도 추가한다

### PR Assignee

- 항상 `ohah`를 assignee로 지정한다

## 4. PR 본문 작성 규칙

PR 본문은 **한국어**로 최대한 자세하게 작성한다.

### 필수 섹션

```markdown
## 개요

이 PR의 목적과 배경을 설명한다. 왜 이 변경이 필요한지.

## 변경 내용

### 새로 추가된 파일

- `파일경로` — 설명

### 수정된 파일

- `파일경로` — 무엇을 왜 변경했는지

### 삭제된 파일

- `파일경로` — 왜 삭제했는지

## 구현 상세

기술적인 구현 방식, 아키텍처 결정, 주요 로직 등을 상세히 설명한다.

## 테스트

- [ ] 테스트 항목 1
- [ ] 테스트 항목 2

## 스크린샷 (UI 변경 시)

해당하는 경우 스크린샷을 첨부한다.
```

## 5. 전체 실행 순서

1. `git status`, `git diff`로 변경 사항 파악
2. 린트/포맷 자동 수정 실행 (`cargo fmt --all`, `bun run format`, `bun run lint:fix`)
3. 컴파일/타입 검사 (`cargo check`, `cargo test`, `npx tsc --noEmit`)
4. 변경 사항을 논리 단위로 분리하여 개별 커밋 (포맷 수정 포함)
5. `gh pr list`로 현재 브랜치의 기존 PR 확인
6. 기존 PR이 있으면 push + 설명 업데이트, 없으면 새 PR 생성
7. PR URL을 사용자에게 반환
