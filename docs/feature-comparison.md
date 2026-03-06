# mitmproxy vs cheolsu-proxy 기능 비교

> 최종 업데이트: 2026-03-07
>
> mitmproxy를 레퍼런스로 cheolsu-proxy의 현재 지원 기능과 차이점을 정리한 문서입니다.

## 공통 지원 기능

| 기능                    | mitmproxy | cheolsu-proxy | 비고                                 |
| ----------------------- | :-------: | :-----------: | ------------------------------------ |
| HTTP/HTTPS 인터셉트     |     O     |       O       |                                      |
| TLS 인증서 자동 생성    |     O     |       O       | rcgen / OpenSSL 선택 가능            |
| 요청/응답 상세 보기     |     O     |       O       |                                      |
| 헤더/바디 표시          |     O     |       O       |                                      |
| JSON/XML/HTML 구문 강조 |     O     |       O       | Monaco Editor 기반                   |
| 이미지/미디어 미리보기  |     O     |       O       | 이미지, 비디오, 오디오               |
| 바이너리 파일 감지      |     O     |       O       | 40개+ 파일 시그니처                  |
| 요청 리플레이           |     O     |       O       | 개별 + 시퀀스 리플레이               |
| 요청 차단 (Block)       |     O     |       O       | 상태코드/응답 지정 가능              |
| 헤더/바디 수정 규칙     |     O     |       O       | ModifyRequest / ModifyResponse       |
| 필터링/검색             |     O     |       O       | mitmproxy: DSL, cheolsu: 쿼리 에디터 |
| cURL 복사               |     O     |       O       |                                      |
| gzip/brotli 압축 해제   |     O     |       O       |                                      |
| 시스템 프록시 자동 설정 |     X     |       O       | 앱 시작/종료 시 자동 관리            |
| 대용량 바디 파일 저장   |     X     |       O       | 1MB 이상 자동 파일 저장              |

## mitmproxy에만 있는 기능

| 기능                   | 설명                                  | 우선순위  |
| ---------------------- | ------------------------------------- | :-------: |
| HTTP/2, HTTP/3 (QUIC)  | 다중 프로토콜 지원                    |           |
| ~~WebSocket 뷰~~       | ~~WebSocket 메시지 인터셉트 및 표시~~ | 구현 완료 |
| Transparent Proxy      | OS 레벨 투명 프록시 모드              |           |
| Reverse Proxy          | 특정 서버로 트래픽 전달               |           |
| SOCKS Proxy            | SOCKS5 프록시 모드                    |           |
| Upstream Proxy         | 상위 프록시 체이닝                    |           |
| Map Local              | 요청을 로컬 파일로 대체               |           |
| Map Remote             | URL을 다른 URL로 리다이렉트           |           |
| Anticache              | 캐시 헤더 자동 제거                   |           |
| Anticomp               | 압축 비활성화 (디버깅용)              |           |
| Sticky Auth/Cookie     | 인증/쿠키 자동 재전송                 |           |
| Server-side Replay     | 저장된 응답으로 서버 모킹             |           |
| Save/Load (Flow 파일)  | 세션 저장 및 불러오기                 |           |
| HAR Export             | HAR 형식 내보내기                     |           |
| Python 스크립트 애드온 | 사용자 스크립트로 기능 확장           |           |
| 콘솔 UI (TUI)          | 터미널 인터랙티브 UI                  |           |
| mitmdump (CLI)         | tcpdump 스타일 CLI 도구               |           |
| DNS 인터셉트           | DNS 쿼리/응답 조작                    |           |
| Proxy Authentication   | 프록시 접속 인증                      |           |
| Content View 확장      | GraphQL, MQTT, Socket.IO, WBXML 등    | 부분 구현 |

> 우선순위 컬럼은 cheolsu-proxy에 도입할 기능을 선별할 때 사용합니다.

## cheolsu-proxy에만 있는 기능

| 기능                    | 설명                                                   |
| ----------------------- | ------------------------------------------------------ |
| 네이티브 데스크톱 앱    | Tauri 기반 (경량, 저메모리)                            |
| 시퀀스 리플레이         | 여러 요청을 순차 배치 실행                             |
| 호스트/경로 트리 뷰     | 트래픽을 호스트별 트리 구조로 그룹핑                   |
| 핀/체크박스 선택        | 트랜잭션 고정 및 다중 선택                             |
| 일시정지/재개           | 트래픽 캡처 일시 중지                                  |
| 시스템 프록시 자동 관리 | 앱 시작/종료 시 자동 설정/해제                         |
| 백그라운드 데몬         | UDS 기반 독립 데몬 프로세스                            |
| CLI 인터셉트 셸         | headless 모드에서 규칙 관리 (block, modify-request 등) |

## 아키텍처 차이

| 항목        | mitmproxy                 | cheolsu-proxy            |
| ----------- | ------------------------- | ------------------------ |
| 언어        | Python                    | Rust + TypeScript        |
| UI          | Web (React) + TUI (urwid) | Tauri (네이티브 + React) |
| 프록시 엔진 | 자체 구현                 | hudsucker 기반           |
| 상태 관리   | Redux                     | Zustand                  |
| 에디터      | CodeMirror                | Monaco Editor            |
| 확장성      | Python 스크립트 애드온    | 인터셉트 규칙 (UI/CLI)   |
| 배포        | pip install               | 네이티브 바이너리        |
| 프로세스    | 단일 프로세스             | 데몬 + UI 분리           |
