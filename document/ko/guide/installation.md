# 설치 가이드

Cheolsu Proxy를 설치하고 첫 실행까지의 과정을 안내합니다.

---

## 시스템 요구사항

### macOS

- **운영체제**: macOS 12 (Monterey) 이상
- **아키텍처**: Apple Silicon (M1/M2/M3/M4) 및 Intel 모두 지원
- **디스크 공간**: 최소 200MB 이상의 여유 공간
- **권한**: 프록시 설정 변경을 위한 관리자 권한 필요

### Windows

> Windows 지원은 현재 준비 중입니다. 향후 릴리즈에서 제공될 예정이며, 진행 상황은 [로드맵](../releases/roadmap.md)을 참고하세요.

---

## 다운로드

최신 버전은 GitHub Releases 페이지에서 다운로드할 수 있습니다.

1. [GitHub Releases](https://github.com/ohah/cheolsu-proxy/releases) 페이지에 접속합니다
2. 최신 릴리즈의 Assets 섹션에서 운영체제에 맞는 파일을 다운로드합니다
   - **macOS (Apple Silicon)**: `Cheolsu.Proxy_x.x.x_aarch64.dmg`
   - **macOS (Intel)**: `Cheolsu.Proxy_x.x.x_x64.dmg`

> 본인의 Mac이 Apple Silicon인지 Intel인지 확인하려면, 좌측 상단 Apple 메뉴 → **이 Mac에 관하여**에서 칩 정보를 확인하세요.

---

## macOS 설치

### .dmg 파일로 설치

1. 다운로드한 `.dmg` 파일을 더블클릭하여 디스크 이미지를 마운트합니다
2. **Cheolsu Proxy** 아이콘을 **Applications** 폴더로 드래그하여 복사합니다
3. 설치가 완료되면 디스크 이미지를 언마운트(추출)합니다

### 첫 실행 시 macOS Gatekeeper 안내

Cheolsu Proxy를 처음 실행하면 macOS Gatekeeper가 실행을 차단할 수 있습니다. 이는 앱이 Apple의 공증(notarization)을 거치지 않은 경우 발생하는 정상적인 보안 동작입니다.

**해결 방법:**

1. Finder에서 Applications 폴더의 **Cheolsu Proxy**를 **Control + 클릭** (또는 우클릭)합니다
2. 컨텍스트 메뉴에서 **열기**를 선택합니다
3. "확인되지 않은 개발자" 경고 대화상자에서 **열기** 버튼을 클릭합니다

> 이 과정은 최초 실행 시 한 번만 필요합니다. 이후에는 정상적으로 실행됩니다.

또는 **시스템 설정 → 개인정보 보호 및 보안** 하단에서 "확인 없이 열기" 옵션을 통해 허용할 수도 있습니다.

---

## 인터페이스 소개

Cheolsu Proxy는 세 가지 인터페이스를 제공합니다. 사용 목적과 환경에 따라 적합한 인터페이스를 선택하세요.

### 1. Desktop GUI

Tauri 기반의 데스크톱 애플리케이션입니다. 트래픽 테이블, 상세 보기, 인터셉트 규칙 설정 등 모든 기능을 시각적으로 사용할 수 있습니다. 대부분의 사용자에게 권장되는 기본 인터페이스입니다.

### 2. TUI (Terminal User Interface)

터미널에서 동작하는 텍스트 기반 인터페이스입니다. SSH 환경이나 GUI를 사용할 수 없는 서버 환경에서 유용합니다. Desktop GUI와 동일한 프록시 데몬에 연결하여 트래픽을 모니터링합니다.

### 3. MCP Server

[Model Context Protocol](https://modelcontextprotocol.io/) 서버로, Claude Code, Cursor 등 AI 어시스턴트에서 캡처된 트래픽을 직접 조회하고 조작할 수 있습니다. 자세한 내용은 [MCP Server](../features/mcp-server.md) 문서를 참고하세요.

---

## TUI 실행 방법

TUI는 터미널에서 별도의 바이너리로 실행합니다. Desktop GUI와 동일한 프록시 데몬에 연결되므로, 두 인터페이스를 동시에 사용할 수도 있습니다.

```bash
cheolsu-proxy-tui
```

> TUI 바이너리는 Desktop 앱과 별도로 제공됩니다. GitHub Releases에서 TUI 바이너리를 확인하세요.

---

## 다음 단계

설치가 완료되었다면 다음 가이드를 따라 진행하세요.

- **[기본 사용법](./basic-usage.md)**: 프록시 시작, 트래픽 확인, HAR 내보내기 등 기본적인 사용법을 익힙니다
- **[SSL 인증서 설치](./ssl-certificates.md)**: HTTPS 트래픽을 가로채려면 인증서 설치가 필요합니다
- **[프록시 설정](./proxying.md)**: 시스템 프록시 설정 및 모바일 기기 연결 방법을 알아봅니다
