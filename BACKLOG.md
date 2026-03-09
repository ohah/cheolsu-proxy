# Backlog - 미구현 기능

## ~~1. 트래픽 리플레이 (Traffic Replay)~~ ✅ 완료

캡처된 HTTP 요청을 다시 서버로 전송하는 기능. 디버깅할 때 동일한 요청을 반복 테스트하거나, 특정 파라미터만 바꿔서 재전송할 수 있다. 예를 들어 API 호출이 실패했을 때 헤더나 바디를 수정해서 다시 보내보는 식으로 사용.

- 단일 요청 재전송
- 요청 수정 후 재전송
- 시퀀스 재생 (여러 요청을 순서대로)

---

## ~~2. Map Local / Map Remote~~ ✅ 완료

**Map Local**: 특정 URL의 응답을 로컬 파일로 대체. 예를 들어 `https://api.example.com/data`를 요청하면 실제 서버 대신 로컬의 `test-data.json` 파일 내용을 응답으로 돌려줌. 프론트엔드 개발 시 API 목업이나 특정 시나리오 재현에 유용.

**Map Remote**: 특정 URL의 요청을 다른 URL로 리다이렉트. 예를 들어 프로덕션 API 요청을 로컬 개발 서버로 보내거나, CDN URL을 로컬 에셋 서버로 돌릴 수 있음.

- URL 패턴 매칭 (와일드카드, 정규식)
- 요청/응답 단위 매핑 규칙
- UI에서 규칙 관리

---

## ~~3. 스크립팅/플러그인 (TypeScript)~~ ✅ 완료

사용자가 TypeScript로 프록시 동작을 커스터마이징하는 기능. 요청/응답을 가로채서 동적으로 수정하거나 로깅하는 스크립트를 작성할 수 있음. Python 대신 TypeScript를 채택하여 프론트엔드(Tauri UI)와 언어 통일.

- TypeScript 런타임 내장 (deno_core)
- 요청/응답 훅 (onRequest, onResponse, onWebSocketMessage)
- 스크립트 핫 리로드
- 빌트인 API (헤더 수정, 바디 변환, 로깅 등)
- TUI: Settings 탭에서 스크립트 로드/언로드
- Desktop: Monaco 에디터 내장 (인라인 편집, 파일 불러오기, API Reference, 콘솔 로그, 드래그 리사이즈)
- MCP 서버: load_script / unload_script 도구

---

## ~~4. Protobuf / gRPC 디코딩~~ ✅ 완료

Protobuf 바이너리 데이터를 디코딩하여 필드별로 표시하는 기능. gRPC Content-Type 자동 감지.

- Desktop: Protobuf Preview 뷰어 (`protobuf-preview.tsx`, `protobuf-decoder.ts`)
- Wire type 정보 표시 및 필드별 검사

---

## ~~5. SOCKS 프록시~~ ✅ 완료

SOCKS5 프로토콜 완전 구현. RFC 1929 인증 지원.

- SOCKS5 핸드셰이크 (인증 포함)
- TCP CONNECT 지원
- Upstream proxy SOCKS5 지원
- 테스트 커버리지 포함 (`crates/proxyapi_v2/tests/socks5_tests.rs`)

---

## 6. HTTP/3 (QUIC)

HTTP/3은 TCP 대신 UDP 기반의 QUIC 프로토콜을 사용하는 차세대 HTTP. 연결 설정이 빠르고(0-RTT), 패킷 손실 시 다른 스트림에 영향을 주지 않는 멀티플렉싱이 장점. 현재 Chrome, Safari 등 주요 브라우저와 Cloudflare, Google 등이 지원 중.

- QUIC 프로토콜 파싱
- HTTP/3 요청/응답 가로채기
- QUIC 인증서 처리

---

## 7. 투명 프록시 모드 (Transparent Proxy)

클라이언트가 프록시 설정 없이도 트래픽이 자동으로 프록시를 거치게 하는 모드. OS의 패킷 리다이렉션(macOS: pf, Linux: iptables/nftables)으로 특정 포트의 트래픽을 프록시로 보냄. 프록시 설정을 지원하지 않는 앱의 트래픽도 캡처 가능.

- macOS pf 규칙 자동 설정
- 원본 목적지 주소 복원 (SO_ORIGINAL_DST)
- TLS SNI 기반 호스트 식별

