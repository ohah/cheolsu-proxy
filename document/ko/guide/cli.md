# CLI

Cheolsu Proxy는 MCP 서버와 동일한 기능을 제공하는 CLI(명령줄 인터페이스)를 제공합니다. 스크립팅, 자동화, MCP를 사용할 수 없는 환경에서 유용합니다.

## 설치

```bash
# 소스에서 빌드
cargo build -p cheolsu-cli --release

# 바이너리 위치: target/release/cheolsu
```

## 사용법

```bash
cheolsu <서브커맨드> [옵션]
```

## 서브커맨드

### 트래픽

```bash
# 캡처된 트래픽 검색
cheolsu traffic search --host example.com --method GET --limit 20

# 트랜잭션 상세 조회
cheolsu traffic get <트랜잭션-id>

# WebSocket 메시지 조회
cheolsu traffic ws --connection-id ws://example.com --limit 50

# SSE 이벤트 조회
cheolsu traffic sse --limit 50

# 두 트랜잭션 비교
cheolsu traffic diff <id-a> <id-b>

# 캡처된 트래픽 전체 삭제
cheolsu traffic clear

# 트래픽으로부터 OpenAPI 스펙 생성
cheolsu traffic openapi --host api.example.com --title "My API"
```

### 규칙

```bash
# 인터셉트 규칙 목록
cheolsu rule list

# 차단 규칙 추가
cheolsu rule add --name "광고 차단" --pattern "*.ads.com" --action-type block

# 원격 매핑 규칙 추가
cheolsu rule add --name "개발 리다이렉트" --pattern "*api.example.com*" \
  --action-type map_remote --target-url "http://localhost:3000" --preserve-path true

# 요청 수정 규칙 추가
cheolsu rule add --name "인증 추가" --pattern "*api.example.com*" \
  --action-type modify_request --add-header "Authorization=Bearer token123"

# 규칙 제거
cheolsu rule remove <규칙-id>
```

### 분석

```bash
# 느린 요청 분석
cheolsu analyze performance --threshold-ms 500 --limit 10

# 에러 분석
cheolsu analyze errors

# 엔드포인트별 통계
cheolsu analyze endpoints --sort-by duration --limit 20

# 중복 요청 탐지
cheolsu analyze duplicates --window-ms 5000

# N+1 쿼리 패턴 탐지
cheolsu analyze n-plus1

# 트래픽 타임라인
cheolsu analyze timeline --bucket-seconds 30

# 전체 분석
cheolsu analyze full
```

### 리플레이

```bash
# HTTP 요청 직접 전송 (데몬 불필요)
cheolsu replay request --method GET --url https://httpbin.org/get

# 헤더와 바디 포함 전송
cheolsu replay request --method POST --url https://httpbin.org/post \
  --header "Content-Type=application/json" --body '{"key":"value"}'

# 캡처된 트랜잭션 순차 재전송
cheolsu replay sequence <txn-id-1> <txn-id-2> --delay-ms 100

# 동시 요청 부하 테스트
cheolsu replay repeat --method GET --url https://example.com \
  --iterations 100 --concurrency 10
```

### 설정

```bash
# 업스트림 프록시 설정
cheolsu settings upstream-proxy --enabled true --host proxy.corp.com --port 8080

# 쓰로틀링 설정
cheolsu settings throttle --enabled true --download-rate 50000 --latency-ms 200

# 프록시 인증 설정
cheolsu settings proxy-auth --enabled true --method basic --username user --password pass

# 연결 전략 설정
cheolsu settings connection-strategy eager

# 빠른 설정
cheolsu settings quick --no-caching true --block-cookies true

# 클라이언트 인증서 설정 (mTLS)
cheolsu settings client-certificate --enabled true --cert-path ./cert.pem --key-path ./key.pem

# SSL 프록싱 리스트 설정
cheolsu settings ssl-proxying-list --mode whitelist \
  --entry "api.example.com=true" \
  --entry "api.example.com:8443=true"
```

### 세션

```bash
# 현재 트래픽을 세션 파일로 저장
cheolsu session save ./traffic.cheolsu --name "내 세션"

# 세션 파일 로드
cheolsu session load ./traffic.cheolsu

# 기존 트래픽에 추가로 로드
cheolsu session load ./traffic.cheolsu --append
```

### 기타

```bash
# 프록시 상태 확인
cheolsu status

# HAR 파일로 내보내기
cheolsu export-har ./output.har --host example.com

# TUI 모드 실행
cheolsu tui --port 8100

# 스크립트 관리
cheolsu script load --path ./my-script.js
cheolsu script unload
```

## 종료 코드

| 코드 | 의미 |
|------|------|
| 0 | 성공 |
| 1 | 에러 (데몬 미실행, 잘못된 인자, 작업 실패) |

## Claude Code 연동

Cheolsu Proxy는 Claude Code 대화 중에 직접 사용할 수 있는 스킬을 제공합니다:

- `/cheolsu <커맨드>` — CLI 커맨드 실행
- `/dev-url-mapping <도메인> <로컬주소>` — 로컬 개발 URL 매핑 설정
