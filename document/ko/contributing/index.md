---
title: 기여하기
description: Cheolsu Proxy 프로젝트에 기여하는 방법
---

# 기여하기

Cheolsu Proxy 프로젝트에 기여해주셔서 감사합니다! 이 프로젝트를 개선하는 데 도움을 주시는 커뮤니티의 기여를 환영합니다. 버그 수정, 새로운 기능 추가, 문서 개선 등에 관심이 있으시다면 참여할 수 있는 다양한 방법이 있습니다.

## 기여 방법

이 프로젝트에 기여하기 시작하는 몇 가지 단계입니다:

1. **저장소 포크**: GitHub에서 저장소를 포크하고 로컬 머신에 클론합니다
2. **브랜치 생성**: 변경사항을 위한 새 브랜치를 생성합니다
3. **변경사항 구현**: 변경사항을 만들고 철저히 테스트합니다
4. **커밋**: 설명적인 커밋 메시지와 함께 변경사항을 커밋합니다
5. **풀 리퀘스트**: 변경사항을 포크에 푸시하고 풀 리퀘스트를 제출합니다

작은 버그 수정부터 주요한 새로운 기능까지, 어떤 규모의 기여든 감사합니다. 하고 싶은 변경사항에 대해 확신이 서지 않는다면, 먼저 이슈를 열어서 유지보수자들과 논의해보세요.

## 기여 유형

### 🐛 버그 수정

- 버그 리포트 확인 및 재현
- 근본 원인 분석
- 테스트 케이스 작성
- 수정 사항 구현

### ✨ 새로운 기능

- 기능 요청 검토
- 설계 문서 작성
- 구현 및 테스트
- 문서 업데이트

### 📚 문서 개선

- 사용자 가이드 작성
- API 문서 업데이트
- 코드 주석 개선
- 번역 작업

### 🧪 테스트

- 단위 테스트 작성
- 통합 테스트 추가
- 성능 테스트
- 보안 테스트

## 개발 환경 설정

### 필수 요구사항

- **Rust**: 1.70.0 이상
- **Node.js**: 22.0.0 이상
- **Tauri CLI**: 최신 버전
- **Git**: 2.0.0 이상

### Rust 설치

```bash
# macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows
# https://rustup.rs/ 에서 rustup-init.exe 다운로드 후 실행
```

### 개발 도구 설치

```bash
# 코드 포맷팅
rustup component add rustfmt

# 린터
rustup component add clippy

# Tauri CLI
cargo install tauri-cli
```

### 프로젝트 클론 및 빌드

```bash
# 저장소 클론
git clone https://github.com/ohah/cheolsu-proxy.git
cd cheolsu-proxy

# 의존성 설치
cargo build
cd tauri-ui && bun install
```

## 개발 워크플로우

### 1. 이슈 확인

