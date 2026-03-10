# TLS 인증서 고도화 전략 비교: cheolsu-proxy vs mitmproxy

> 작성일: 2026-03-11
> 목적: mitmproxy 레퍼런스 분석을 통한 인증서/TLS 전략 개선 방향 도출

---

## 1. 현재 cheolsu-proxy가 우위인 부분

| 항목 | cheolsu-proxy | mitmproxy |
|------|--------------|-----------|
| TLS Passthrough | 실패 횟수 기반 자동 바이패스 | `ignore_connection` 플래그 (수동 설정) |
| TLS 엔진 | rustls + OpenSSL 하이브리드 이중 엔진 | OpenSSL 단일 엔진 |
| Apple 서비스 특수 처리 | 전용 암호화 스위트/타임아웃 적용 | 없음 |
| 인증서 캐시 | moka 기반 (TTL 180일, 용량 무제한) | 인메모리 dict (100개 제한) |

---

## 2. mitmproxy에는 있지만 우리에게 없는 것

### 2.1 Upstream Certificate Sniffing (상류 인증서 스니핑)

mitmproxy의 핵심 전략. `upstream_cert` 옵션 (기본값: True).

**동작 흐름:**

```
[클라이언트] → ClientHello → [프록시] → 상류 서버에 먼저 연결
                                        → 실제 인증서의 CN, SAN, 조직명 복사
                                        → 이 정보로 위조 인증서 생성
                              ← 위조 인증서로 클라이언트 응답
```

**세부 사항:**
- 상류 인증서에서 CN, 모든 SAN(altnames), 조직명을 복사
- CRL Distribution Points도 추출하여 위조 인증서에 포함
- 최종 인증서의 SAN = 상류 인증서 SAN + 클라이언트 SNI + 로컬 소켓 주소 + 서버 주소 (중복 제거)
- `add_upstream_certs_to_client_chain` 옵션: 상류 서버의 전체 인증서 체인을 클라이언트에게 전달 가능

**기대 효과:** 호스트명 기반 인증서 생성보다 위조 인증서 품질이 크게 향상되어 TLS 실패율 감소.

### 2.2 Eager vs Lazy 연결 전략

| 전략 | 설명 | 장점 | 단점 |
|------|------|------|------|
| **Eager** | 클라이언트 핸드셰이크 전에 서버 먼저 연결 | 정확한 인증서 복제 가능 | 지연시간 증가 |
| **Lazy** | 클라이언트 SNI/ALPN 확보 후 서버 연결 | 빠른 응답 | 인증서 정보 제한적 |

- Eager: `establish_server_tls_first = True` → 상류 인증서를 먼저 가져와서 위조 인증서 생성
- Lazy: `wait_for_clienthello = True` → 클라이언트의 ClientHello에서 SNI/ALPN 정보를 먼저 확보

### 2.3 ALPN 미러링 (5단계 우선순위)

mitmproxy의 ALPN 프로토콜 선택 로직:

1. 클라이언트가 ALPN 전송 시 → 서버 옵션과 호환되면 사용
2. 서버가 ALPN 광고 시 → 서버 옵션에 있으면 반환
3. 서버가 명시적 거부 시 → 클라이언트에게도 거부 미러링
4. h2 비활성 시 → HTTP/1만 사용, 클라이언트 선호 순서 존중
5. 매칭 실패 시 → `NO_OVERLAPPING_PROTOCOLS` 반환

### 2.4 mTLS (상호 인증서) 지원

- `client_certs`: 프록시 → 서버 간 클라이언트 인증서 전달 (파일 또는 디렉토리)
- `request_client_cert`: 클라이언트 → 프록시 간 mTLS 요청

### 2.5 도메인별 커스텀 인증서

- `--certs [domain=]path`로 특정 도메인에 사용자 제공 리프 인증서 사용 가능
- 와일드카드 패턴 매칭 지원
- 기업 내부 서버 등 특수 인증서가 필요한 환경에서 유용

### 2.6 TLS 이벤트 훅 아키텍처

mitmproxy는 7개의 TLS 이벤트 훅을 제공:

