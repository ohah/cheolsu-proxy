# QA/기업 환경을 위한 기능 로드맵

> Charles Proxy, mitmproxy 등 기존 도구와 비교하여 QA팀 및 기업 환경에서 필요한 기능을 정리한 문서입니다.

## 현재 구현된 기능

| 기능 | 설명 | 비교 |
|------|------|------|
| HTTPS 가로채기 | 사용자별 고유 CA 인증서 자동 생성 | Charles/mitmproxy 동등 |
| 인터셉트 규칙 | Block, ModifyRequest/Response, MapLocal, MapRemote | Charles 동등 |
| 서버 리플레이 | 캡처된 응답을 저장하고 동일 요청 시 자동 반환 | mitmproxy 동등 |
| 요청 리플레이 | 개별 또는 순차적 요청 재전송 | Charles/mitmproxy 동등 |
| 스크립팅 | TypeScript/JavaScript 기반 요청/응답/WebSocket 조작 | mitmproxy의 addon과 유사, TS 지원은 차별화 |
| 네트워크 스로틀링 | GPRS ~ WiFi 프리셋 포함, 커스텀 설정 가능 | Charles 동등 |
| WebSocket 캡처/주입 | 실시간 메시지 모니터링 및 주입, Socket.IO/MQTT 자동 감지 | Charles보다 우위 |
| HAR 내보내기 | HTTP Archive 형식으로 트래픽 내보내기 | 표준 호환 |
| MCP 서버 연동 | AI 어시스턴트(Claude 등)와 직접 연동 | **고유 차별화** |
| 다중 인터페이스 | GUI, TUI, CLI, Headless 모드 | **고유 차별화** |
| TLS 1.0/1.1 지원 | 레거시 클라이언트를 위한 하이브리드 TLS 핸들러 | 대부분의 도구에서 미지원 |

---

## 필요 기능 목록

### 우선순위: 높음

#### 1. Breakpoint (실시간 요청/응답 편집)

Charles Proxy에서 가장 많이 사용되는 기능. 요청이 나가기 전 또는 응답이 돌아올 때 일시 정지하고, GUI/TUI에서 직접 수정 후 전달한다.

**현재 상태:** 인터셉트 규칙은 사전 정의된 규칙만 지원. 실시간 수동 편집 미지원.

**구현 범위:**
- 특정 URL 패턴에 대한 breakpoint 설정
- 요청/응답 일시 정지 및 대기
- GUI/TUI에서 헤더, 바디 직접 편집
- 편집 후 Forward / Drop / Abort 선택
- MCP 도구로도 breakpoint 제어 가능

**참고:** Charles의 Breakpoints, mitmproxy의 `intercept` 명령

---

#### 2. 세션 저장/불러오기

캡처한 트래픽 전체를 프로젝트 파일로 저장하고 나중에 다시 열어 분석하는 기능. QA가 버그 리포트에 트래픽 덤프를 첨부할 때 필수.

**현재 상태:** HAR 내보내기는 있으나, 자체 세션 파일(인터셉트 규칙, 스크립트 설정 포함) 저장/복원 미지원.

**구현 범위:**
- `.cheolsu` 세션 파일 포맷 정의 (트래픽 + 설정 + 규칙 포함)
- 세션 저장 (전체 또는 필터링된 트래픽)
- 세션 불러오기 및 트래픽 뷰어에서 재생
- CLI에서 `--load-session` 옵션
- HAR 파일 가져오기 지원

**참고:** Charles의 Session 저장(.chls), Fiddler의 SAZ 파일

---

#### 3. 모바일 CA 인증서 배포 페이지

모바일 QA에서 가장 큰 진입장벽. 모바일 기기에서 프록시의 CA 인증서를 쉽게 다운로드하고 설치할 수 있어야 한다.

**현재 상태:** CA 인증서 자동 생성은 되지만, 모바일 기기에서의 편리한 배포 수단 미지원.

**구현 범위:**
- 프록시 포트에서 특정 경로(예: `http://cheolsu.proxy/ssl`)로 접속 시 CA 인증서 다운로드 페이지 제공
- iOS용 `.pem`, Android용 `.der` 형식 자동 변환 제공
- 설치 가이드 페이지 (iOS, Android 버전별 단계별 안내)
- QR 코드로 인증서 다운로드 URL 공유

**참고:** Charles의 `chls.pro/ssl`, Proxyman의 인증서 설치 가이드

---

### 우선순위: 중간

