# 릴리즈 노트

Cheolsu Proxy의 버전별 업데이트 내역을 확인할 수 있습니다.

## 최신 릴리즈

### v0.1.0 (2026-03-14)

첫 번째 공개 릴리즈로, 핵심 프록시 기능과 트래픽 조작 도구를 포함합니다.

📦 [GitHub Release 다운로드](https://github.com/ohah/cheolsu-proxy/releases/tag/v0.1.0)

:::warning 서명되지 않은 빌드
이 빌드는 코드 서명이 되어 있지 않습니다. `/Applications`에 복사한 후 터미널에서 다음 명령어를 실행하세요:

```bash
xattr -cr /Applications/Cheolsu\ Proxy.app
```

최초 1회만 실행하면 됩니다.
:::

**프록시 코어**:

- HTTP/HTTPS 트래픽 실시간 캡처 및 분석
- 하이브리드 TLS 엔진 (rustls + OpenSSL 자동 전환)
- Upstream Certificate Sniffing — 실제 서버 인증서 미러링
- TLS 자동 학습 바이패스 — MITM 거부 도메인 자동 우회
- SOCKS5 프록시 지원
- HTTP/2 지원 (클라이언트 및 업스트림)
- Upstream Proxy 지원 (HTTP/HTTPS/SOCKS5 체인)
- 네트워크 쓰로틀링 — 대역폭 제한 및 지연 시뮬레이션
- DNS Host Mapping — 커스텀 DNS 해석
- 프록시 인증 — Basic, Bearer, API Key
- SSL Proxying 화이트리스트/블랙리스트
- TLS Passthrough 화이트리스트/블랙리스트
- 도메인별 TLS 버전/암호화 스위트 설정
- Lazy/Eager 연결 전략
- 자동 CA 인증서 생성 및 시스템 설치
- macOS 시스템 프록시 자동 설정

**트래픽 조작**:

- 인터셉트 규칙 — 와일드카드 패턴 기반 (Block, Map Local, Map Remote, Rewrite)
- Breakpoint — 요청/응답 실시간 편집 후 전달
- 트래픽 리플레이 — 헤더/바디 편집 후 재전송
- Advanced Repeat — 설정 가능한 반복 횟수의 대량 리플레이
- 트래픽 비교(Diff) — 두 트랜잭션 나란히 비교
- Map Local / Map Remote — 로컬 파일 또는 다른 서버로 리다이렉트
- JavaScript/TypeScript 스크립팅 (deno_core 기반, async/await, 타이머, 핫 리로드)
- Server Replay (응답 캐싱 및 재사용)
- Quick Settings — No Caching, Block Cookies, No Gzip 토글

**프로토콜 지원**:

- WebSocket 모니터링 및 메시지 주입 (가상 스크롤)
- GraphQL, Socket.IO, MQTT 콘텐츠 뷰어
- gRPC/Protobuf 디코딩 (raw wire format 트리 뷰)
- SSE 스트리밍 캡처
- multipart/form-data 및 urlencoded 바디 뷰어

**인터페이스**:

- Desktop GUI (Tauri + React) — 다크 테마, i18n (한국어/영어), 시스템 트레이
- Terminal TUI (ratatui) — 네트워크, WebSocket, 규칙, 스크립트, Breakpoint, 설정, 로그 탭
- MCP Server — AI 어시스턴트 연동 (트래픽 조회, 규칙 관리, 스크립트 제어)

**내보내기 & 세션**:

- HAR 내보내기/가져오기 (HTTP Archive 1.2)
- 세션 저장/불러오기 및 자동 세션 복원
- 코드 내보내기 — cURL, fetch, HTTPie, Python requests
- 클립보드 복사

**인증서 관리**:

- 원클릭 macOS 키체인 설치
- 모바일 기기 인증서 배포 (내장 웹 서버 + QR 코드)
- Client SSL Certificate (mTLS) — 도메인별 설정, PKCS12 임포트

**기타**:

- Grafana Loki 스타일 쿼리 빌더
- 네이티브 macOS 메뉴바 (Proxy/Tools)
- 글로벌 단축키 (프록시 토글)
- 자동 업데이터 (Tauri Updater)
- 싱글 인스턴스
- 데몬 아키텍처 (다중 클라이언트 동시 연결)
- CLI 설치/제거 (데스크톱 앱에서)
- 로그 뷰어

**플랫폼 지원**:

- macOS (Apple Silicon / aarch64)
- Windows 및 Linux는 향후 지원 예정

## 업데이트 알림

새로운 릴리즈 알림을 받으려면:

1. [GitHub 저장소](https://github.com/ohah/cheolsu-proxy) 방문
2. **Watch** 버튼 클릭
3. **Releases only** 선택

## 피드백

버그 리포트나 기능 요청:

- [GitHub Issues](https://github.com/ohah/cheolsu-proxy/issues) — 버그 리포트 및 기능 요청
