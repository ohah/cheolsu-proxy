# 가이드

Cheolsu Proxy는 Rust와 Tauri로 만든 HTTP/HTTPS 디버깅 프록시입니다. 웹 브라우저나 애플리케이션과 서버 사이에 위치하여 네트워크 트래픽을 실시간으로 모니터링하고 분석할 수 있습니다. API 개발, 프론트엔드 디버깅, 모바일 앱 테스트 등 다양한 상황에서 활용할 수 있습니다.

## 시작하기

### [설치](./installation.md)

Cheolsu Proxy를 다운로드하고 시스템에 설치하는 방법을 안내합니다. macOS를 지원하며, Desktop GUI, TUI, MCP Server 3가지 인터페이스를 제공합니다.

### [기본 사용법](./basic-usage.md)

프록시를 시작하고 트래픽을 캡처하는 기본적인 과정을 설명합니다. 처음 사용하는 분들은 여기서 시작하세요.

### [SSL 인증서](./ssl-certificates.md)

HTTPS 트래픽을 캡처하려면 Cheolsu Proxy의 CA 인증서를 시스템에 신뢰 등록해야 합니다. 플랫폼별 인증서 설치 방법을 안내합니다.

## 핵심 개념

### [프록시 설정](./proxying.md)

Cheolsu Proxy가 프록시로서 동작하는 방식과 시스템 프록시 설정, 수동 프록시 설정, Upstream Proxy, 데몬 아키텍처 등을 설명합니다.

### [트래픽 기록](./recording.md)

트래픽 캡처, 테이블/트리 뷰, 요청/응답 상세 보기, HAR 내보내기 등 기록 관련 기능을 안내합니다.

### [세션 관리](./sessions.md)

캡처된 트래픽을 세션으로 저장하고 불러오는 방법을 설명합니다.

## [문제 해결](./troubleshooting.md)

프록시 시작 실패, 웹사이트 접속 불가, HTTPS 인증서 문제, 성능 이슈 등 자주 발생하는 문제의 해결 방법을 안내합니다.

## 고급 기능

트래픽 캡처 이상의 고급 기능을 활용하여 요청/응답을 조작하고 자동화할 수 있습니다.

- [Cheolsu-Query](/features/cheolsu-query) - 네트워크 요청 필터링 쿼리 언어
- [인터셉트 규칙](/features/intercept-rules) - 요청 차단, 수정, 리다이렉트
- [스크립팅](/features/scripting) - JavaScript/TypeScript로 트래픽 자동 조작
- [WebSocket](/features/websocket) - WebSocket 트래픽 모니터링 및 주입
- [Server Replay](/features/server-replay) - 캡처된 응답 캐싱 및 재사용
- [MCP Server](/features/mcp-server) - AI 어시스턴트 연동

## 주의사항

**개발 및 테스트 목적으로만 사용하세요.** Cheolsu Proxy는 네트워크 트래픽을 가로채고 수정할 수 있는 강력한 도구입니다. 다음 사항을 반드시 준수해주세요.

1. **악의적 사용 금지** - 데이터 위변조, 무단 트래픽 가로채기 등 악의적 목적으로 사용하지 마세요.
2. **회사 환경 규정 준수** - 기업 네트워크에서 사용하는 경우 IT 부서의 승인을 받고 관련 규정을 확인하세요.
3. **개인정보 보호** - 민감한 개인정보가 포함된 트래픽을 가로채지 마세요. 로그에 민감 정보가 기록되지 않도록 주의하세요.
4. **법적 책임** - Cheolsu Proxy 사용에 따른 모든 법적 책임은 사용자에게 있습니다.

---

**시작하기**: [설치](./installation.md) 또는 이미 설치했다면 [기본 사용법](./basic-usage.md)부터 시작하세요.
