# QA/기업 환경을 위한 기능 로드맵

> Charles Proxy, mitmproxy 등 기존 도구와 비교하여 QA팀 및 기업 환경에서 필요한 기능을 정리한 문서입니다.

## 현재 구현된 기능

| 기능                  | 설명                                                     | 비교                                       |
| --------------------- | -------------------------------------------------------- | ------------------------------------------ |
| HTTPS 가로채기        | 사용자별 고유 CA 인증서 자동 생성                        | Charles/mitmproxy 동등                     |
| 인터셉트 규칙         | Block, ModifyRequest/Response, MapLocal, MapRemote       | Charles 동등                               |
| 서버 리플레이         | 캡처된 응답을 저장하고 동일 요청 시 자동 반환            | mitmproxy 동등                             |
| 요청 리플레이         | 개별 또는 순차적 요청 재전송                             | Charles/mitmproxy 동등                     |
| 스크립팅              | TypeScript/JavaScript 기반 요청/응답/WebSocket 조작      | mitmproxy의 addon과 유사, TS 지원은 차별화 |
| 네트워크 스로틀링     | GPRS ~ WiFi 프리셋 포함, 커스텀 설정 가능                | Charles 동등                               |
| WebSocket 캡처/주입   | 실시간 메시지 모니터링 및 주입, Socket.IO/MQTT 자동 감지 | Charles보다 우위                           |
| HAR 내보내기          | HTTP Archive 형식으로 트래픽 내보내기                    | 표준 호환                                  |
| MCP 서버 연동         | AI 어시스턴트(Claude 등)와 직접 연동                     | **고유 차별화**                            |
| 다중 인터페이스       | GUI, TUI, CLI, Headless 모드                             | **고유 차별화**                            |
| TLS 1.0/1.1 지원      | 레거시 클라이언트를 위한 하이브리드 TLS 핸들러           | 대부분의 도구에서 미지원                   |
| Breakpoints           | 요청/응답 일시 정지, Forward/Drop/Abort/ModifyAndForward | Charles 동등                               |
| 세션 저장/불러오기    | .cheolsu 세션 파일 (트래픽+규칙+설정), HAR 가져오기      | Charles 동등                               |
| 모바일 CA 인증서 배포 | 웹 기반 다운로드 페이지, iOS/Android 설치 가이드         | Charles 동등                               |
| Host Mapping (DNS)    | 도메인→IP 매핑, 와일드카드 패턴, 포트 지정               | Charles 동등                               |
| 트래픽 비교 (Diff)    | 헤더/바디/JSON 구조적 diff                               | Charles 동등                               |
| SOCKS5 Proxy          | RFC 1929 인증 포함 완전 구현                             | Charles 동등                               |
| Protobuf 디코딩       | Wire type 기반 자동 디코딩, gRPC Content-Type 감지       | mitmproxy 동등                             |
| Waterfall 타이밍 분석 | 크롬 DevTools 스타일 시간축 차트, 응답 시간별 색상 코딩  | Charles 동등                               |
| 본문 뷰어 강화        | Hex 뷰어, 이미지 미리보기, Protobuf 트리뷰, Multipart   | Charles 동등                               |
| 조건부 Throttle       | 인터셉트 규칙별 조건부 응답 스트림 속도 제한             | Charles보다 우위                           |
| OpenAPI 스펙 생성     | 캡처된 트래픽에서 OpenAPI 3.0 스펙 자동 생성/내보내기    | **고유 차별화**                            |

---

## 미구현 기능 목록

### 우선순위: 중간

#### 1. Waterfall 개별 단계 타이밍 측정

현재 Waterfall 차트 시각화(시간축 기반 바 차트, 응답 시간별 색상 코딩)는 구현되었으나, 개별 네트워크 단계별 타이밍 분해는 미구현.

**미구현 범위:**

- 개별 단계 타이밍 측정: DNS Lookup, TCP Connect, TLS Handshake, TTFB, Content Transfer (데이터 구조만 정의됨)
- 느린 요청 자동 하이라이트 (임계값 설정 가능)
- 통계 요약: 평균/p95/p99 응답 시간, 도메인별 집계

---

#### 2. 클라이언트별 트래픽 분리

여러 기기/사용자의 트래픽을 구분하여 관리하는 기능.

**구현 범위:**

- 클라이언트 IP별 자동 태깅
- 사용자 정의 태그/라벨 부여
- 태그별 필터링
- 프록시 접속 인증 (Basic Auth)

---

### 우선순위: 낮음

#### 3. 자동 응답 검증 (Contract Testing)

API 응답이 정의된 스펙과 일치하는지 실시간으로 검증하는 기능. 현재 OpenAPI 스펙 자동 생성/내보내기는 구현되었으나, 실시간 검증은 미구현.

**미구현 범위:**

- 외부 OpenAPI/Swagger 스펙 파일 로드
- 실시간 요청/응답을 스펙과 대조
- 불일치 항목 경고 (누락된 필드, 타입 불일치, 예상 외 상태 코드 등)
- 검증 결과 리포트 생성

---

## 우선순위 요약

| 우선순위 | 기능                            | 상태             |
| -------- | ------------------------------- | ---------------- |
| ~~높음~~ | ~~Breakpoint (실시간 편집)~~    | ✅ 구현 완료     |
| ~~높음~~ | ~~세션 저장/불러오기~~          | ✅ 구현 완료     |
| ~~높음~~ | ~~모바일 CA 인증서 배포~~       | ✅ 구현 완료     |
| ~~중간~~ | ~~타이밍 분석 (Waterfall)~~     | ✅ 부분 구현 (차트 완료, 단계별 타이밍 미구현) |
| ~~중간~~ | ~~DNS Spoofing / Host Mapping~~ | ✅ 구현 완료     |
| ~~중간~~ | ~~트래픽 비교 (Diff)~~          | ✅ 구현 완료     |
| 중간     | 클라이언트별 트래픽 분리        | 팀 환경에서 필수 |
| ~~낮음~~ | ~~gRPC / Protobuf~~             | ✅ 구현 완료     |
| ~~낮음~~ | ~~본문 뷰어 강화~~              | ✅ 구현 완료     |
| 낮음     | 자동 응답 검증                  | 부분 구현 (스펙 생성만) |
