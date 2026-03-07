---
title: 개발 환경 설정
description: Cheolsu Proxy 개발을 위한 환경 설정 가이드
---

# 개발 환경 설정

Cheolsu Proxy 개발을 위한 완전한 환경 설정 가이드입니다.

## 기술 스택

이 프로젝트는 다음 도구와 기술을 사용합니다:

- **Rust**: Cargo 패키지 매니저를 사용하는 핵심 백엔드 언어
- **Tauri**: 데스크톱 애플리케이션 프레임워크
- **bun**: Node.js 호환 런타임 및 패키지 매니저 (mise.toml로 버전 고정)
- **oxc**: 포맷(oxfmt) 및 린트(oxlint)용 JavaScript/TypeScript 도구체인
- **Rspress**: 문서 사이트 생성기

## 시스템 요구사항

### 운영체제

- **macOS**: 10.15 (Catalina) 이상
- **Windows**: Windows 10 이상

### 하드웨어

- **RAM**: 최소 8GB (16GB 권장)
- **저장공간**: 최소 10GB 여유 공간
- **CPU**: 64비트 프로세서

## 필수 소프트웨어 설치

### 1. Rust 설치

#### macOS

```bash
# Rust 설치
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 환경 변수 로드
source ~/.cargo/env

# 설치 확인
rustc --version
cargo --version
```

#### Windows

1. [rustup.rs](https://rustup.rs/)에서 `rustup-init.exe` 다운로드
2. 실행하여 설치 마법사 따라하기
3. PowerShell에서 확인:
   ```powershell
   rustc --version
   cargo --version
   ```

### 2. mise로 개발 환경 맞추기 (권장)

이 프로젝트는 [mise](https://mise.jdx.dev/)로 Node.js, Bun, Rust 버전을 고정합니다.

```bash
# mise 설치 (없는 경우)
curl https://mise.run | sh

# 저장소 루트에서 실행 시 mise.toml에 맞춰 node, bun, rust 설치
cd cheolsu-proxy
mise install

# 확인
node --version
bun --version
rustc --version
```

### 3. Node.js / Bun 직접 설치 (mise 미사용 시)

#### macOS (Homebrew)

```bash
brew install node bun
```

#### Windows

1. [nodejs.org](https://nodejs.org/)에서 LTS 버전 다운로드
2. [bun.sh](https://bun.sh/)에서 Bun 설치
3. PowerShell에서 확인:
   ```powershell
   node --version
   bun --version
   ```

### 4. Tauri CLI 설치

```bash
# Tauri CLI 설치
cargo install tauri-cli

# 설치 확인
tauri --version
```

### 5. 개발 도구 설치

#### Rust 도구

```bash
# 코드 포맷팅
rustup component add rustfmt

# 린터
rustup component add clippy

# 문서 생성
rustup component add rust-docs

# 소스 코드
rustup component add rust-src
```

#### 추가 Rust 도구

```bash
# 코드 분석
cargo install cargo-audit
cargo install cargo-outdated

# 성능 분석
cargo install flamegraph

# 테스트 커버리지
cargo install cargo-tarpaulin
```

## 프로젝트 설정

### 1. 저장소 클론

```bash
# 저장소 클론
git clone https://github.com/ohah/cheolsu-proxy.git
cd cheolsu-proxy

# 원격 저장소 설정 확인
git remote -v
```

### 2. 의존성 설치

#### Rust 의존성

```bash
# 루트 디렉토리에서
cargo build

# 모든 패키지 빌드
cargo build --workspace
```

#### Node.js 의존성

```bash
# Tauri UI 디렉토리로 이동
cd desktop

# 의존성 설치 (루트에서 워크스페이스 전체)
bun install
```

### 3. 개발 서버 실행

```bash
# Tauri 개발 서버 실행
cd desktop
bun run tauri dev

# bun 사용
bun run tauri dev
```

## IDE 설정

### VS Code (권장)

#### 필수 확장 프로그램

```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "vadimcn.vscode-lldb",
    "ms-vscode.vscode-typescript-next",
    "bradlc.vscode-tailwindcss",
    "oxc.oxc",
    "ms-vscode.vscode-json"
  ]
}
```

#### 설정 파일 (`.vscode/settings.json`)

```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "rust-lang.rust-analyzer",
  "[typescript]": {
    "editor.defaultFormatter": "oxc.oxc"
  },
  "[typescriptreact]": {
    "editor.defaultFormatter": "oxc.oxc"
  }
}
```

### IntelliJ IDEA / CLion

#### Rust 플러그인

1. **File** > **Settings** > **Plugins**
2. "Rust" 검색하여 설치
3. Rust toolchain 경로 설정

#### 설정

- **Rust toolchain**: `~/.cargo/bin/rustc`
- **Cargo path**: `~/.cargo/bin/cargo`

## 빌드 및 테스트

### 1. 빌드

```bash
# 개발 빌드
cargo build

# 릴리즈 빌드
cargo build --release

# 특정 패키지만 빌드
cargo build -p proxyapi_v2

# 모든 기능 포함 빌드
cargo build --features "native-tls-client,rcgen-ca,openssl-ca"
```

### 2. 테스트

```bash
# 모든 테스트 실행
cargo test

# 특정 테스트 실행
cargo test test_name

# 통합 테스트
cargo test --test integration_test

# 벤치마크
cargo bench
```

### 3. 코드 품질 검사

```bash
# 코드 포맷팅
cargo fmt

# 린터 실행
cargo clippy

# 보안 검사
cargo audit

# 오래된 의존성 확인
cargo outdated
```

## Tauri UI 개발

### 1. 개발 서버

```bash
# Tauri 앱과 함께 실행
bun run tauri dev
```

### 2. 빌드

```bash
# Tauri 앱 빌드
bun run tauri build
```

### 3. 테스트

추가 예정.

## 디버깅

#### Tauri 디버깅

```bash
# 디버그 모드로 실행
cd desktop
RUST_LOG=debug bun run tauri dev
```

개발자 도구 켜기

Cmd + shift + I

#### Tauri 빌드 오류

```bash
# 캐시 정리.
cargo clean
# Tauri CLI 재설치
cargo install tauri-cli --force

# 시스템 의존성 확인
# macOS: Xcode Command Line Tools
# Windows: Visual Studio Build Tools
```

### 플랫폼별 문제

#### macOS

```bash
# Xcode Command Line Tools 설치
xcode-select --install

# Homebrew 업데이트
brew update && brew upgrade
```

#### Windows

```bash
# Visual Studio Build Tools 설치
# https://visualstudio.microsoft.com/visual-cpp-build-tools/

# Windows SDK 설치 확인
```

## 다음 단계

환경 설정이 완료되면 다음 문서를 참조하세요:

- [프로젝트 구조](/ko/contributing/code-structure) - 코드베이스 구조 이해
- [테스트](/ko/contributing/testing) - 테스트 작성 및 실행
- [기여 가이드](/ko/contributing/) - 기여 프로세스
