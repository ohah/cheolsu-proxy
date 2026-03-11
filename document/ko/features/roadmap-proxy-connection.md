# 프록시 연결 기능 로드맵

> 프록시 서버 연결과 관련하여 추가 지원 가능한 기능을 우선순위별로 정리한 문서입니다.

## 현재 구현된 연결 관련 기능

| 기능 | 설명 | 비교 |
| --- | --- | --- |
| HTTP/HTTPS 프록시 | MITM 기반 트래픽 가로채기 | Charles/mitmproxy 동등 |
| SOCKS5 프록시 | RFC 1929 인증 포함 완전 구현 | Charles 동등 |
| Upstream Proxy | HTTP/HTTPS/SOCKS upstream 지원, 인증 및 바이패스 | Charles 동등 |
| TLS 1.0/1.1 레거시 | OpenSSL/rustls 하이브리드 핸들러 | **고유 차별화** |
| WebSocket 캡처 | 양방향 메시지 모니터링/주입, Socket.IO/MQTT 감지 | Charles보다 우위 |
| 연결 전략 (Eager/Lazy) | ClientHello 분석 후 백그라운드 서버 연결 | **고유 차별화** |
| 네트워크 스로틀링 | Token Bucket 기반, GPRS~WiFi 프리셋 | Charles 동등 |
| 동시 연결 제한 | Semaphore 기반 최대 연결 수 제어 | 기본 기능 |

---

## Tier 1 — 즉시 구현 (높은 수요 + 기존 인프라 활용)

### 1. SSE (Server-Sent Events) 스트리밍 캡처

LLM API(Claude, ChatGPT, Gemini 등)를 포함한 대부분의 AI 스트리밍 API가 SSE를 사용합니다. AI 시대에 프록시 디버깅 도구로서 핵심 기능입니다.

**배경:**

SSE는 HTTP 응답으로 `text/event-stream` Content-Type을 사용하여 서버에서 클라이언트로 이벤트를 스트리밍하는 단방향 프로토콜입니다. WebSocket과 달리 일반 HTTP 위에서 동작하므로 프록시에서 가로채기는 쉽지만, 이벤트 단위로 파싱하여 실시간으로 표시하는 기능은 별도로 구현해야 합니다.

**구현 범위:**

- `text/event-stream` Content-Type 자동 감지
- SSE 이벤트 실시간 파싱 (`event`, `data`, `id`, `retry` 필드)
- 이벤트별 시간순 목록 표시 (WebSocket 메시지 뷰와 유사한 UX)
- JSON `data` 필드 자동 포맷팅 (LLM 스트리밍 응답 디코딩)
- 이벤트 타입별 필터링
- 스크립팅 훅: `cheolsu.onSSEMessage` (이벤트 수정/차단)
- SSE 연결 목록 관리 (활성/종료 상태 구분)
- 스트리밍 중 이벤트 카운트 실시간 업데이트

**사용 시나리오:**

```
# Claude API 스트리밍 응답 디버깅
POST https://api.anthropic.com/v1/messages
Content-Type: application/json
→ Response: text/event-stream

event: content_block_delta
data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}

event: message_stop
data: {"type":"message_stop"}
```

**참고:** WebSocket 캡처 인프라(WebSocketRegistry, 메시지 뷰어)를 재활용하여 구현 비용을 최소화합니다.

---

### 2. gRPC 트래픽 디코딩

마이크로서비스 아키텍처에서 gRPC는 사실상의 표준 통신 프로토콜입니다. 기존의 HTTP/2 feature와 Protobuf 디코더를 조합하여 gRPC 트래픽을 구조화된 형태로 표시합니다.

**배경:**

gRPC는 HTTP/2 위에서 동작하며, Protocol Buffers를 직렬화 형식으로 사용합니다. 현재 Cheolsu Proxy는 HTTP/2 feature(`http2`)와 Protobuf wire-type 기반 디코더를 이미 갖추고 있으므로, 이를 gRPC 문맥에 맞게 조합하는 것이 핵심입니다.

**구현 범위:**

- `application/grpc`, `application/grpc+proto`, `application/grpc-web` Content-Type 감지
- gRPC 프레임 파싱 (Compressed-Flag + Message-Length + Message 구조)
- gRPC 메타데이터 표시 (서비스명, 메서드명, 상태 코드)
- gRPC 상태 코드 매핑 (`grpc-status` 헤더 → 사람이 읽을 수 있는 이름)
- Unary / Server Streaming / Client Streaming / Bidirectional Streaming 구분
- Protobuf 메시지 자동 디코딩 (기존 디코더 활용)
- gRPC-Web 지원 (브라우저 기반 gRPC 클라이언트)
- `.proto` 파일 로드 시 필드명 매핑 (선택적 기능)

