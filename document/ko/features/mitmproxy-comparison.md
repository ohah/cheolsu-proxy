# mitmproxy 대비 기능 비교

mitmproxy를 레퍼런스로 한 cheolsu-proxy의 기능 구현 현황을 정리한 문서입니다.

> 마지막 업데이트: 2026-03-07

---

## 1. 프록시 모드

| 기능                    | mitmproxy | cheolsu-proxy | 상태   |
| ----------------------- | --------- | ------------- | ------ |
| Regular (HTTP CONNECT)  | O         | O             | 구현됨 |
| Upstream Proxy (체이닝) | O         | O             | 구현됨 |
| Reverse Proxy           | O         | X             | 미구현 |
| Transparent Proxy       | O         | X             | 미구현 |
| SOCKS5 Proxy            | O         | X             | 미구현 |
| WireGuard / TUN         | O         | X             | 미구현 |
| DNS Proxy               | O         | X             | 미구현 |

---

## 2. 요청/응답 조작

| 기능                       | mitmproxy | cheolsu-proxy | 상태   |
| -------------------------- | --------- | ------------- | ------ |
| 트래픽 보기                | O         | O             | 구현됨 |
| Intercept (가로채기/중단)  | O         | O             | 구현됨 |
| 헤더 수정 (ModifyRequest)  | O         | O             | 구현됨 |
| 바디 수정 (ModifyResponse) | O         | O             | 구현됨 |
| 요청 차단 (Block)          | O         | O             | 구현됨 |
| Map Local (로컬 파일 매핑) | O         | O             | 구현됨 |
| Map Remote (원격 재매핑)   | O         | O             | 구현됨 |

---

## 3. Replay 기능

| 기능                        | mitmproxy | cheolsu-proxy | 상태   |
| --------------------------- | --------- | ------------- | ------ |
| Client Replay (요청 재전송) | O         | O             | 구현됨 |
| Sequence Replay (배치 실행) | X         | O             | 구현됨 |
| Server Replay (응답 모킹)   | O         | O             | 구현됨 |

---

## 4. 필터링 & 차단

| 기능                               | mitmproxy | cheolsu-proxy | 상태   |
| ---------------------------------- | --------- | ------------- | ------ |
| 메서드별 필터링                    | O         | O             | 구현됨 |
| 고급 필터 (status, url, and/or 등) | O         | O             | 구현됨 |
| Blocklist (URL 차단)               | O         | O             | 구현됨 |
| IP 기반 접근 차단                  | O         | X             | 미구현 |

---

## 5. 콘텐츠 뷰어

| 기능                | mitmproxy | cheolsu-proxy     | 상태   |
| ------------------- | --------- | ----------------- | ------ |
| JSON 뷰어           | O         | O (Monaco Editor) | 구현됨 |
| XML/HTML 뷰어       | O         | O (Monaco Editor) | 구현됨 |
| GraphQL 뷰어        | O         | O (Monaco Editor) | 구현됨 |
| CSS/JS/TS 뷰어      | O         | O (Monaco Editor) | 구현됨 |
| 이미지 미리보기     | O         | O                 | 구현됨 |
| 비디오/오디오       | X         | O                 | 구현됨 |
| MQTT 뷰어           | O         | O                 | 구현됨 |
| Socket.IO 뷰어      | O         | O                 | 구현됨 |
| Multipart Form 뷰어 | O         | X                 | 미구현 |
| Protobuf 뷰어       | O         | X                 | 미구현 |

---

## 6. 데이터 내보내기/가져오기

| 기능                 | mitmproxy | cheolsu-proxy | 상태   |
| -------------------- | --------- | ------------- | ------ |
| cURL 복사            | O         | O             | 구현됨 |
| HAR 내보내기         | O         | O             | 구현됨 |
| httpie 복사          | O         | X             | 미구현 |
| Python requests 복사 | O         | X             | 미구현 |
| Flow 파일 저장/로드  | O         | X             | 미구현 |

---

## 7. 인증 & 보안

