# 릴리즈 노트

Cheolsu Proxy의 버전별 업데이트 내역을 확인할 수 있습니다.

## 최신 릴리즈

### v0.1.0 (개발 중)

첫 번째 공개 릴리즈로, 핵심 프록시 기능과 트래픽 조작 도구를 포함합니다.

**핵심 기능**:

- HTTP/HTTPS 트래픽 실시간 캡처 및 분석
- 하이브리드 TLS 엔진 (rustls + native-tls 자동 전환)
- 자동 CA 인증서 생성 및 시스템 설치
- macOS 시스템 프록시 자동 설정

**트래픽 조작**:

- 인터셉트 규칙 (Block, Modify Request/Response, Map Local, Map Remote)
- JavaScript/TypeScript 스크립팅 (Deno Core 기반)
- Server Replay (응답 캐싱 및 재사용)

**프로토콜 지원**:

- WebSocket 모니터링 및 메시지 주입 (Plain Text, Socket.IO, MQTT)
- Upstream Proxy 지원 (프록시 체인, 인증)
- gRPC/Protobuf 디코딩

**인터페이스**:

- Desktop GUI (Tauri)
- Terminal TUI
- MCP Server (AI 어시스턴트 연동)

**기타**:

- Cheolsu-Query 필터링 언어
- HAR 내보내기 (HTTP Archive 1.2)
- 세션 저장/불러오기
- cURL 및 다양한 코드 내보내기
- 데몬 아키텍처 (다중 클라이언트 동시 연결)

## 업데이트 알림

새로운 릴리즈 알림을 받으려면:

1. [GitHub 저장소](https://github.com/ohah/cheolsu-proxy) 방문
2. **Watch** 버튼 클릭
3. **Releases only** 선택

## 피드백

버그 리포트나 기능 요청:

- [GitHub Issues](https://github.com/ohah/cheolsu-proxy/issues) - 버그 리포트 및 기능 요청
- [GitHub Discussions](https://github.com/ohah/cheolsu-proxy/discussions) - 토론 및 질문