- [GitHub Issues](https://github.com/ohah/cheolsu-proxy/issues)에서 작업할 이슈 선택
- 이슈에 "assign"하여 작업 중임을 표시
- 이슈에 대한 질문이나 제안사항이 있으면 댓글 작성

### 2. 브랜치 생성

```bash
# 최신 코드로 업데이트
git checkout main
git pull origin main

# 새 브랜치 생성
git checkout -b feature/your-feature-name
# 또는
git checkout -b fix/your-bug-fix
```

### 3. 개발 및 테스트

```bash
# 개발 서버 실행
cd tauri-ui
bun run tauri dev

# 테스트 실행
cargo test
cargo clippy
cargo fmt
```

### 4. 커밋

```bash
# 변경사항 스테이징
git add .

# 커밋 (커밋 메시지 규칙 준수)
git commit -m "feat: 새로운 기능 추가"
git commit -m "fix: 버그 수정"
git commit -m "docs: 문서 업데이트"
```

### 5. 풀 리퀘스트

```bash
# 브랜치 푸시
git push origin feature/your-feature-name

# GitHub에서 풀 리퀘스트 생성
```

## 코딩 스타일

### Rust 코딩 규칙

- **Rust 표준 코딩 컨벤션** 따르기 (rustfmt 사용)
- **에러 처리**: Result 타입을 사용하여 명시적으로 처리
- **문서화 주석**: `///` 를 사용
- **모듈 구조**: 명확하게 분리
- **테스트**: 새로운 기능에는 적절한 테스트 작성

### TypeScript/React 코딩 규칙

- **TypeScript strict 모드** 사용
- **함수형 컴포넌트**와 hooks 사용
- **타입 정의**: 명시적으로 작성
- **컴포넌트**: 단일 책임 원칙 따르기
- **인터페이스**: 가능하면 Interface 사용
- **Props**: 인라인 형식 사용하지 않기

### 파일 구조

- **FSD(Feature-Sliced Design)** 아키텍처 따르기
- **네이밍 컨벤션**:
  - 변수와 함수: camelCase
  - 상수: UPPER_SNAKE_CASE
  - 컴포넌트: PascalCase
  - 파일명: kebab-case

## 커밋 메시지 규칙

### 형식

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### 타입

- `feat`: 새로운 기능
- `fix`: 버그 수정
- `docs`: 문서 변경
- `style`: 코드 포맷팅, 세미콜론 누락 등
- `refactor`: 코드 리팩토링
- `test`: 테스트 추가 또는 수정
- `chore`: 빌드 프로세스, 보조 도구 변경

### 예시

```bash
feat(proxy): TLS 1.3 지원 추가
fix(ui): 다크 테마에서 텍스트 가독성 개선
docs(guide): 인증서 설치 가이드 업데이트
refactor(ca): 인증서 생성 로직 최적화
```

## 테스트

### 단위 테스트

```bash
# 모든 테스트 실행
cargo test

# 특정 테스트 실행
cargo test test_name

# 통합 테스트 실행
cargo test --test integration_test
```

### UI 테스트

```bash
# Tauri 앱 테스트
cd tauri-ui
bun run tauri test
```

### 수동 테스트

```bash
# HTTP 서버와 클라이언트 설치
cargo install echo-server xh

# HTTP 서버 실행
echo-server

# HTTP 클라이언트로 테스트
xh --proxy http:http://127.0.0.1:8100 GET http://127.0.0.1:8080
```

## 풀 리퀘스트 가이드라인

### PR 제목

- 명확하고 간결하게 작성
- 변경사항을 한 줄로 요약
- 이슈 번호 포함 (예: "Fix #123")

### PR 설명

```markdown
## 변경사항

- 변경된 내용을 명확히 설명

## 테스트

- [ ] 단위 테스트 통과
- [ ] 통합 테스트 통과
- [ ] 수동 테스트 완료

## 체크리스트

- [ ] 코드 스타일 가이드 준수
- [ ] 문서 업데이트
- [ ] 브레이킹 체인지 없음

## 관련 이슈

Closes #123
```

### 리뷰 프로세스

1. **자동 검사**: CI/CD 파이프라인 통과
2. **코드 리뷰**: 최소 1명의 관리자 승인
3. **테스트**: 모든 테스트 통과
4. **병합**: 관리자만 병합 가능 (Squash and merge 권장)

## 문제 해결

### 일반적인 문제

**빌드 실패**:

```bash
# 의존성 업데이트
cargo update
cd tauri-ui && bun update

# 캐시 정리
cargo clean
cd tauri-ui && rm -rf node_modules && bun install
```

**테스트 실패**:

```bash
# 테스트 환경 확인
cargo test --verbose

# 특정 테스트 디버깅
cargo test test_name -- --nocapture
```

**Tauri 빌드 오류**:

```bash
# Tauri CLI 업데이트
cargo install tauri-cli --force

# 의존성 재설치
cd tauri-ui && bun install
```

## 커뮤니티

### 소통 채널

- **GitHub Issues**: 버그 리포트, 기능 요청
- **GitHub Discussions**: 일반적인 질문, 아이디어 공유
- **Pull Requests**: 코드 리뷰, 토론

### 행동 강령

모든 기여자는 [행동 강령](https://github.com/ohah/cheolsu-proxy/blob/main/CODE_OF_CONDUCT.md)을 준수해야 합니다.

## 라이선스

기여하시는 모든 코드는 프로젝트와 동일한 라이선스(MIT/Apache-2.0)로 배포됩니다.

## 감사

모든 기여자분들께 감사드립니다! 여러분의 기여가 Cheolsu Proxy를 더 나은 도구로 만들어줍니다.

---

더 자세한 정보가 필요하시면 [개발 환경 설정](/ko/contributing/development-setup)을 참조하세요.
