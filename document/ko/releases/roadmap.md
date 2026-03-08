# 지원 예정

Cheolsu Proxy의 향후 지원 예정 기능들을 확인할 수 있습니다.

## 개발 로드맵

### 구현 완료된 기능

- **트래픽 캡처**: HTTP/HTTPS 실시간 모니터링
- **WebSocket**: Plain Text, Socket.IO, MQTT 프로토콜 감지 및 메시지 주입
- **인터셉트 규칙**: Block, Modify Request/Response, Map Local, Map Remote
- **스크립팅**: JavaScript/TypeScript 기반 요청/응답/WebSocket 훅
- **Server Replay**: 캡처된 응답 캐싱 및 자동 반환
- **MCP Server**: AI 어시스턴트 연동 (Claude Code, Cursor 등)
- **HAR 내보내기**: Desktop, TUI 모두 지원
- **Upstream Proxy**: 프록시 체인 및 인증 지원
- **시스템 프록시 자동 설정**: macOS networksetup 연동
- **CLI 설치**: `cheolsu` 터미널 명령어 설치/제거
- **자동 업데이트**: Tauri Updater 기반 GitHub Releases 배포
- **TLS 하이브리드 엔진**: rustls + native-tls 자동 전환
- **TLS Passthrough**: 자동 학습 기반 바이패스

### 추후 지원 예정 기능

- **플랫폼 지원**:
  - Windows 지원

- **프록시 모드**:
  - Reverse Proxy
  - Transparent Proxy
  - SOCKS5 Proxy

- **내보내기/가져오기**:
  - 세션 저장/불러오기

- **확장 기능**:
  - 플러그인 시스템

## 업데이트 알림

새로운 릴리즈가 나올 때마다 GitHub에서 알림을 받으려면:

1. [GitHub 저장소](https://github.com/ohah/cheolsu-proxy) 방문
2. **Watch** 버튼 클릭
3. **Releases only** 선택

## 피드백

기능 요청이나 제안이 있으시면:

- [GitHub Issues](https://github.com/ohah/cheolsu-proxy/issues)에서 이슈 생성
- [GitHub Discussions](https://github.com/ohah/cheolsu-proxy/discussions)에서 토론 참여

---

**최신 정보**: 개발 진행 상황은 [GitHub](https://github.com/ohah/cheolsu-proxy)에서 확인하세요.