| 훅 | 설명 |
|---|---|
| `tls_clienthello` | ClientHello 수신 시. `ignore_connection`으로 패스스루, `establish_server_tls_first`로 서버 우선 연결 결정 |
| `tls_start_client` | 클라이언트 TLS 협상 시작 전 |
| `tls_start_server` | 서버 TLS 협상 시작 전 |
| `tls_established_client` | 클라이언트 핸드셰이크 성공 |
| `tls_established_server` | 서버 핸드셰이크 성공 |
| `tls_failed_client` | 클라이언트 핸드셰이크 실패 |
| `tls_failed_server` | 서버 핸드셰이크 실패 |

**ClientHelloData 속성:** `sni`, `cipher_suites`, `alpn_protocols`, `extensions` (type + raw_bytes)

### 2.7 TLS 버전 및 암호화 스위트 세분화

- 클라이언트/서버 방향별 TLS 버전 범위 독립 설정 (`tls_version_client_min/max`, `tls_version_server_min/max`)
- 클라이언트/서버 방향별 암호화 스위트 독립 설정 (`ciphers_client`, `ciphers_server`)
- 기본 27개 암호 스위트 (Mozilla 설정 가이드라인 기반, GCM과 ChaCha20 우선)
- ECDH 곡선 독립 설정 (`tls_ecdh_curve_client/server`)

### 2.8 인증서 생성 세부 사항

| 항목 | mitmproxy | 비고 |
|------|-----------|------|
| CA 유효기간 | -2일 ~ +10년 | 시계 오차 대비 -2일 |
| 리프 유효기간 | -2일 ~ +365일 | |
| CN 길이 제한 | 64자 미만 | RFC 준수 |
| SAN critical 설정 | subject 비어있을 때 critical | RFC 5280 4.2.1.6 |
| Authority Key Identifier | 포함 | |
| Subject Key Identifier | 의도적 생략 | SChannel 호환성 (#6494) |
| 파일 권한 | `umask_secret()` (모드 0o77) | 보안 |

---

## 3. 양쪽 모두 미지원인 고급 영역

| 기능 | 설명 | 필요 기술 |
|------|------|-----------|
| TLS 지문 위장 (JA3/JA4 스푸핑) | ClientHello를 브라우저처럼 위장 | uTLS 등 별도 구현 |
| ECH (Encrypted Client Hello) | 암호화된 SNI 처리 | 표준 아직 발전 중 |
| 인증서 피닝 바이패스 | 앱 레벨 피닝 우회 | 외부 도구 의존 (`apk-mitm`, `objection` 등) |

---

## 4. 구현 우선순위 제안

도입 효과가 가장 큰 순서로 정리:

### P0: Upstream Certificate Sniffing
- **효과:** 위조 인증서 품질 향상 → TLS 실패율 감소 → passthrough 전환 감소
- **난이도:** 중간 (서버 먼저 연결 → 인증서 파싱 → SAN/CN 복사)
- **구현 포인트:** `dummy_cert()` 생성 시 상류 인증서 정보 활용

### P1: Eager/Lazy 연결 전략
- **효과:** P0과 시너지. 상황에 따라 최적 전략 선택 가능
- **난이도:** 중간~높음 (연결 타이밍 제어 로직 변경)
- **구현 포인트:** ClientHello 수신 후 서버 연결 타이밍 분기

### P2: ALPN 미러링 고도화
- **효과:** HTTP/2 환경에서 호환성 향상
- **난이도:** 낮음
- **구현 포인트:** 클라이언트/서버 ALPN 협상 중계 로직 추가

### P3: 도메인별 커스텀 인증서
- **효과:** 기업 내부 서버, 특수 인증서 환경 대응
- **난이도:** 낮음
- **구현 포인트:** 설정 파일에서 도메인-인증서 매핑 로드

### P4: mTLS 지원
- **효과:** 클라이언트 인증서가 필요한 서버 환경 대응
- **난이도:** 중간

---

## 5. 참고 자료

- [mitmproxy 인증서 문서](https://docs.mitmproxy.org/stable/concepts-certificates/)
- [mitmproxy TLS 동작 원리](https://docs.mitmproxy.org/stable/concepts-howmitmproxyworks/)
- mitmproxy 소스: `net/tls.py`, `certs.py`, `tls.py` (레이어)
