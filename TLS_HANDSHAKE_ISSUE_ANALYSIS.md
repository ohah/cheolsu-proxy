# TLS 1.2 rustls handshake failed 이슈 분석

## 🔍 문제 개요

Apple 서비스들(`wps.apple.com`, `gateway.icloud.com` 등)과의 TLS 핸드셰이크에서 `tls handshake eof` 오류가 발생하는 문제를 분석했습니다.

## 📊 로그 분석 결과

### ✅ 성공하는 연결들

- `api2.cursor.sh:443` - TLS 1.2 성공
- `ssl.gstatic.com:443` - TLS 1.2 성공
- `chat.google.com:443` - TLS 1.2 성공

### ❌ 실패하는 연결들

- `wps.apple.com:443` - TLS 1.2 실패 (tls handshake eof)
- `gateway.icloud.com:443` - TLS 1.2 실패 (tls handshake eof)
- `jamf.payhere.in:8443` - TLS 1.2 실패 (tls handshake eof)

## 🔍 핵심 발견사항

### 1. ClientHello 패턴 차이

#### 성공하는 연결 (api2.cursor.sh):

```
📊 초기 버퍼 크기: 253 bytes
  - 레코드 길이: 248 bytes
  - 세션 ID 길이: 32 bytes
  - 암호화 스위트 길이: 36 bytes
  - 암호화 스위트 개수: 18
  - 암호화 스위트: ["0x1301", "0x1302", "0x1303", "0xc02f", "0xc02b", "0xc030", "0xc02c", "0xc027", "0xcca9", "0xcca8"]
  - Extensions: 1개 - SNI (0x0000, 3328 bytes)
```

#### 실패하는 연결 (wps.apple.com):

```
📊 초기 버퍼 크기: 517 bytes
  - 레코드 길이: 512 bytes
  - 세션 ID 길이: 32 bytes
  - 암호화 스위트 길이: 42 bytes
  - 암호화 스위트 개수: 21
  - 암호화 스위트: ["0xcaca", "0x1301", "0x1302", "0x1303", "0xc02c", "0xc02b", "0xcca9", "0xc030", "0xc02f", "0xcca8"]
  - Extensions: 1개 - ec_point_formats (0x000b, 2666 bytes)
```

### 2. 핵심 차이점

| 구분                  | 성공하는 연결 | 실패하는 연결             |
| --------------------- | ------------- | ------------------------- |
| **Extensions**        | SNI (0x0000)  | ec_point_formats (0x000b) |
| **SNI 지원**          | ✅ 있음       | ❌ 없음                   |
| **Apple 특별 스위트** | ❌ 없음       | ✅ 0xcaca                 |
| **메시지 크기**       | 253 bytes     | 517 bytes                 |

## 🚨 문제의 핵심

### 1. SNI Extension 부재

- **Apple 서비스들은 SNI Extension을 사용하지 않음**
- 대신 `ec_point_formats` Extension만 사용
- rustls가 SNI를 기대하는데 없어서 핸드셰이크 실패

### 2. Apple 특별 암호화 스위트

- `0xcaca`는 Apple만의 특별한 암호화 스위트
- rustls가 이 스위트를 지원하지 않을 수 있음

### 3. Extensions 구조 차이

- 성공하는 연결: SNI 기반
- 실패하는 연결: ec_point_formats 기반

## 💡 해결방안

### 1. SNI Extension 기반 처리 (범용적 접근)

- **SNI가 없는 연결을 감지하여 OpenSSL 우선 사용**
- Apple 도메인 하드코딩 대신 SNI Extension 존재 여부로 판단
- 모든 SNI 없는 연결에 대해 범용적으로 적용

### 2. OpenSSL 우선 사용

- SNI가 없는 연결은 OpenSSL을 우선 사용
- rustls는 SNI를 기대하므로 SNI 없는 연결에서 문제 발생
- OpenSSL은 SNI 없이도 연결 처리 가능

### 3. Extensions 처리 개선

- SNI가 없어도 다른 Extensions로 연결을 처리할 수 있도록
- ec_point_formats Extension을 적절히 처리

### 4. 범용적 접근법

- 특정 도메인에 의존하지 않고 SNI Extension 기반으로 판단
- 미래의 다른 SNI 없는 서비스들도 자동으로 처리 가능

## 🔧 구현 계획

1. **SNI Extension 감지 로직 추가** ✅
2. **SNI 없는 연결 처리 개선** ✅
3. **OpenSSL 우선 사용 로직** ✅
4. **범용적 접근법 적용** ✅

## 📝 참고사항

- 이 문제는 Apple 서비스들의 특별한 TLS 구현 때문에 발생
- 일반적인 웹 서비스들은 SNI를 사용하므로 문제없음
- Apple 생태계의 특별한 요구사항을 고려한 해결책 필요
