# gRPC / Protobuf

## 개요

Cheolsu Proxy는 gRPC 트래픽을 자동으로 감지하고, Protobuf 메시지를 디코딩하여 사람이 읽을 수 있는 형태로 표시합니다. `.proto` 정의 파일 없이도 Wire Type 기반으로 메시지를 자동 디코딩합니다.

---

## 자동 감지

다음 조건으로 gRPC 트래픽을 자동 감지합니다:

- Content-Type이 `application/grpc` 또는 `application/grpc+proto`로 시작
- Content-Type 서브타입 추출 (예: `proto`, `json`)

---

## 분석 정보

### 메타데이터

| 항목              | 설명                                                      |
| ----------------- | --------------------------------------------------------- |
| **서비스 이름**   | URI 경로에서 추출 (예: `/package.ServiceName/MethodName`) |
| **메서드 이름**   | 호출된 RPC 메서드                                         |
| **상태 코드**     | gRPC 상태 코드 (OK, CANCELLED, UNKNOWN 등 16개)           |
| **상태 메시지**   | gRPC 상태 메시지 (percent-encoding 자동 디코딩)           |
| **스트리밍 타입** | Unary, Server Streaming, Client Streaming, Bidirectional  |

### gRPC 상태 코드

| 코드 | 이름              | 설명             |
| ---- | ----------------- | ---------------- |
| 0    | OK                | 성공             |
| 1    | CANCELLED         | 요청 취소        |
| 2    | UNKNOWN           | 알 수 없는 오류  |
| 3    | INVALID_ARGUMENT  | 잘못된 인수      |
| 4    | DEADLINE_EXCEEDED | 타임아웃         |
| 5    | NOT_FOUND         | 리소스 없음      |
| 7    | PERMISSION_DENIED | 권한 거부        |
| 13   | INTERNAL          | 서버 내부 오류   |
| 14   | UNAVAILABLE       | 서비스 사용 불가 |
| 16   | UNAUTHENTICATED   | 인증 실패        |

### Protobuf 프레임 파싱

gRPC 바디는 Length-Prefixed Message 형식으로 구성됩니다:

```
[Compressed-Flag (1 byte)] [Message-Length (4 bytes, Big Endian)] [Message]
```

각 프레임의 압축 여부와 메시지 데이터를 분리하여 표시합니다. Wire Type 기반으로 필드를 자동 디코딩하여 `.proto` 파일 없이도 메시지 구조를 확인할 수 있습니다.

---

## 활용 사례

### 마이크로서비스 디버깅

gRPC 기반 마이크로서비스 간 통신을 모니터링하고, 서비스/메서드별로 요청을 필터링하여 특정 RPC 호출을 추적할 수 있습니다.

### 에러 진단

gRPC 상태 코드와 상태 메시지를 확인하여 요청 실패 원인을 파악합니다. DEADLINE_EXCEEDED, UNAVAILABLE 등의 상태를 추적하여 네트워크 이슈나 서비스 장애를 진단할 수 있습니다.

### 페이로드 검사

Protobuf 메시지의 필드 구조를 확인하여 요청/응답 데이터가 예상대로 직렬화되었는지 검증합니다.

---

## 사용 방법

### Desktop

트래픽 로그에서 gRPC 요청을 선택하면 트랜잭션 상세 보기에서 서비스/메서드 정보, 상태 코드, 디코딩된 Protobuf 메시지를 확인할 수 있습니다.

### MCP

```
"gRPC 트래픽을 보여줘"
"DEADLINE_EXCEEDED 상태의 gRPC 요청을 찾아줘"
```
