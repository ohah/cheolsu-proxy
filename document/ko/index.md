# Cheolsu Proxy

Rust와 Tauri로 구축된 HTTP/HTTPS 디버깅 프록시입니다. 웹 브라우저와 서버 사이의 네트워크 트래픽을 실시간으로 캡처, 분석, 수정할 수 있습니다.

> 이 프로젝트는 [Proxelar](https://github.com/emanuele-em/proxelar)에서 포크하여 시작되었습니다.

## 주요 기능

- **HTTP/HTTPS 트래픽 캡처**: 실시간 네트워크 트래픽 모니터링 및 분석
- **WebSocket 지원**: Plain Text, Socket.IO, MQTT 프로토콜 감지 및 메시지 주입
- **인터셉트 규칙**: Block, Modify Request/Response, Map Local, Map Remote 5가지 액션
- **JavaScript/TypeScript 스크립팅**: Deno Core 기반 요청/응답/WebSocket 훅
- **Server Replay**: 캡처된 응답을 캐싱하여 동일 요청에 자동 반환
- **MCP Server**: Claude Code, Cursor 등 AI 어시스턴트 연동
- **Cheolsu-Query**: 고급 트래픽 필터링 쿼리 언어
- **TLS 지원**: 하이브리드 TLS 엔진 (rustls + native-tls 자동 전환)
- **HAR 내보내기**: HTTP Archive 1.2 형식 지원
- **3가지 인터페이스**: Desktop GUI (Tauri), Terminal TUI, MCP Server
- **데몬 아키텍처**: 여러 클라이언트가 동일한 프록시 데몬에 동시 연결 가능
- **시스템 프록시 자동 설정**: macOS networksetup 연동
- **Upstream Proxy**: 프록시 체인 구성 및 인증 지원

## 가이드

Cheolsu Proxy를 시작하기 위한 단계별 가이드입니다.

- [설치](/guide/installation) - 다운로드 및 설치
- [기본 사용법](/guide/basic-usage) - 프록시 시작, 트래픽 캡처, 요청 확인
- [SSL 인증서](/guide/ssl-certificates) - HTTPS 캡처를 위한 인증서 설치
- [프록시 설정](/guide/proxying) - 시스템 프록시, 포트 설정, 모바일 연결
- [트래픽 기록](/guide/recording) - 트래픽 뷰, 필터링, HAR 내보내기
- [세션 관리](/guide/sessions) - 세션 저장 및 불러오기
- [문제 해결](/guide/troubleshooting) - 자주 발생하는 문제 해결

## 기능 문서

- [MCP Server](/features/mcp-server) - AI 어시스턴트 연동
- [Cheolsu-Query](/features/cheolsu-query) - 네트워크 요청 필터링 쿼리 언어
- [인터셉트 규칙](/features/intercept-rules) - 요청/응답 가로채기 및 수정
- [스크립팅](/features/scripting) - JavaScript/TypeScript로 트래픽 조작
- [WebSocket](/features/websocket) - WebSocket 트래픽 모니터링 및 주입
- [Server Replay](/features/server-replay) - 캡처된 응답 캐싱 및 재사용
- [TLS 1.0/1.1 지원](/features/tls-support) - 레거시 TLS 클라이언트 지원

## 릴리즈 노트

최신 업데이트 정보와 개발 로드맵을 확인하세요.

- [릴리즈 노트](/releases/) - 버전별 업데이트 내역

## 기여하기

프로젝트에 기여하고 싶으시다면 [기여자 가이드](/contributing/)를 참조하세요.

- [개발 환경 설정](/contributing/development-setup)
- [프로젝트 구조](/contributing/code-structure)
- [테스트](/contributing/testing)

## 라이센스

이 프로젝트는 MIT 및 Apache 2.0 라이센스 하에 배포됩니다.

- [MIT 라이센스](https://github.com/ohah/cheolsu-proxy/blob/main/LICENSE-MIT)
- [Apache 2.0 라이센스](https://github.com/ohah/cheolsu-proxy/blob/main/LICENSE-APACHE)