---

## 8. 리버스 프록시 모드 (Reverse Proxy)

일반 프록시(forward proxy)는 클라이언트 앞에 두지만, 리버스 프록시는 서버 앞에 배치. 들어오는 요청을 백엔드 서버로 전달하면서 트래픽을 관찰/수정. 서버 개발 시 외부에서 들어오는 요청을 디버깅하거나 API 게이트웨이 역할을 테스트할 때 사용.

- 특정 포트 리스닝 → 백엔드 서버 전달
- 호스트 헤더 재작성
- 로드밸런싱 (기본 라운드로빈)

---

## ~~9. Upstream Proxy 체이닝~~ ✅ 완료

프록시가 직접 대상 서버에 연결하지 않고, 또 다른 프록시를 경유하는 기능. 회사 네트워크에서 이미 프록시가 있는 환경이나, Tor/VPN 프록시를 거쳐야 할 때 필요. 우리 프록시 → 회사 프록시 → 인터넷 순서로 체이닝.

- HTTP/HTTPS upstream 프록시 설정
- SOCKS upstream 프록시 설정
- 프록시 인증 (Basic)
- TUI: Settings 탭에서 Upstream Proxy 설정 UI
- Desktop: Settings에서 Upstream Proxy 설정

---

## 10. 클라이언트 인증서 검증 (mTLS)

일반 TLS는 서버만 인증서를 제시하지만, mTLS(Mutual TLS)는 클라이언트도 인증서를 제시. 금융/기업 환경에서 클라이언트 신원 확인에 사용됨. MITM 프록시가 중간에서 클라이언트 인증서를 서버에 전달하거나, 직접 검증하는 기능.

- 클라이언트 인증서 감지 및 전달
- 인증서 체인 검증
- UI에서 인증서 정보 표시

---

## 11. WireGuard VPN 모드

우리 앱 자체가 WireGuard VPN 서버 역할을 하여 모든 트래픽을 캡처하는 모드. 별도 서버 불필요 — 같은 와이파이 내에서 내 컴퓨터에 VPN 연결하는 방식. 프록시 설정이 불가능한 앱, 다른 기기(폰/태블릿)의 트래픽까지 캡처 가능. 모든 케이스를 커버하는 가장 범용적인 방식.

- Rust 크레이트: `boringtun` (Cloudflare 프로덕션 사용, 순수 Rust WireGuard 구현)
- TUN 가상 네트워크 디바이스 생성 (`tun` 크레이트)
- IP 패킷 → TCP 스트림 재조립 (`smoltcp` 크레이트)
- 재조립된 스트림을 기존 프록시 파이프라인에 연결
- QR 코드로 기기 설정 공유 (WireGuard 앱에서 스캔)
- DNS 요청 가로채기 (UDP)

### 사용 흐름

1. cheolsu-proxy 실행 → WireGuard 서버 자동 시작
2. UI에서 QR 코드 표시
3. 기기에서 WireGuard 앱으로 QR 스캔
4. 해당 기기의 모든 트래픽 캡처 시작

---

## 우선순위 제안

| 순위   | 기능                       | 이유                                          |
| ------ | -------------------------- | --------------------------------------------- |
| ~~P1~~ | ~~트래픽 리플레이~~        | ✅ 완료                                       |
| ~~P1~~ | ~~Map Local / Map Remote~~ | ✅ 완료                                       |
| ~~P2~~ | ~~스크립팅 (TypeScript)~~  | ✅ 완료                                       |
| ~~P2~~ | ~~Upstream Proxy 체이닝~~  | ✅ 완료                                       |
| ~~P3~~ | ~~Protobuf / gRPC 디코딩~~ | ✅ 완료                                       |
| ~~P3~~ | ~~SOCKS 프록시~~           | ✅ 완료                                       |
| P3     | WireGuard VPN 모드         | 모든 케이스 대응, 다른 기기 캡처              |
| P3     | 투명 프록시 모드           | 프록시 미지원 앱 대응 (WireGuard로 대체 가능) |
| P4     | 클라이언트 인증서 (mTLS)   | 특수 환경                                     |
| P4     | 리버스 프록시 모드         | 서버 디버깅용                                 |
| P5     | HTTP/3 (QUIC)              | 생태계 아직 미성숙, 구현 복잡도 높음          |
