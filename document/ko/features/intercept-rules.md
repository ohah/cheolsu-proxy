# 인터셉트 규칙

## 개요

인터셉트 규칙은 Cheolsu Proxy를 통과하는 HTTP 트래픽을 자동으로 가로채고 수정하는 기능입니다. 와일드카드 패턴으로 대상 요청을 지정하고, 사전에 정의된 액션을 적용하여 요청이나 응답을 변경할 수 있습니다.

이 기능은 다양한 개발 및 디버깅 시나리오에서 유용합니다.

- 개발 중인 API의 응답을 변경하여 에러 처리 로직을 테스트
- 광고나 분석 도메인의 요청을 차단
- 프로덕션 서버 요청을 로컬 개발 서버로 리다이렉트
- CORS 헤더를 추가하여 크로스 오리진 문제 해결
- 로컬 파일로 서버 응답을 대체하여 프론트엔드 개발 속도 향상

스크립팅과 비교하면, 인터셉트 규칙은 코드 작성 없이 GUI에서 간단히 설정할 수 있다는 장점이 있습니다. 반복적이고 정형화된 트래픽 조작에 적합하며, 복잡한 조건 분기나 동적 로직이 필요한 경우에는 [스크립팅](./scripting.md)을 사용하는 것이 좋습니다.

---

## 액션 타입

### 1. Block (차단)

요청을 차단하고 지정된 상태 코드와 응답 바디를 반환합니다. 실제 서버로 요청이 전달되지 않습니다.

- **상태 코드**: HTTP 상태 코드 설정 (기본 403)
- **응답 바디**: 커스텀 응답 본문

**활용 사례**

광고 차단이나 특정 API 호출을 방지할 때 유용합니다. 예를 들어, 모바일 앱에서 광고 SDK의 네트워크 요청을 차단하여 광고 없는 환경에서 테스트하거나, 외부 분석 서비스로의 요청을 차단하여 네트워크 트래픽을 줄일 수 있습니다.

```
패턴:    *ads.example.com*
상태 코드: 403
응답 바디: {"error": "blocked"}
```

```
패턴:    *google-analytics.com*
상태 코드: 204
응답 바디: (비어 있음)
```

### 2. Modify Request (요청 수정)

서버로 전달되기 전에 요청을 수정합니다. 원본 요청은 변경되지 않고, 프록시가 서버로 전달하는 요청만 수정됩니다.

- **헤더 추가**: 새로운 요청 헤더 추가
- **헤더 제거**: 기존 요청 헤더 제거
- **바디 변경**: 요청 본문 교체

**활용 사례**

인증 헤더를 자동으로 추가하거나, User-Agent를 변경하여 특정 디바이스를 시뮬레이션할 때 유용합니다. API 키가 필요한 서비스를 테스트할 때 매번 헤더를 수동으로 설정하지 않아도 됩니다.

```
패턴:       *api.example.com/v1/*
헤더 추가:   Authorization: Bearer eyJhbGciOiJIUzI1NiIs...
```

```
패턴:       *example.com*
헤더 추가:   User-Agent: Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)
헤더 제거:   Accept-Encoding
```

### 3. Modify Response (응답 수정)

서버 응답을 클라이언트에 전달하기 전에 수정합니다. 서버의 실제 응답은 정상적으로 수신하되, 클라이언트가 받는 응답만 변경됩니다.

- **상태 코드 변경**: HTTP 상태 코드 수정
- **헤더 추가/제거**: 응답 헤더 조작
- **바디 변경**: 응답 본문 교체

**활용 사례**

CORS 문제를 해결하거나, 에러 상황을 시뮬레이션할 때 유용합니다. 로컬 개발 환경에서 API 서버가 CORS 헤더를 반환하지 않는 경우, 이 규칙으로 필요한 헤더를 추가할 수 있습니다.

```
패턴:       *api.example.com*
헤더 추가:   Access-Control-Allow-Origin: *
헤더 추가:   Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS
헤더 추가:   Access-Control-Allow-Headers: Content-Type, Authorization
```

```
패턴:       *api.example.com/users*
상태 코드:   500
바디 변경:   {"error": "Internal Server Error", "message": "Database connection failed"}
```

### 4. Map Local (로컬 파일 매핑)

요청에 대해 서버 대신 로컬 파일의 내용을 응답으로 반환합니다. 서버에 요청을 보내지 않고 로컬 파일로 즉시 응답합니다.

- **파일 경로**: 로컬 파일 시스템의 파일 경로
- **상태 코드**: 커스텀 HTTP 상태 코드
- **응답 헤더**: 커스텀 응답 헤더

