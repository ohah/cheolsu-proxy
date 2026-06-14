---
title: 테스트
description: Cheolsu Proxy 테스트 작성 및 실행 가이드
---

# 테스트

Cheolsu Proxy의 테스트 전략과 실행 방법을 설명합니다.

## 테스트 전략

### 테스트 피라미드

```
        E2E Tests (소수)
       /              \
   Integration Tests (중간)
  /                      \
Unit Tests (다수)
```

- **Unit Tests**: 개별 함수/메서드 테스트
- **Integration Tests**: 모듈 간 상호작용 테스트
- **E2E Tests**: 전체 애플리케이션 테스트

## Rust 테스트

OpenSSL dylib 경로를 자동으로 설정해 주는 루트 스크립트를 사용하는 것이 가장 간단합니다.

```bash
# 전체 워크스페이스 테스트 (DYLD_LIBRARY_PATH 자동 설정)
bun run cargo:test

# 특정 크레이트만
bun run cargo:test -- -p proxy_daemon
```

CI는 두 단계로 나누어 실행합니다(`proxyapi_v2`는 `full` 기능 필요).

```bash
cargo test --workspace --exclude proxyapi_v2
cargo test -p proxyapi_v2 --features full
```

> macOS에서 직접 실행할 때 `libssl.3.dylib`를 찾지 못하면 `DYLD_LIBRARY_PATH`에 번들 경로(`desktop/src-tauri/openssl-bundle/lib`)를 추가하거나 `OPENSSL_DIR=/opt/homebrew/opt/openssl@3`를 지정하세요.

### Rust 포맷

```bash
# 포맷 적용
bun run cargo:fmt
# 포맷 검사 (CI와 동일)
cargo fmt --all -- --check
```

## 프론트엔드 테스트

테스트는 `bun test`로 실행합니다(`desktop/` 디렉터리 기준).

```bash
cd desktop

bun run test            # 스토어/엔티티/라이브러리 단위 테스트
bun run test:components # features/widgets/pages 컴포넌트 테스트
bun run test:e2e        # E2E 테스트
bun run test:all        # 위 전체 실행
```

### JavaScript/TypeScript 린트 & 포맷

저장소 루트에서 실행합니다(oxlint / oxfmt 기반).

```bash
bun run lint          # oxlint
bun run format:check  # oxfmt --check
bun run format        # oxfmt (적용)
```

## i18n 카탈로그 동기화

GUI 코드(문자열)를 변경했다면 카탈로그를 갱신하고 함께 커밋해야 합니다. CI가 카탈로그가 최신인지 검사합니다.

```bash
cd desktop && bun run extract
```

## CI 체크 항목

PR은 다음 항목을 통과해야 합니다.

- **Rust Format Check** — `cargo fmt --all -- --check`
- **Rust Unit Test** — 워크스페이스 + `proxyapi_v2 --features full`
- **JavaScript/TypeScript Lint Check** — `format:check`, `lint`, i18n 카탈로그 동기화
- **Frontend Unit Test** — `bun test`
