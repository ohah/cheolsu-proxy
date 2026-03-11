<div align="center">
<img style="width:100px; margin:auto" src="assets/logo.png">
<h1> Cheolsu Proxy </h1>
<h2> 간단한 <i>Man In The Middle</i> 프록시</h2>

[English](README.md)

</div>

![GitHub](https://img.shields.io/github/license/ohah/cheolsu-proxy/)
![GitHub last commit](https://img.shields.io/github/last-commit/ohah/cheolsu-proxy/)
![GitHub top language](https://img.shields.io/github/languages/top/ohah/cheolsu-proxy/)

## 소개

Rust 기반 **Man in the Middle 프록시**로, HTTP/HTTPS/WebSocket 트래픽을 검사하고 조작할 수 있습니다. 데스크톱 GUI (Tauri + React), 터미널 UI (Ratatui), headless CLI 모드를 제공하며, 스크립트 기반 트래픽 조작과 인터셉트 규칙을 지원합니다. MCP를 통해 AI 어시스턴트와도 연동됩니다.

## 기능

### 핵심

- HTTP / HTTPS 트래픽 가로채기
- WebSocket 캡처 및 메시지 인젝션
- TLS 1.0/1.1 레거시 클라이언트 지원 (하이브리드 TLS 핸들러)
- 요청/응답 상세 조회 (헤더, 바디, 미디어 프리뷰)
- 업스트림 프록시 지원 (HTTP/HTTPS/SOCKS, 인증 포함)
- 네트워크 대역폭 제어 (Throttling)

### 트래픽 조작

- **인터셉트 규칙** — 요청 차단, 요청/응답 수정, 로컬 파일 매핑, 원격 URL 리다이렉트
- **서버 리플레이** — 캡처된 응답을 저장하고 동일 요청 시 자동 반환
- **요청 리플레이** — 개별 또는 순차적 요청 재전송
- **스크립팅** — TypeScript 기반 요청/응답/WebSocket 조작 (Deno Core / V8)

### 필터링 및 내보내기

- **Cheolsu-Query** — 트래픽 필터링 전용 쿼리 언어 (method, status, URL + 논리 연산자)
- **HAR 내보내기** — HTTP Archive 형식으로 트래픽 내보내기

### 인터페이스

- GUI 데스크톱 앱 (Tauri + React)
- TUI 터미널 인터페이스 (Ratatui)
- CLI headless 모드
- MCP 서버 (AI 어시스턴트 연동)
- 다크 / 라이트 테마
- 다국어 지원 (영어, 한국어)
- 글로벌 단축키

### 보안

- 사용자별 고유 CA 인증서 자동 생성 (개인키는 바이너리에 포함되지 않음)

## 시작하기

### 1. 인증서 자동 생성 및 설치

Cheolsu Proxy는 첫 실행 시 자동으로 고유한 CA 인증서를 생성합니다.

**macOS에서 인증서 수동 설치:**

1. 앱을 실행하면 콘솔에 인증서 파일 경로가 표시됩니다:

   ```
   📁 경로: ~/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer
   ```

2. Keychain Access 앱을 실행하세요
3. 'login' 키체인을 선택하세요
4. File > Import Items... 메뉴를 선택하세요
5. 위 경로의 `cheolsu-proxy.cer` 파일을 선택하세요
6. 인증서를 더블클릭하고 '항상 신뢰'로 설정하세요

**다른 OS 가이드:**

- [Ubuntu 가이드](https://ubuntu.com/server/docs/security-trust-store)
- [Windows 가이드](https://learn.microsoft.com/en-us/skype-sdk/sdn/articles/installing-the-trusted-root-certificate)

### 2. 시스템 프록시 설정

로컬 시스템 프록시를 `127.0.0.1:8100`으로 설정하세요.

- [macOS 가이드](https://support.apple.com/it-it/guide/mac-help/mchlp2591/mac)
- [Ubuntu 가이드](https://help.ubuntu.com/stable/ubuntu-help/net-proxy.html.en)
- [Windows 가이드](https://support.microsoft.com/en-us/windows/use-a-proxy-server-in-windows-03096c53-0554-4ffe-b6ab-8b1deee8dae1)

## 문서

자세한 문서는 [공식 문서 사이트](https://ohah.github.io/cheolsu-proxy)를 참조하세요.

- **사용자 가이드**: 설치, 설정, 사용법
- **기여자 가이드**: 개발 환경 설정, 코드 구조, 기여 방법
- **기능 문서**: TLS 지원, 인증서 설정 등

### 로컬 문서

마크다운 문서는 [document/](document/) 디렉토리를 참조하세요.

- [TLS 1.0/1.1 지원](document/features/TLS_1_0_1_1_SUPPORT.md) — 레거시 TLS 클라이언트 지원
- [인터셉트 규칙](document/ko/features/intercept-rules.md) — 트래픽 차단, 수정, 리다이렉트
- [서버 리플레이](document/ko/features/server-replay.md) — 캡처된 응답 자동 반환
- [스크립팅](document/ko/features/scripting.md) — TypeScript 기반 트래픽 조작
- [WebSocket](document/ko/features/websocket.md) — WebSocket 캡처 및 인젝션
- [MCP 서버](document/ko/features/mcp-server.md) — AI 어시스턴트 연동
- [Cheolsu-Query](document/ko/features/cheolsu-query.md) — 트래픽 필터링 쿼리 언어

## 개발

### 사전 요구사항

- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/) 또는 npm
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)
- OpenSSL 3.x (빌드에 필요)

**macOS:**

```bash
brew install openssl@3 pkg-config
```

### 1. 인증서 생성

프록시가 HTTPS 트래픽을 중간에서 분석하려면 CA 인증서가 필요합니다. 프로젝트 루트의 `crates/` 디렉토리에서 인증서 생성 스크립트를 실행하세요:

```bash
cd crates
bash ../install_cer.sh
```

> 스크립트가 `crates/proxyapi_v2/src/certificate_authority/` 디렉토리에 인증서 파일들을 생성합니다.
> 개인키는 PKCS#8 형식으로 변환됩니다 (rcgen 라이브러리 호환).

### 2. CA 인증서 시스템 신뢰 등록

생성된 CA 인증서를 운영체제에 등록해야 HTTPS 프록시가 정상 동작합니다.

**macOS:**

```bash
open crates/proxyapi_v2/src/certificate_authority/cheolsu-proxy.cer
```

Keychain Access가 열리면 인증서를 더블클릭 → "신뢰" 섹션 → **"항상 신뢰"**로 설정하세요.

**Linux:**

```bash
sudo cp crates/proxyapi_v2/src/certificate_authority/cheolsu-proxy.cer /usr/local/share/ca-certificates/cheolsu-proxy.crt
sudo update-ca-certificates
```

**Windows:**

인증서 관리자(`certmgr.msc`)에서 "신뢰할 수 있는 루트 인증 기관"에 `cheolsu-proxy.cer`를 가져오세요.

### 3. 개발 서버 실행

```bash
cd desktop
bun install
bun tauri dev
```

OpenSSL 링크 오류가 발생하면 환경변수를 설정하세요:

```bash
PKG_CONFIG_PATH="/opt/homebrew/opt/openssl@3/lib/pkgconfig" bun tauri dev
```

### 4. CLI (Headless) 모드

GUI 없이 프록시만 실행할 수 있습니다:

```bash
bun tauri dev -- -- --headless --port 8100
```

| 옵션            | 단축 | 설명                                 |
| --------------- | ---- | ------------------------------------ |
| `--headless`    | `-H` | GUI 없이 프록시만 실행               |
| `--port <PORT>` | `-p` | 프록시 리슨 포트 (기본: 8100)        |
| `--host <HOST>` | `-b` | 프록시 리슨 호스트 (기본: 127.0.0.1) |
| `--verbose`     | `-v` | 상세 로깅 활성화                     |

### 5. 테스트

```bash
# 단위 테스트 (TLS 전략 선택, ClientHello 파싱 등)
PKG_CONFIG_PATH="/opt/homebrew/opt/openssl@3/lib/pkgconfig" cargo test -p proxyapi_v2 --lib

# 통합 테스트 (CA 인증서 시스템 신뢰 등록 필요)
PKG_CONFIG_PATH="/opt/homebrew/opt/openssl@3/lib/pkgconfig" cargo test -p proxyapi_v2 --test rcgen_ca
```

### 인증서 파일 위치

- **개발 환경**: `crates/proxyapi_v2/src/certificate_authority/`
- **프로덕션 (macOS)**: `~/Library/Application Support/com.cheolsu-proxy/`
- **프로덕션 (Windows)**: `%APPDATA%/com.cheolsu-proxy/` (향후 지원)
- **프로덕션 (Linux)**: `~/.config/com.cheolsu-proxy/` (향후 지원)

## 도움말 및 토론

사용법에 대한 질문이 있으시면 [GitHub Discussions](https://github.com/ohah/cheolsu-proxy/discussions)를 이용해 주세요!

![GitHub Discussions](https://img.shields.io/github/discussions/ohah/cheolsu-proxy)

## 기여하기

기여는 언제나 환영합니다!

## 라이선스

자세한 내용은 [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT)를 참조하세요.
