# mitmproxy 대비 기능 비교

mitmproxy를 레퍼런스로 한 cheolsu-proxy의 기능 구현 현황을 정리한 문서입니다.

> 마지막 업데이트: 2026-03-06

---

## 1. 프록시 모드

현재 Regular(HTTP CONNECT) 모드만 지원합니다.

| 기능                    | mitmproxy | cheolsu-proxy | 상태   |
| ----------------------- | --------- | ------------- | ------ |
| Regular (HTTP CONNECT)  | O         | O             | 구현됨 |
| Reverse Proxy           | O         | X             | 미구현 |
| Transparent Proxy       | O         | X             | 미구현 |
| SOCKS5 Proxy            | O         | X             | 미구현 |
| Upstream Proxy (체이닝) | O         | X             | 미구현 |
| WireGuard / TUN         | O         | X             | 미구현 |
| DNS Proxy               | O         | X             | 미구현 |

---

## 2. 요청/응답 조작

현재 트래픽을 읽기 전용으로만 표시합니다.

| 기능                       | mitmproxy | cheolsu-proxy | 상태   |
| -------------------------- | --------- | ------------- | ------ |
| 트래픽 보기                | O         | O             | 구현됨 |
| Intercept (가로채기/중단)  | O         | X             | 미구현 |
| 헤더 수정 (modifyheaders)  | O         | X             | 미구현 |
| 바디 수정 (modifybody)     | O         | X             | 미구현 |
| URL 리다이렉트             | O         | X             | 미구현 |
| Map Local (로컬 파일 매핑) | O         | X             | 미구현 |
| Map Remote (원격 재매핑)   | O         | X             | 미구현 |

---

## 3. Replay 기능

| 기능                        | mitmproxy | cheolsu-proxy | 상태   |
| --------------------------- | --------- | ------------- | ------ |
| Client Replay (요청 재전송) | O         | X             | 미구현 |
| Server Replay (응답 모킹)   | O         | X             | 미구현 |

---

## 4. 필터링 & 차단

| 기능                                | mitmproxy | cheolsu-proxy | 상태   |
| ----------------------------------- | --------- | ------------- | ------ |
| 메서드별 필터링                     | O         | O             | 구현됨 |
| 고급 Flow 필터 (regex, 상태코드 등) | O         | X             | 미구현 |
| Blocklist (URL 차단)                | O         | X             | 미구현 |
| IP 기반 접근 차단                   | O         | X             | 미구현 |

---

## 5. 콘텐츠 뷰어

| 기능                | mitmproxy | cheolsu-proxy     | 상태   |
| ------------------- | --------- | ----------------- | ------ |
| JSON 뷰어           | O         | O (Monaco Editor) | 구현됨 |
| XML/HTML 뷰어       | O         | X                 | 미구현 |
| GraphQL 뷰어        | O         | X                 | 미구현 |
| Multipart Form 뷰어 | O         | X                 | 미구현 |
| 이미지 미리보기     | O         | X                 | 미구현 |
| Protobuf 뷰어       | O         | X                 | 미구현 |
| MQTT 뷰어           | O         | X                 | 미구현 |
| Socket.IO 뷰어      | O         | X                 | 미구현 |

---

## 6. 데이터 내보내기/가져오기

| 기능                 | mitmproxy | cheolsu-proxy | 상태   |
| -------------------- | --------- | ------------- | ------ |
| HAR 내보내기         | O         | X             | 미구현 |
| cURL 복사            | O         | X             | 미구현 |
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
| 프록시 인증 (Basic/Digest)      | O         | X                       | 미구현 |
| Sticky Cookie                   | O         | X                       | 미구현 |
| Sticky Auth                     | O         | X                       | 미구현 |
| Anti-Cache (캐시 헤더 제거)     | O         | X                       | 미구현 |
| Anti-Compression (비압축 요청)  | O         | X                       | 미구현 |
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
| 커스텀 스크립트 로딩 | O (Python)          | X             | 미구현 |
| 이벤트 훅 시스템     | O                   | X             | 미구현 |

---

## 10. UI/UX

| 기능                    | mitmproxy         | cheolsu-proxy | 상태   |
| ----------------------- | ----------------- | ------------- | ------ |
| GUI                     | O (mitmweb)       | O (Tauri)     | 구현됨 |
| CLI 모드                | O (mitmproxy TUI) | O (headless)  | 구현됨 |
| 다크/라이트 테마        | O                 | O             | 구현됨 |
| 시스템 프록시 자동 설정 | X (수동)          | O (macOS)     | 구현됨 |
| Flow 주석/코멘트        | O                 | X             | 미구현 |
| Flow 마킹               | O                 | X             | 미구현 |

---

## 구현 우선순위 제안

### 높은 우선순위 (프록시 도구 핵심 기능)

1. **Intercept** - 요청/응답 가로채기 및 수정 (MITM 프록시의 핵심 기능)
2. **cURL/HAR 내보내기** - 디버깅 워크플로우 필수 기능
3. **Client Replay** - 요청 재전송
4. **고급 필터링** - URL, 상태코드, 헤더 기반 필터

### 중간 우선순위

5. Reverse Proxy 모드
6. Map Local / Map Remote
7. Blocklist (URL 차단)
8. 콘텐츠 뷰어 확장 (XML/HTML, GraphQL 등)
9. Anti-Cache / Anti-Compression

### 낮은 우선순위

10. SOCKS5, Transparent, Upstream 모드
11. HTTP/3 (QUIC)
12. Plugin/Script 시스템
13. DNS Proxy
14. Raw TCP / UDP

---

## cheolsu-proxy만의 차별화 기능

mitmproxy에는 없지만 cheolsu-proxy에 구현된 기능:

- **하이브리드 TLS 엔진**: rustls + OpenSSL 자동 전환 (ClientHello 분석 기반)
- **데몬 아키텍처**: Unix Domain Socket 기반 다중 클라이언트 지원
- **시스템 프록시 자동 설정**: macOS networksetup 연동
- **스트리밍 응답 최적화**: SSE, Chunked, NDJSON 자동 감지 및 최적화
- **세션별 캐시 격리**: 세션 해시 기반 독립 캐시 디렉토리
