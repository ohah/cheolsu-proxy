---
title: 인증서 설정
description: 운영체제별 CA 인증서 설치 및 설정 가이드
---

# 인증서 설정

HTTPS 트래픽을 모니터링하려면 Cheolsu Proxy가 생성한 CA 인증서를 시스템에 설치해야 합니다.

## 인증서 파일 위치

Cheolsu Proxy는 다음 위치에 CA 인증서를 생성합니다:

- **macOS**: `~/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer`
- **Windows**: `%APPDATA%/com.cheolsu-proxy/cheolsu-proxy.cer` (향후 지원)
- **Linux**: `~/.config/com.cheolsu-proxy/cheolsu-proxy.cer` (향후 지원)

## macOS 설정

### 1. Keychain Access를 통한 설치

1. **Keychain Access 앱 실행**

   - Spotlight 검색에서 "Keychain Access" 입력
   - 또는 Applications > Utilities > Keychain Access

2. **키체인 선택**

   - 왼쪽 사이드바에서 "login" 키체인 선택
   - 시스템 전체에서 사용하려면 "System" 키체인 선택

3. **인증서 가져오기**

   - 메뉴에서 File > Import Items... 선택
   - `~/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer` 파일 선택
   - "Open" 클릭

4. **신뢰 설정**
   - 가져온 인증서를 더블클릭
   - "Trust" 섹션 확장
   - "When using this certificate"를 "Always Trust"로 설정
   - 창을 닫으면 비밀번호 입력 요청
   - 관리자 비밀번호 입력

### 2. 터미널을 통한 설치

```bash
# 인증서 파일을 시스템 키체인에 추가
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ~/Library/Application\ Support/com.cheolsu-proxy/cheolsu-proxy.cer
```

### 3. 설치 확인

```bash
# 인증서가 올바르게 설치되었는지 확인
security find-certificate -c "cheolsu-proxy" /Library/Keychains/System.keychain
```

## Windows 설정 (향후 지원)

### 1. 인증서 관리자 사용

1. **인증서 관리자 실행**

   - Windows 키 + R을 누르고 `certmgr.msc` 입력
   - Enter 키 누르기

2. **신뢰할 수 있는 루트 인증 기관 추가**

   - 왼쪽 트리에서 "Trusted Root Certification Authorities" > "Certificates" 선택
   - 오른쪽 클릭 > "All Tasks" > "Import..." 선택

3. **인증서 가져오기**
   - `%APPDATA%/com.cheolsu-proxy/cheolsu-proxy.cer` 파일 선택
   - "Next" 클릭하여 마법사 완료

### 2. PowerShell을 통한 설치

```powershell
# 인증서 가져오기
Import-Certificate -FilePath "$env:APPDATA\com.cheolsu-proxy\cheolsu-proxy.cer" -CertStoreLocation Cert:\LocalMachine\Root
```

## Linux 설정 (향후 지원)

### 1. Ubuntu/Debian

```bash
# 인증서를 시스템 인증서 저장소에 복사
sudo cp ~/.config/com.cheolsu-proxy/cheolsu-proxy.cer /usr/local/share/ca-certificates/cheolsu-proxy.crt

# 인증서 저장소 업데이트
sudo update-ca-certificates
```

### 2. CentOS/RHEL/Fedora

```bash
# 인증서를 시스템 인증서 저장소에 복사
sudo cp ~/.config/com.cheolsu-proxy/cheolsu-proxy.cer /etc/pki/ca-trust/source/anchors/cheolsu-proxy.crt

# 인증서 저장소 업데이트
sudo update-ca-trust
```

### 3. 수동 설정

```bash
# 인증서를 개별 애플리케이션에 추가
# Firefox
certutil -A -n "cheolsu-proxy" -t "C,," -i ~/.config/com.cheolsu-proxy/cheolsu-proxy.cer -d sql:$HOME/.mozilla/firefox/*.default

# Chrome/Chromium
# Chrome은 시스템 인증서 저장소를 사용하므로 위의 시스템 설정이 필요
```

## 브라우저별 설정

### Chrome/Chromium

Chrome은 시스템 인증서 저장소를 사용하므로, 위의 운영체제별 설정만으로 충분합니다.

### Firefox

Firefox는 자체 인증서 저장소를 사용합니다:

1. **Firefox 설정 열기**

   - 주소창에 `about:preferences#privacy` 입력
   - "Certificates" 섹션에서 "View Certificates" 클릭

2. **인증서 가져오기**
   - "Authorities" 탭 선택
   - "Import..." 버튼 클릭
   - 인증서 파일 선택
   - "Trust this CA to identify websites" 체크
   - "OK" 클릭

### Safari

Safari는 macOS의 Keychain을 사용하므로, 위의 macOS 설정만으로 충분합니다.

## 문제 해결

### 인증서가 신뢰되지 않는 경우

1. **인증서 재설치**

   - 기존 인증서 삭제 후 재설치
   - 브라우저 캐시 및 쿠키 삭제

2. **권한 확인**

   - 관리자 권한으로 설치했는지 확인
   - 파일 권한이 올바른지 확인

3. **시간 동기화**
   - 시스템 시간이 올바른지 확인
   - 인증서 유효 기간 확인

### HTTPS 사이트에 접근할 수 없는 경우

1. **프록시 설정 확인**

   - 시스템 프록시가 `127.0.0.1:8100`으로 설정되었는지 확인
   - Cheolsu Proxy가 실행 중인지 확인

2. **방화벽 확인**

   - 방화벽이 포트 8100을 차단하지 않는지 확인
   - Cheolsu Proxy가 네트워크 접근을 허용하는지 확인

3. **다른 프록시 소프트웨어**
   - 다른 프록시나 VPN 소프트웨어와 충돌하지 않는지 확인
   - 프록시 설정 우선순위 확인

### 인증서 오류 메시지

**"NET::ERR_CERT_AUTHORITY_INVALID"**

- CA 인증서가 올바르게 설치되지 않음
- 인증서 재설치 필요

**"NET::ERR_CERT_COMMON_NAME_INVALID"**

- 인증서의 Common Name이 요청한 도메인과 일치하지 않음
- Cheolsu Proxy의 동적 인증서 생성 기능 확인

**"NET::ERR_CERT_DATE_INVALID"**

- 시스템 시간이 잘못됨
- 인증서가 만료됨 (재생성 필요)

## 보안 고려사항

### 인증서 보안

- CA 인증서는 로컬에서만 사용
- 외부로 전송하지 않음
- 정기적인 인증서 갱신 (향후 지원)

### 네트워크 보안

- 로컬 네트워크에서만 사용 권장
- 공용 네트워크에서는 주의
- 방화벽 설정으로 접근 제한

### 데이터 보호

- 모든 트래픽 데이터는 로컬에만 저장
- 외부 서버로 전송되지 않음
- 개인정보 보호 정책 준수
