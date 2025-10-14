---
title: 시작하기
description: Cheolsu Proxy 설치 및 기본 사용법
---

# 시작하기

Cheolsu Proxy를 사용하여 네트워크 트래픽을 모니터링하고 분석하는 방법을 알아보세요.

## 시스템 요구사항

- **macOS**: 10.15 (Catalina) 이상
- **Windows**: Windows 10 이상 (향후 지원)
- **Linux**: Ubuntu 18.04 이상 (향후 지원)

## 설치

### 1. 바이너리 다운로드

GitHub Releases에서 최신 버전을 다운로드하세요:

```bash
# macOS
curl -L https://github.com/ohah/cheolsu-proxy/releases/latest/download/cheolsu-proxy-macos.tar.gz | tar -xz
```

### 2. 실행

```bash
./cheolsu-proxy
```

## 첫 실행 설정

### 1. 인증서 자동 생성

Cheolsu Proxy는 첫 실행 시 자동으로 고유한 CA 인증서를 생성합니다. 콘솔에 다음과 같은 메시지가 표시됩니다:

```
🔐 CA 인증서가 생성되었습니다.
📁 경로: ~/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer
```

### 2. 인증서 설치

HTTPS 트래픽을 모니터링하려면 CA 인증서를 시스템에 설치해야 합니다.

**macOS에서 설치:**

1. Keychain Access 앱을 실행하세요
2. 'login' 키체인을 선택하세요
3. File > Import Items... 메뉴를 선택하세요
4. `~/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer` 파일을 선택하세요
5. 인증서를 더블클릭하고 '항상 신뢰'로 설정하세요

### 3. 시스템 프록시 설정

시스템 프록시를 `127.0.0.1:8100`으로 설정하세요.

**macOS에서 설정:**

1. System Preferences > Network
2. 고급 > 프록시 탭
3. "웹 프록시(HTTP)"와 "보안 웹 프록시(HTTPS)"를 체크
4. 서버: `127.0.0.1`, 포트: `8100`

## 기본 사용법

### 1. 프록시 시작

1. Cheolsu Proxy를 실행하세요
2. 기본 설정으로 `127.0.0.1:8100`에서 프록시가 시작됩니다
3. 다른 주소나 포트를 사용하려면 설정을 변경할 수 있습니다

### 2. 트래픽 모니터링

1. 웹 브라우저나 다른 애플리케이션에서 네트워크 요청을 시작하세요
2. Cheolsu Proxy의 메인 화면에서 실시간으로 요청 목록을 확인할 수 있습니다
3. 각 요청을 클릭하면 상세 정보를 볼 수 있습니다

### 3. 요청 필터링

- 메서드별 필터링: GET, POST, PUT, DELETE 등
- 상태 코드별 필터링: 200, 404, 500 등
- URL 패턴별 검색

### 4. 요청 관리

- **삭제**: 개별 요청을 목록에서 제거
- **정리**: 모든 요청을 한 번에 삭제
- **내보내기**: 요청 데이터를 파일로 저장 (향후 지원)

## 문제 해결

### 인증서 관련 문제

**"인증서를 신뢰할 수 없습니다" 오류:**

1. 인증서가 올바르게 설치되었는지 확인
2. Keychain Access에서 인증서의 신뢰 설정 확인
3. 브라우저의 인증서 저장소 확인

**HTTPS 사이트에 접근할 수 없음:**

1. 시스템 프록시 설정이 올바른지 확인
2. 방화벽이 포트 8100을 차단하지 않는지 확인
3. 다른 프록시 소프트웨어와 충돌하지 않는지 확인

### 연결 문제

**프록시에 연결할 수 없음:**

1. Cheolsu Proxy가 실행 중인지 확인
2. 포트 8100이 사용 중이지 않은지 확인
3. 방화벽 설정 확인

## 다음 단계

- [주요 기능](/ko/guide/features) - Cheolsu Proxy의 모든 기능 살펴보기
- [인증서 설정](/ko/guide/certificate-setup) - 상세한 인증서 설정 가이드
- [기여하기](/ko/contributing/) - 프로젝트에 기여하는 방법