**gRPC 상태 코드:**

| 코드 | 이름 | 설명 |
| --- | --- | --- |
| 0 | OK | 성공 |
| 1 | CANCELLED | 클라이언트가 취소 |
| 2 | UNKNOWN | 알 수 없는 오류 |
| 3 | INVALID_ARGUMENT | 잘못된 인자 |
| 4 | DEADLINE_EXCEEDED | 타임아웃 |
| 5 | NOT_FOUND | 리소스 없음 |
| 12 | UNIMPLEMENTED | 미구현 메서드 |
| 13 | INTERNAL | 내부 오류 |
| 14 | UNAVAILABLE | 서비스 불가 |

**사용 시나리오:**

```
# gRPC Unary 호출
POST /grpc.health.v1.Health/Check HTTP/2
Content-Type: application/grpc

# 디코딩된 표시:
Service: grpc.health.v1.Health
Method: Check
Status: 0 (OK)
Request:  { 1: "my-service" }
Response: { 1: 1 }  # SERVING

# .proto 파일 로드 시:
Request:  { service: "my-service" }
Response: { status: SERVING }
```

---

### 3. 연결 상태 모니터링 / 통계

프록시를 통과하는 네트워크 연결의 실시간 상태와 통계를 제공합니다. 디버깅 도구로서 "지금 무슨 일이 일어나고 있는가"에 대한 가시성을 확보합니다.

**구현 범위:**

#### 실시간 메트릭

- 활성 연결 수 (HTTP, WebSocket, SSE, 터널)
- 연결 풀 상태 (idle / in-use / waiting)
- TLS 핸드셰이크 시간 분포
- 초당 요청/응답 처리량 (RPS)
- 전송 바이트 수 (업로드/다운로드)

#### 도메인별 통계

- 도메인별 요청 수, 평균 응답 시간, 에러율
- 도메인별 연결 재사용율 (Keep-Alive 효율)
- 가장 느린 도메인 Top N
- 가장 많은 트래픽을 생성하는 도메인 Top N

#### 에러 추적

- 연결 실패 (TCP, TLS 핸드셰이크, 타임아웃)
- TLS 인증서 오류 상세 (만료, 호스트 불일치 등)
- upstream proxy 연결 실패
- 클라이언트/서버 연결 끊김

#### 시계열 차트

- 시간대별 트래픽 추이 (요청 수, 바이트 수)
- 응답 시간 추이 (평균, p95, p99)
- 에러율 추이

**데이터 수집 포인트:**

```rust
// ProxyContext에 메트릭 수집기 추가
pub struct ConnectionMetrics {
    pub active_connections: AtomicU64,
    pub total_requests: AtomicU64,
    pub total_bytes_sent: AtomicU64,
    pub total_bytes_received: AtomicU64,
    pub tls_handshake_duration: Histogram,
    pub response_duration: Histogram,
    pub domain_stats: DashMap<String, DomainMetrics>,
    pub error_counts: DashMap<ErrorCategory, AtomicU64>,
}
```

**인터페이스:**

- **GUI**: 대시보드 탭 (실시간 차트 + 요약 테이블)
- **TUI**: 상단 상태 바 + 통계 탭
- **MCP**: AI 어시스턴트에게 "가장 느린 API는?" 같은 질의 가능
- **CLI**: `--stats` 플래그로 주기적 출력

---

## Tier 2 — 중기 (기업 환경 지원 강화)

### 4. 프록시 체이닝 (Multi-hop Proxy)

여러 프록시를 순차적으로 경유하는 체인 구성. 기업 네트워크에서 내부 프록시 → 외부 프록시 → 인터넷 구조를 지원합니다.

**구현 범위:**

- 프록시 체인 정의 (순서가 있는 프록시 목록)
- 도메인/규칙별 다른 체인 적용
- 체인 내 프록시 상태 모니터링 (연결 가능 여부)
- 프로토콜 혼합 지원 (HTTP → SOCKS5 → HTTP 등)
- 기존 UpstreamProxyConfig 구조 확장

**설정 예시:**

