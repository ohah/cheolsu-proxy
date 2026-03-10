# Cheolsu-Query

## 개요

Cheolsu-Query는 Cheolsu Proxy에서 캡처된 네트워크 트래픽을 필터링하기 위한 전용 쿼리 언어입니다. 수백, 수천 개의 요청이 쌓인 트래픽 목록에서 원하는 요청만 빠르게 찾을 수 있습니다.

트래픽 목록을 스크롤하며 눈으로 찾는 대신 Cheolsu-Query를 사용하면, HTTP 메서드, 상태 코드, URL 패턴 등의 조건을 조합하여 정확히 원하는 요청만 표시할 수 있습니다. 예를 들어 "POST 요청 중 500 에러가 발생한 API 호출"을 찾으려면 다음과 같이 입력합니다:

```text
method="POST" and status="5xx" and url|="api"
```

Monaco Editor 기반으로 구현되어 있어 VS Code와 동일한 편집 경험을 제공하며, 구문 강조, 자동완성, 실시간 문법 검증을 지원합니다.

---

## 필터 키워드

| 키워드    | 설명                                | 예시                 |
| --------- | ----------------------------------- | -------------------- |
| `method`  | HTTP 메서드 필터링                  | `method="GET"`       |
| `methods` | 여러 HTTP 메서드 필터링 (쉼표 구분) | `methods="GET,POST"` |
| `status`  | HTTP 상태 코드 필터링               | `status="2xx,404"`   |
| `url`     | URL 패턴 필터링                     | `url\|="api"`        |

---

## 연산자

### 비교 연산자

| 연산자 | 설명                 | 예시                |
| ------ | -------------------- | ------------------- |
| `=`    | 정확히 일치          | `method="GET"`      |
| `\|=`  | 포함 (대소문자 구분) | `url\|="api"`       |
| `\|~`  | 포함 (대소문자 무시) | `url\|~="API"`      |
| `!=`   | 일치하지 않음        | `method!="OPTIONS"` |
| `!~`   | 포함하지 않음        | `url!~="static"`    |

### 논리 연산자

| 연산자 | 설명              | 예시                            |
| ------ | ----------------- | ------------------------------- |
| `and`  | AND 조건 (기본값) | `method="GET" and status="2xx"` |
| `or`   | OR 조건           | `method="GET" or status="5xx"`  |

괄호 `()`를 사용하여 논리 연산의 우선순위를 지정할 수 있습니다.

---

## 사용 예시

### 기본 필터링

```text
# GET 요청만 표시
method="GET"

# 성공 응답만 표시
status="2xx"

# URL에 "api"가 포함된 요청만 표시
url|="api"

# 에러 응답 제외
status!="4xx,5xx"
```

### 복합 조건

```text
# GET 또는 POST 요청만 표시
methods="GET,POST"

# API 요청 중 성공한 GET 요청만 표시
method="GET" and status="2xx" and url|="api"

# POST 요청이거나 서버 에러인 것 표시
method="POST" or status="5xx"

# 괄호를 사용한 조건 그룹화
(method="GET" or method="POST") and status="2xx" and url!~="static"
```

### 실제 사용 시나리오

```text
# CORS preflight(OPTIONS)를 제외한 API 요청만 모니터링
url|="api" and method!="OPTIONS"

# 클라이언트/서버 에러 응답만 분석
status="4xx,5xx"

# 광고/분석 관련 요청을 제외하고 보기
url!~="google-analytics" and url!~="ads"

# 로컬 개발 서버 요청만 표시
url|="localhost" or url|="127.0.0.1"

# 특정 API 엔드포인트의 실패한 요청 찾기
url|="/api/v1/users" and status!="2xx"
```

---

## 테마 지원

- **라이트 테마**: `cheolsu-light`
- **다크 테마**: `cheolsu-dark`
- 시스템 테마에 따라 자동 전환

---

## 키보드 단축키

| 단축키             | 동작      |
| ------------------ | --------- |
| `Cmd/Ctrl + Enter` | 쿼리 적용 |

자동완성은 `"`, `,`, ` `, `|`, `!`, `=` 입력 시 자동으로 트리거됩니다.

---

## 자동완성 기능

### 키워드 제안

`method`, `methods`, `status`, `url`

### HTTP 메서드 제안

`GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`, `CONNECT`, `TRACE`

### 상태 코드 제안

- 범위 코드: `1xx` (Informational), `2xx` (Success), `3xx` (Redirection), `4xx` (Client Error), `5xx` (Server Error)
- 구체적 코드: `200`, `201`, `204`, `400`, `401`, `403`, `404`, `500`, `502`, `503`

### 연산자 제안

`=`, `|=`, `|~`, `!=`, `!~`

---

## 문법 주의 사항

### 값은 반드시 따옴표로 감싸야 합니다

```text
# 올바름
method="GET"

# 오류
method=GET
```

### 등호는 하나만 사용합니다

```text
# 올바름
method="GET"

# 오류
method=="GET"
```

### 논리 연산자는 `and` / `or`를 사용합니다

```text
# 올바름
method="GET" and status="2xx"

# 오류
method="GET" && status="2xx"
```

---

## 빠른 참조

```text
# 메서드 필터
method="GET"                    # 정확히 일치
methods="GET,POST"              # 여러 메서드

# URL 필터
url|="api"                      # 포함 (대소문자 구분)
url|~="API"                     # 포함 (대소문자 무시)
url!~="static"                  # 포함하지 않음

# 상태 코드 필터
status="2xx"                    # 2xx 범위 전체
status="200,201"                # 특정 코드
status!="4xx,5xx"               # 에러 제외

# 조합
method="POST" and status="5xx"  # AND 조건
method="GET" or status="5xx"    # OR 조건
(A or B) and C                  # 괄호로 그룹화
```

---

## 관련 문서

- [기본 사용법](../guide/basic-usage.md)
- [문제 해결](../guide/troubleshooting.md)