**활용 사례**

프론트엔드 개발 시 서버에서 제공하는 JavaScript, CSS, JSON 파일을 로컬에서 수정 중인 파일로 대체할 때 유용합니다. 프로덕션 사이트를 열어둔 채로 로컬에서 수정한 파일을 바로 적용하여 결과를 확인할 수 있습니다.

```
패턴:       *cdn.example.com/app.js
파일 경로:   /Users/dev/project/dist/app.js
상태 코드:   200
응답 헤더:   Content-Type: application/javascript
```

```
패턴:       *api.example.com/config.json
파일 경로:   /Users/dev/mock-data/config.json
상태 코드:   200
응답 헤더:   Content-Type: application/json
```

### 5. Map Remote (원격 URL 매핑)

요청을 다른 URL로 리다이렉트합니다. 클라이언트는 리다이렉트가 발생한 사실을 알지 못하며, 프록시가 대상 URL로 요청을 전달하고 그 응답을 클라이언트에 반환합니다.

- **타겟 URL**: 요청을 보낼 대상 URL
- **경로 유지**: 원본 요청의 경로를 타겟 URL에 유지할지 여부

**활용 사례**

프로덕션 환경의 API 요청을 로컬 개발 서버로 리다이렉트하거나, API 버전을 변경하여 테스트할 때 유용합니다. 모바일 앱처럼 API 서버 주소를 쉽게 변경할 수 없는 환경에서 특히 효과적입니다.

```
패턴:       *api.example.com*
타겟 URL:   http://localhost:3000
경로 유지:   예
```

위 설정을 적용하면 `https://api.example.com/v1/users` 요청이 `http://localhost:3000/v1/users`로 전달됩니다.

```
패턴:       *api.example.com/v1/*
타겟 URL:   https://api.example.com/v2/
경로 유지:   예
```

위 설정을 적용하면 API v1 요청이 v2 엔드포인트로 자동 전환됩니다.

---

## 규칙 설정

### 패턴 매칭

와일드카드 패턴을 사용하여 URL을 매칭합니다. `*`는 임의의 문자열(0개 이상)과 일치합니다.

| 패턴 | 설명 | 매칭 예시 |
| --- | --- | --- |
| `*ads.example.com*` | 호스트명에 ads.example.com이 포함된 모든 URL | `https://ads.example.com/banner` |
| `*api.example.com/v1/*` | 특정 API 경로 아래의 모든 요청 | `https://api.example.com/v1/users` |
| `*.json` | JSON 파일 요청 | `https://cdn.example.com/config.json` |
| `*example.com*/graphql` | 특정 엔드포인트 | `https://api.example.com/graphql` |
| `*localhost:8080*` | 로컬 개발 서버 | `http://localhost:8080/api/data` |

### HTTP 메서드 필터

선택적으로 HTTP 메서드를 지정하여 특정 메서드의 요청만 매칭할 수 있습니다.

- GET, POST, PUT, DELETE, PATCH, OPTIONS 등
- 미지정 시 모든 메서드에 적용

예를 들어, CORS preflight 요청(OPTIONS)에만 응답 헤더를 추가하거나, POST 요청만 차단하는 등의 세밀한 제어가 가능합니다.

### 규칙 활성화/비활성화

각 규칙은 개별적으로 활성화/비활성화할 수 있습니다. 규칙을 삭제하지 않고도 일시적으로 비활성화할 수 있어, 자주 사용하는 규칙을 보관해두고 필요할 때 켜고 끄는 방식으로 활용할 수 있습니다.

---

## 사용 방법

### Desktop

1. 사이드바에서 **Intercept Rules** 메뉴 선택
2. **규칙 추가** 버튼 클릭
3. 패턴, HTTP 메서드(선택), 액션 타입, 세부 옵션 설정
4. 저장

트래픽 테이블에서 트랜잭션을 우클릭하여 해당 요청의 URL을 기반으로 빠르게 규칙을 생성할 수도 있습니다. 자주 디버깅하는 API가 있다면 이 방법이 편리합니다.

### TUI

1. **Rules** 탭으로 이동
2. 규칙 추가/편집/삭제 가능
3. 패턴과 액션 타입을 폼으로 입력

### MCP

AI 어시스턴트를 통해 자연어로 규칙을 관리할 수 있습니다.

```
"example.com 도메인을 차단하는 규칙을 추가해줘"
"API 응답에 CORS 헤더를 추가하는 규칙을 만들어줘"
"api.example.com 요청을 localhost:3000으로 리다이렉트하는 규칙을 만들어줘"
```