| 기능                            | mitmproxy | cheolsu-proxy           | 상태   |
| ------------------------------- | --------- | ----------------------- | ------ |
| CA 인증서 자동 생성             | O         | O                       | 구현됨 |
| TLS 1.0/1.1 레거시 지원         | O         | O (하이브리드 TLS 엔진) | 구현됨 |
| TLS Passthrough (자동 바이패스) | O         | O                       | 구현됨 |
| gzip/brotli 압축 해제           | O         | O                       | 구현됨 |
| 프록시 인증 (Basic/Digest)      | O         | X                       | 미구현 |
| Sticky Cookie                   | O         | X                       | 미구현 |
| Sticky Auth                     | O         | X                       | 미구현 |
| SSL Pinning 우회                | O         | X                       | 미구현 |

---

## 8. 프로토콜 지원

| 기능          | mitmproxy | cheolsu-proxy        | 상태   |
| ------------- | --------- | -------------------- | ------ |
| HTTP/1.1      | O         | O                    | 구현됨 |
| HTTP/2        | O         | O (ALPN 협상)        | 구현됨 |
| WebSocket     | O         | O (메시지 주입 포함) | 구현됨 |
| HTTP/3 (QUIC) | O         | X                    | 미구현 |
| Raw TCP       | O         | X                    | 미구현 |
| UDP           | O         | X                    | 미구현 |
| DNS           | O         | X                    | 미구현 |

---

## 9. 플러그인/스크립팅

| 기능                 | mitmproxy           | cheolsu-proxy | 상태   |
| -------------------- | ------------------- | ------------- | ------ |
| Addon/Plugin 시스템  | O (43개 내장 addon) | X             | 미구현 |
| 커스텀 스크립트 로딩 | O (Python)          | O (JS/TS)     | 구현됨 |
| 이벤트 훅 시스템     | O                   | O (onRequest/onResponse/onWebSocketMessage) | 구현됨 |
| MCP Server (AI 통합) | X                   | O             | 구현됨 |

---

## 10. UI/UX

| 기능                    | mitmproxy         | cheolsu-proxy | 상태   |
| ----------------------- | ----------------- | ------------- | ------ |
| GUI                     | O (mitmweb)       | O (Tauri)     | 구현됨 |
| TUI 모드                | O (mitmproxy TUI) | O (ratatui)   | 구현됨 |
| 다크/라이트 테마        | O                 | O             | 구현됨 |
| 시스템 프록시 자동 설정 | X (수동)          | O (macOS)     | 구현됨 |
| Flow 주석/코멘트        | O                 | X             | 미구현 |
| Flow 마킹               | O                 | X             | 미구현 |

---

## cheolsu-proxy만의 차별화 기능

mitmproxy에는 없지만 cheolsu-proxy에 구현된 기능:

- **MCP Server**: AI 어시스턴트(Claude Code, Cursor 등)에서 캡처된 트래픽을 직접 조회/조작 가능
- **하이브리드 TLS 엔진**: rustls + OpenSSL 자동 전환 (ClientHello 분석 기반)
- **데몬 아키텍처**: Unix Domain Socket 기반 다중 클라이언트 지원
- **시스템 프록시 자동 설정**: macOS networksetup 연동
- **JavaScript/TypeScript 스크립팅**: Deno Core 기반 3가지 훅 (onRequest, onResponse, onWebSocketMessage)
- **Server Replay**: 캡처된 응답을 캐싱하여 동일 요청에 자동 반환
- **스트리밍 응답 최적화**: SSE, Chunked, NDJSON 자동 감지 및 최적화
- **세션별 캐시 격리**: 세션 해시 기반 독립 캐시 디렉토리
- **시퀀스 리플레이**: 여러 요청을 순차 배치 실행
- **호스트/경로 트리 뷰**: 트래픽을 호스트별 계층 구조로 그룹핑
- **핀/체크박스 선택**: 트랜잭션 고정 및 다중 선택
- **비디오/오디오 미리보기**: MP4, WebM, MP3, WAV 등 미디어 재생
- **HAR 내보내기**: Desktop, TUI 모두 지원
- **CLI 설치**: `cheolsu` 터미널 명령어 설치/제거
- **자동 업데이트**: Tauri Updater 기반 GitHub Releases 배포