```json
{
  "proxy_chains": [
    {
      "name": "corporate",
      "match": ["*.internal.company.com"],
      "chain": [
        { "type": "http", "host": "proxy1.corp.com", "port": 8080 },
        { "type": "socks5", "host": "proxy2.corp.com", "port": 1080, "auth": { "username": "user", "password": "pass" } }
      ]
    }
  ]
}
```

---

### 5. PAC (Proxy Auto-Configuration) 지원

PAC 파일을 파싱/실행하여 조건별로 자동 프록시를 선택합니다. 기업 환경에서 시스템 PAC 설정과 연동할 수 있습니다.

**구현 범위:**

- PAC 파일 로드 (로컬 파일 / URL)
- `FindProxyForURL(url, host)` JavaScript 함수 실행 (Deno Core 활용)
- PAC 결과에 따른 프록시 라우팅 (`DIRECT`, `PROXY`, `SOCKS`)
- 시스템 PAC 설정 자동 감지 (macOS, Windows)
- PAC 테스트 도구 (URL 입력 → 어떤 프록시를 선택하는지 확인)

---

### 6. TCP Keep-Alive / 커넥션 풀 튜닝

서버측 연결의 Keep-Alive 및 커넥션 풀 동작을 세분화 설정합니다.

**구현 범위:**

- 글로벌 / 도메인별 설정 가능
- 커넥션 풀 크기 (최대 idle 연결 수)
- idle 타임아웃 (미사용 연결 자동 종료)
- Keep-Alive 간격 및 프로브 횟수
- 연결 최대 수명 (max lifetime)
- DNS TTL 존중 여부

---

## Tier 3 — 장기 (차별화 / 완성도)

### 7. DNS-over-HTTPS (DoH) / DNS-over-TLS (DoT)

프록시 레벨에서 DNS 쿼리를 암호화하여 전송합니다.

**구현 범위:**

- DoH 리졸버 (Cloudflare, Google 등 선택 가능)
- DNS 캐싱 (TTL 기반)
- 도메인별 DNS 서버 지정
- DNS 쿼리 로깅 (어떤 도메인을 조회했는지 기록)

---

### 8. HTTP/2 Multiplexing 최적화

동일 호스트에 대한 HTTP/2 스트림 멀티플렉싱을 최적화합니다.

**구현 범위:**

- 호스트별 HTTP/2 연결 풀링
- 스트림 우선순위 설정
- h2c (cleartext HTTP/2) 지원
- HTTP/2 → HTTP/1.1 폴백 자동 감지

---

### 9. Happy Eyeballs (IPv6 듀얼 스택)

IPv4/IPv6 동시 연결 시도 후 빠른 쪽을 선택하는 알고리즘을 구현합니다.

**구현 범위:**

- RFC 8305 Happy Eyeballs v2 구현
- IPv6 리스닝 지원
- 도메인별 IPv4/IPv6 선호도 설정
- 연결 지연 통계 수집

---

### 10. 리버스 프록시 모드

특정 백엔드 서버 앞에서 리버스 프록시로 동작합니다. API 개발 시 클라이언트 설정 없이 트래픽을 분석할 수 있습니다.

**구현 범위:**

- 리스닝 포트 → 백엔드 서버 매핑
- 가상 호스트 기반 라우팅
- 요청/응답 수정 (헤더 추가/삭제)
- 로드 밸런싱 (라운드 로빈, 가중치)

---

## 우선순위 요약

| 우선순위 | 기능 | 구현 난이도 | 사용자 영향 | 상태 |
| --- | --- | --- | --- | --- |
| **Tier 1-1** | SSE 스트리밍 캡처 | 중 | 매우 높음 | 📋 계획 |
| **Tier 1-2** | gRPC 트래픽 디코딩 | 중 | 높음 | 📋 계획 |
| **Tier 1-3** | 연결 상태 모니터링/통계 | 중 | 높음 | 📋 계획 |
| Tier 2-1 | 프록시 체이닝 | 높음 | 중간 | 📋 계획 |
| Tier 2-2 | PAC 파일 지원 | 중 | 중간 | 📋 계획 |
| Tier 2-3 | 커넥션 풀 튜닝 | 낮음 | 낮음 | 📋 계획 |
| Tier 3-1 | DNS-over-HTTPS | 중 | 낮음 | 📋 계획 |
| Tier 3-2 | HTTP/2 최적화 | 높음 | 낮음 | 📋 계획 |
| Tier 3-3 | Happy Eyeballs | 중 | 낮음 | 📋 계획 |
| Tier 3-4 | 리버스 프록시 모드 | 높음 | 낮음 | 📋 계획 |
