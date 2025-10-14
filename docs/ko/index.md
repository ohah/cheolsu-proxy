---
title: Cheolsu Proxy
description: Rust와 Tauri로 구축된 간단한 Man In The Middle 프록시
---

<div align="center">
<img style="width:100px; margin:auto" src="/assets/logo.png">
<h1> Cheolsu Proxy </h1>
<h2> A simple <i>Man In The Middle</i> proxy</h2>
</div>

[![build](https://github.com/ohah/cheolsu-proxy/actions/workflows/autofix.yml/badge.svg?branch=master)](https://github.com/ohah/cheolsu-proxy/actions/workflows/autofix.yml)
![GitHub](https://img.shields.io/github/license/ohah/cheolsu-proxy/)
![GitHub last commit](https://img.shields.io/github/last-commit/ohah/cheolsu-proxy/)
![GitHub top language](https://img.shields.io/github/languages/top/ohah/cheolsu-proxy/)

## 소개

Rust 기반의 **Man in the Middle 프록시**로, 네트워크 트래픽에 대한 가시성을 제공하는 초기 단계 프로젝트입니다. 현재는 HTTP 및 HTTPS 요청과 응답을 표시하지만, 향후 더 고급 사용 사례를 위해 트래픽 조작을 허용하는 것이 목표입니다.

![Cast](/assets/screenshots/0.gif)

## 주요 기능

- 🔐 HTTP / HTTPS 지원
- 🔒 TLS 1.0/1.1 레거시 클라이언트 지원 (하이브리드 TLS 핸들러)
- 🖱️ GUI 인터페이스
- ⌨️ 사용자 정의 주소 및 리스닝 포트 선택 가능
- 🔍 각 요청 및 응답에 대한 상세 정보
- 🎯 메서드별 요청 목록 필터링
- ❌ 목록에서 단일 요청 삭제
- 🚫 모든 요청 삭제 및 테이블 정리
- 🌌 다크/라이트 테마
- 🔐 **보안**: 사용자별 고유 CA 인증서 자동 생성 (개인키는 바이너리에 포함되지 않음)

## 빠른 시작

### 1. 인증서 자동 생성 및 설치

Cheolsu Proxy는 첫 실행 시 자동으로 고유한 CA 인증서를 생성합니다.

**macOS에서 인증서 수동 설치:**

1. 앱을 실행하면 콘솔에 인증서 파일 경로가 표시됩니다:

   ```
   📁 경로: ~/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer
   ```

2. Keychain Access 앱을 실행하세요
3. 'login' 키체인을 선택하세요
4. File > Import Items... 메뉴를 선택하세요
5. 위 경로의 `cheolsu-proxy.cer` 파일을 선택하세요
6. 인증서를 더블클릭하고 '항상 신뢰'로 설정하세요

### 2. 시스템 프록시 설정

로컬 시스템 프록시를 `127.0.0.1:8100`으로 설정하세요.

자세한 설정 방법은 [인증서 설정 가이드](/ko/guide/certificate-setup)를 참조하세요.

## 스크린샷

### 리스닝 주소 입력

![Mitm proxy Screenshot 1](/assets/screenshots/1b.png)
![Mitm proxy Screenshot 1](/assets/screenshots/1w.png)
![Mitm proxy Screenshot 1](/assets/screenshots/2w.png)

### 요청 목록

![Mitm proxy Screenshot 2](/assets/screenshots/3w.png)
![Mitm proxy Screenshot 2](/assets/screenshots/3b.png)
![Mitm proxy Screenshot 2](/assets/screenshots/4b.png)

### 요청 및 응답 상세 정보

![Mitm proxy Screenshot 3](/assets/screenshots/5b.png)
![Mitm proxy Screenshot 3](/assets/screenshots/5w.png)

## 라이선스

자세한 내용은 [LICENSE-APACHE](https://github.com/ohah/cheolsu-proxy/blob/master/LICENSE-APACHE), [LICENSE-MIT](https://github.com/ohah/cheolsu-proxy/blob/master/LICENSE-MIT)를 참조하세요.

## 기여하기

기여는 항상 환영합니다! 자세한 내용은 [기여 가이드](/ko/contributing/)를 참조하세요.

## 문서 및 도움말

[cheolsu-proxy](https://github.com/ohah/cheolsu-proxy) 사용에 대한 질문이 있으시면 GitHub Discussions를 이용해주세요!
![GitHub Discussions](https://img.shields.io/github/discussions/ohah/cheolsu-proxy)
