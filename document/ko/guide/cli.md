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

모든 커맨드는 전역 `--output` 옵션으로 출력 형식을 선택할 수 있습니다(`text` 기본, `json`).

```bash
cheolsu traffic search --host example.com --output json
```

## 서브커맨드

### 트래픽

```bash
# 캡처된 트래픽 검색 (host/method/status/path/limit 필터)
cheolsu traffic search --host example.com --method GET --status 500 --path /api --limit 20

# 트랜잭션 상세 조회
cheolsu traffic get <트랜잭션-id>

# WebSocket 메시지 조회
cheolsu traffic ws --connection-id ws://example.com --limit 50

# SSE 이벤트 조회
cheolsu traffic sse --connection-id https://example.com --limit 50

# 두 트랜잭션 비교
cheolsu traffic diff <id-a> <id-b>

# 캡처된 트래픽 전체 삭제
cheolsu traffic clear

# 트래픽으로부터 OpenAPI 스펙 생성
cheolsu traffic openapi --host api.example.com --path-prefix /v1 --title "My API"
```

### 규칙

`--action-type`에 따라 필요한 옵션이 달라집니다.

```bash
# 인터셉트 규칙 목록
cheolsu rule list

# 차단 규칙 (커스텀 상태코드/바디 선택)
cheolsu rule add --name "광고 차단" --pattern "*.ads.com" --action-type block \
  --status-code 403 --response-body "blocked"

# 요청 수정 규칙 (헤더 추가/제거)
cheolsu rule add --name "인증 추가" --pattern "*api.example.com*" \
  --action-type modify_request --add-header "Authorization=Bearer token123" \
  --remove-header "Cookie"

# 응답 수정 규칙
cheolsu rule add --name "CORS 허용" --pattern "*api.example.com*" \
  --action-type modify_response --add-header "Access-Control-Allow-Origin=*"

# 로컬 매핑 (Map Local) — 응답을 로컬 파일로 대체
cheolsu rule add --name "목 응답" --pattern "*api.example.com/users*" \
  --action-type map_local --file-path ./mock/users.json

# 원격 매핑 (Map Remote)
cheolsu rule add --name "개발 리다이렉트" --pattern "*api.example.com*" \
  --action-type map_remote --target-url "http://localhost:3000" --preserve-path true

# 규칙 제거
cheolsu rule remove <규칙-id>
```

### 브레이크포인트

```bash
# 브레이크포인트 목록
cheolsu breakpoint list

# 브레이크포인트 추가 (요청/응답 단계 선택)
cheolsu breakpoint add --pattern "*api.example.com*" \
  --break-on-request true --break-on-response false

# 대기 중(일시정지)인 브레이크포인트 조회
cheolsu breakpoint pending

# 대기 중 브레이크포인트 처리 (forward / modify_and_forward / drop / abort)
cheolsu breakpoint resolve --id <bp-id> --action modify_and_forward \
  --header "X-Debug=1" --body '{"patched":true}' --status 200

# 브레이크포인트 제거
cheolsu breakpoint remove <bp-id>
```

### 호스트 매핑

```bash
# 호스트 매핑 목록
cheolsu host-mapping list

# 호스트 매핑 추가 (source → target, 포트 선택)
cheolsu host-mapping add --source-host api.example.com \
  --target-host 127.0.0.1 --target-port 3000

# 호스트 매핑 제거
cheolsu host-mapping remove <매핑-id>
```

### 리버스 프록시

```bash
# 리버스 프록시 규칙 목록
cheolsu reverse-proxy list

# 규칙 추가 (Host 패턴 → 백엔드)
cheolsu reverse-proxy add --match-host example.com \
  --backend-scheme http --backend-host 127.0.0.1 --backend-port 8080 \
  --rewrite-host true

# 규칙 제거
cheolsu reverse-proxy remove <규칙-id>
```

### 서버 리플레이

```bash
# 서버 리플레이 항목 목록
cheolsu server-replay list

# 캡처된 트랜잭션을 서버 리플레이로 등록 (매칭 요청에 캐시 응답 반환)
cheolsu server-replay add <트랜잭션-id>

# 항목 제거 / 전체 초기화
cheolsu server-replay remove <항목-id>
cheolsu server-replay clear
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

> `--enabled`, `--no-caching` 등은 **값을 받지 않는 플래그**입니다. 켜려면 플래그를 붙이고, 끄려면 생략하세요(예: `--enabled true`는 오류).

```bash
# 업스트림 프록시 활성화 (인증 선택)
cheolsu settings upstream-proxy --enabled --host proxy.corp.com --port 8080 \
  --username user --password pass

# 업스트림 프록시 비활성화 (플래그 생략)
cheolsu settings upstream-proxy --host proxy.corp.com --port 8080

# 쓰로틀링 설정 (rate 단위: bytes/sec)
cheolsu settings throttle --enabled --download-rate 50000 --upload-rate 20000 --latency-ms 200

# 프록시 인증 설정 (basic / bearer / apikey)
cheolsu settings proxy-auth --enabled --method basic --username user --password pass
cheolsu settings proxy-auth --enabled --method bearer --token "my-token"

# 연결 전략 설정 (lazy / eager / eager_with_fallback)
cheolsu settings connection-strategy eager

# 빠른 설정 (필요한 플래그만 켜기)
cheolsu settings quick --no-caching --block-cookies --no-gzip --block-quic

# 클라이언트 인증서 설정 (mTLS)
cheolsu settings client-certificate --enabled --cert-path ./cert.pem --key-path ./key.pem

# SSL 프록싱 리스트 설정 (mode: blacklist 기본 / whitelist)
cheolsu settings ssl-proxying-list --mode whitelist \
  --entry "api.example.com=true" \
  --entry "api.example.com:8443=true"
```

### 세션

```bash
# 현재 트래픽을 세션 파일로 저장 (.cheolsu.gz로 압축, --filter로 URL 부분 일치 필터)
cheolsu session save ./traffic.cheolsu --name "내 세션" \
  --description "재현 케이스" --filter example.com

# 세션 파일(.cheolsu/.cheolsu.gz) 또는 HAR(.har) 로드
cheolsu session load ./traffic.cheolsu

# 기존 트래픽에 추가로 로드
cheolsu session load ./traffic.cheolsu --append
```

### 스크립트

```bash
# 파일에서 스크립트 로드
cheolsu script load --path ./my-script.js

# 인라인 코드로 로드
cheolsu script load --code "cheolsu.onRequest((req) => ({ action: 'forward' }));"

# 스크립트 언로드
cheolsu script unload
```

### 기타

```bash
# 프록시 상태 확인
cheolsu status

# HAR 파일로 내보내기 (--host / --path 필터)
cheolsu export-har ./output.har --host example.com

# TUI 모드 실행
cheolsu tui --port 8100 --host 0.0.0.0

# Claude Code 스킬 설치 / 설치 상태 확인 / 제거
cheolsu install-skills
cheolsu install-skills --check
cheolsu uninstall-skills

# Shell completion 스크립트 생성 (bash / zsh / fish / powershell)
cheolsu completion zsh > _cheolsu
```

## 종료 코드

| 코드 | 의미                                       |
| ---- | ------------------------------------------ |
| 0    | 성공                                       |
| 1    | 에러 (데몬 미실행, 잘못된 인자, 작업 실패) |

## Claude Code 연동

Cheolsu Proxy는 Claude Code 대화 중에 직접 사용할 수 있는 스킬을 제공합니다:

- `/cheolsu <커맨드>` — CLI 커맨드 실행
- `/dev-url-mapping <도메인> <로컬주소>` — 로컬 개발 URL 매핑 설정