#### 4. 요청 타이밍 분석 (Waterfall)

요청별 네트워크 타이밍을 분해하여 병목 지점을 식별하는 기능. 성능 QA에 필수.

**구현 범위:**
- 요청별 타이밍 분해: DNS Lookup, TCP Connect, TLS Handshake, TTFB (Time to First Byte), Content Transfer
- Waterfall 차트 시각화 (GUI)
- 느린 요청 자동 하이라이트 (임계값 설정 가능)
- 통계 요약: 평균/p95/p99 응답 시간, 도메인별 집계

**참고:** Chrome DevTools의 Network 타이밍, Charles의 Timing 탭

---

#### 5. DNS Spoofing / Remote Host Mapping

특정 도메인을 다른 IP로 매핑하는 기능. 스테이징/개발 서버 테스트 시 hosts 파일 수정 없이 사용 가능.

**구현 범위:**
- 도메인 → IP 매핑 규칙 설정
- 도메인 → 도메인 매핑 (포트 포함)
- 와일드카드 패턴 지원 (예: `*.api.example.com → 192.168.1.100`)
- 규칙별 활성/비활성 토글

**참고:** Charles의 DNS Spoofing, Proxyman의 Map Remote

---

#### 6. 트래픽 비교 (Diff)

두 요청/응답 간 차이를 시각적으로 비교하는 기능. 배포 전후 API 응답 변화를 확인할 때 유용.

**구현 범위:**
- 두 트랜잭션 선택 후 diff 뷰 표시
- 헤더/바디 각각 비교
- JSON 구조적 diff (키 순서 무시, 값 변경 하이라이트)
- diff 결과 내보내기

---

#### 7. 클라이언트별 트래픽 분리

여러 기기/사용자의 트래픽을 구분하여 관리하는 기능.

**구현 범위:**
- 클라이언트 IP별 자동 태깅
- 사용자 정의 태그/라벨 부여
- 태그별 필터링
- 프록시 접속 인증 (Basic Auth)

---

### 우선순위: 낮음

#### 8. gRPC / Protobuf 지원

HTTP/2 기반 gRPC 트래픽 캡처 및 Protobuf 메시지 디코딩.

**구현 범위:**
- HTTP/2 프레임 레벨 캡처
- `.proto` 파일 로드를 통한 Protobuf 디코딩
- gRPC 스트리밍 (Unary, Server/Client/Bidirectional) 시각화
- gRPC 메타데이터 표시

---

#### 9. 자동 응답 검증 (Contract Testing)

API 응답이 정의된 스펙과 일치하는지 실시간으로 검증하는 기능.

**구현 범위:**
- OpenAPI/Swagger 스펙 파일 로드
- 실시간 요청/응답을 스펙과 대조
- 불일치 항목 경고 (누락된 필드, 타입 불일치, 예상 외 상태 코드 등)
- 검증 결과 리포트 생성

---

#### 10. 요청/응답 본문 뷰어 강화

다양한 콘텐츠 타입에 대한 풍부한 뷰어 제공.

**구현 범위:**
- JSON 트리 뷰어 (접기/펼치기, 경로 복사)
- 이미지 미리보기 (JPEG, PNG, GIF, WebP, SVG)
- 폼 데이터 파싱 (multipart/form-data, application/x-www-form-urlencoded)
- XML/HTML 구문 강조 및 포맷팅
- Brotli/gzip 자동 디코딩 표시
- 바이너리 데이터 hex 뷰어

---

## 우선순위 요약

| 우선순위 | 기능 | 핵심 이유 |
|---------|------|-----------|
| 높음 | Breakpoint (실시간 편집) | Charles 사용자가 가장 먼저 찾는 기능 |
| 높음 | 세션 저장/불러오기 | QA 워크플로우의 기본 |
| 높음 | 모바일 CA 인증서 배포 페이지 | 모바일 QA 진입장벽 제거 |
| 중간 | 타이밍 분석 (Waterfall) | 성능 QA에 필수 |
| 중간 | DNS Spoofing / Host Mapping | 환경 전환 편의성 |
| 중간 | 트래픽 비교 (Diff) | 회귀 테스트에 유용 |
| 중간 | 클라이언트별 트래픽 분리 | 팀 환경에서 필수 |
| 낮음 | gRPC / Protobuf | 사용하는 기업에선 필수이나 범용적이진 않음 |
| 낮음 | 자동 응답 검증 | 차별화 포인트 |
| 낮음 | 본문 뷰어 강화 | 사용성 개선 |
