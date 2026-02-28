# Cheolsu-Query

Cheolsu Proxy에서 네트워크 요청을 필터링하기 위한 전용 쿼리 언어입니다.

## 🎯 개요

Cheolsu-Query는 Monaco Editor 기반의 강력한 쿼리 언어로, 네트워크 트래픽을 효율적으로 필터링하고 분석할 수 있도록 설계되었습니다.

### 주요 특징

- **Monaco Editor 기반**: VS Code와 동일한 에디터 엔진 사용
- **구문 강조**: 키워드, 연산자, 문자열 등에 색상 구분
- **자동완성**: 키워드, HTTP 메서드, 상태 코드 자동 제안
- **실시간 검증**: 쿼리 문법 실시간 검증
- **테마 지원**: 라이트/다크 테마 자동 전환

## 🔧 필터 키워드

| 키워드    | 설명                    | 예시                 |
| --------- | ----------------------- | -------------------- |
| `method`  | HTTP 메서드 필터링      | `method="GET"`       |
| `methods` | 여러 HTTP 메서드 필터링 | `methods="GET,POST"` |
| `status`  | HTTP 상태 코드 필터링   | `status="2xx,404"`   |
| `url`     | URL 패턴 필터링         | `url \|= "api"`      |

## ⚙️ 연산자

### 비교 연산자

| 연산자 | 설명                 | 예시                |
| ------ | -------------------- | ------------------- |
| `=`    | 정확히 일치          | `method="GET"`      |
| `\|=`  | 포함 (대소문자 구분) | `url \|= "api"`     |
| `\|~`  | 포함 (대소문자 무시) | `url \|~ "API"`     |
| `!=`   | 일치하지 않음        | `method!="OPTIONS"` |
| `!~`   | 포함하지 않음        | `url!~="static"`    |

### 논리 연산자

| 연산자 | 설명              | 예시                            |
| ------ | ----------------- | ------------------------------- |
| `and`  | AND 조건 (기본값) | `method="GET" and status="2xx"` |
| `or`   | OR 조건           | `method="GET" or status="5xx"`  |

## 📋 사용 예시

### 기본 필터링

```text
# GET 요청만 필터링
method="GET"

# 성공 응답만 필터링
status="2xx"

# API 관련 요청만 필터링
url|="api"

# 에러 응답 제외
status!="4xx,5xx"
```

### 복합 조건

```text
# 여러 HTTP 메서드
method="GET,POST"

# 복합 AND 조건
method="GET" and status="2xx" and url|="api"

# OR 조건
method="POST" or status="5xx"

# 복잡한 조건
(method="GET" or method="POST") and status="2xx" and url!~="static"
```

### 실제 사용 시나리오

```text
# API 요청만 모니터링
url|="api" and method!="OPTIONS"

# 에러 응답 분석
status="4xx,5xx"

# 특정 도메인 제외
url!~="google-analytics" and url!~="ads"

# 개발 환경 API만
url|="localhost" or url|="127.0.0.1"
```

## 🎨 테마 지원

- **라이트 테마**: `cheolsu-light`
- **다크 테마**: `cheolsu-dark`
- **자동 테마 전환**: 시스템 테마에 따라 자동 변경

## ⌨️ 키보드 단축키

- **⌘ + Enter**: 쿼리 적용
- **자동완성**: `"`, `,`, ` `, `|`, `!`, `=` 입력 시 트리거

## 🔍 자동완성 기능

### 키워드 제안

- `method`, `methods`, `status`, `url`

### HTTP 메서드 제안

- `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`, `CONNECT`, `TRACE`

### 상태 코드 제안

- `1xx` (Informational)
- `2xx` (Success)
- `3xx` (Redirection)
- `4xx` (Client Error)
- `5xx` (Server Error)
- 구체적인 코드: `200`, `201`, `204`, `400`, `401`, `403`, `404`, `500`, `502`, `503`

### 연산자 제안

- `=`, `|=`, `|~`, `!=`, `!~`

## 💡 사용 팁

### 1. 효율적인 필터링

```text
# 정확한 조건 사용
method="GET"  # ✅ 좋음
method|="GET" # ❌ 불필요한 패턴 매칭
```

### 2. 성능 최적화

```text
# 구체적인 조건을 먼저 배치
url|="api" and method="GET"  # ✅ 좋음
method="GET" and url|="api"  # ❌ URL 패턴이 더 선택적
```

### 3. 가독성 향상

```text
# 복잡한 조건은 괄호 사용
(method="GET" or method="POST") and status="2xx"
```

## 🚀 고급 사용법

### 정규식 패턴 (향후 지원 예정)

```text
# URL 패턴 매칭
url~="^/api/v[0-9]+/"

# 복잡한 상태 코드 패턴
status~="^[45][0-9][0-9]$"
```

### 시간 기반 필터링 (향후 지원 예정)

```text
# 최근 1시간 내 요청
time>="1h"

# 특정 시간대
time>="09:00" and time<="18:00"
```

## 🔧 문제 해결

### 자주 발생하는 오류

1. **문법 오류**

   ```text
   # ❌ 잘못된 예시
   method=GET

   # ✅ 올바른 예시
   method="GET"
   ```

2. **연산자 오류**

   ```text
   # ❌ 잘못된 예시
   method=="GET"

   # ✅ 올바른 예시
   method="GET"
   ```

3. **논리 연산자 오류**

   ```text
   # ❌ 잘못된 예시
   method="GET" && status="2xx"

   # ✅ 올바른 예시
   method="GET" and status="2xx"
   ```

## 📚 관련 문서

- [기본 사용법](../guide/basic-usage.md)
- [문제 해결](../guide/troubleshooting.md)
- [TLS 지원](./tls-support.md)
