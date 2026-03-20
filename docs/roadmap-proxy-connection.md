# 프록시 연결 기능 로드맵

> 프록시 서버 연결과 관련하여 추가 지원 가능한 기능을 우선순위별로 정리한 문서입니다.

## 현재 구현된 연결 관련 기능

| 기능                   | 설명                                                         | 비교                   |
| ---------------------- | ------------------------------------------------------------ | ---------------------- |
| HTTP/HTTPS 프록시      | MITM 기반 트래픽 가로채기                                    | Charles/mitmproxy 동등 |
| SOCKS5 프록시          | RFC 1929 인증 포함 완전 구현                                 | Charles 동등           |
| Upstream Proxy         | HTTP/HTTPS/SOCKS upstream 지원, 인증 및 바이패스             | Charles 동등           |
| TLS 1.0/1.1 레거시     | OpenSSL/rustls 하이브리드 핸들러                             | **고유 차별화**        |
| WebSocket 캡처         | 양방향 메시지 모니터링/주입, Socket.IO/MQTT 감지             | Charles보다 우위       |
| SSE 스트리밍 캡처      | `text/event-stream` 자동 감지, 이벤트 파싱, 스크립팅 훅 지원 | **고유 차별화**        |
| gRPC 트래픽 디코딩     | gRPC 프레임 파싱, 메타데이터 추출, Protobuf 디코딩           | mitmproxy 동등         |
| 연결 상태 모니터링     | 글로벌/도메인별 메트릭, 에러 추적, 실시간 집계               | Charles보다 우위       |
| 연결 전략 (Eager/Lazy) | ClientHello 분석 후 백그라운드 서버 연결                     | **고유 차별화**        |
| 네트워크 스로틀링      | Token Bucket 기반, GPRS~WiFi 프리셋                          | Charles 동등           |
| 동시 연결 제한         | Semaphore 기반 최대 연결 수 제어                             | 기본 기능              |

---

## Tier 1 — 즉시 구현 (기업 환경 지원 강화)

### 1. 프록시 체이닝 (Multi-hop Proxy)

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
        {
          "type": "socks5",
          "host": "proxy2.corp.com",
          "port": 1080,
          "auth": { "username": "user", "password": "pass" }
        }
      ]
    }
  ]
}
```

---

### 2. PAC (Proxy Auto-Configuration) 지원

PAC 파일을 파싱/실행하여 조건별로 자동 프록시를 선택합니다. 기업 환경에서 시스템 PAC 설정과 연동할 수 있습니다.

**구현 범위:**

- PAC 파일 로드 (로컬 파일 / URL)
- `FindProxyForURL(url, host)` JavaScript 함수 실행 (Deno Core 활용)
- PAC 결과에 따른 프록시 라우팅 (`DIRECT`, `PROXY`, `SOCKS`)
- 시스템 PAC 설정 자동 감지 (macOS, Windows)
- PAC 테스트 도구 (URL 입력 → 어떤 프록시를 선택하는지 확인)

---

### 3. TCP Keep-Alive / 커넥션 풀 튜닝

서버측 연결의 Keep-Alive 및 커넥션 풀 동작을 세분화 설정합니다.

**구현 범위:**

- 글로벌 / 도메인별 설정 가능
- 커넥션 풀 크기 (최대 idle 연결 수)
- idle 타임아웃 (미사용 연결 자동 종료)
- Keep-Alive 간격 및 프로브 횟수
- 연결 최대 수명 (max lifetime)
- DNS TTL 존중 여부

---

## Tier 2 — 장기 (차별화 / 완성도)

### 4. DNS-over-HTTPS (DoH) / DNS-over-TLS (DoT)

프록시 레벨에서 DNS 쿼리를 암호화하여 전송합니다.

**구현 범위:**

- DoH 리졸버 (Cloudflare, Google 등 선택 가능)
- DNS 캐싱 (TTL 기반)
- 도메인별 DNS 서버 지정
- DNS 쿼리 로깅 (어떤 도메인을 조회했는지 기록)

---

### 5. HTTP/2 Multiplexing 최적화

동일 호스트에 대한 HTTP/2 스트림 멀티플렉싱을 최적화합니다.

**구현 범위:**

- 호스트별 HTTP/2 연결 풀링
- 스트림 우선순위 설정
- h2c (cleartext HTTP/2) 지원
- HTTP/2 → HTTP/1.1 폴백 자동 감지

---

### 6. Happy Eyeballs (IPv6 듀얼 스택)

IPv4/IPv6 동시 연결 시도 후 빠른 쪽을 선택하는 알고리즘을 구현합니다.

**구현 범위:**

- RFC 8305 Happy Eyeballs v2 구현
- IPv6 리스닝 지원
- 도메인별 IPv4/IPv6 선호도 설정
- 연결 지연 통계 수집

---

### ~~7. 리버스 프록시 모드~~ ✅

~~특정 백엔드 서버 앞에서 리버스 프록시로 동작합니다. API 개발 시 클라이언트 설정 없이 트래픽을 분석할 수 있습니다.~~

구현 완료. Host 헤더 기반 백엔드 라우팅, 가상 호스트 패턴 매칭, Host 헤더 재작성 지원. 자세한 내용은 [Reverse Proxy](./reverse-proxy.md) 문서를 참고하세요.

---

## 우선순위 요약

| 우선순위     | 기능                   | 구현 난이도 | 사용자 영향 | 상태         |
| ------------ | ---------------------- | ----------- | ----------- | ------------ |
| **Tier 1-1** | 프록시 체이닝          | 높음        | 중간        | 📋 계획      |
| **Tier 1-2** | PAC 파일 지원          | 중          | 중간        | 📋 계획      |
| **Tier 1-3** | 커넥션 풀 튜닝         | 낮음        | 낮음        | 📋 계획      |
| Tier 2-1     | DNS-over-HTTPS         | 중          | 낮음        | 📋 계획      |
| Tier 2-2     | HTTP/2 최적화          | 높음        | 낮음        | 📋 계획      |
| Tier 2-3     | Happy Eyeballs         | 중          | 낮음        | 📋 계획      |
| ~~Tier 2-4~~ | ~~리버스 프록시 모드~~ | ~~높음~~    | ~~낮음~~    | ✅ 구현 완료 |
