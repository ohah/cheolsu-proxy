# Cheolsu Proxy

Rust와 Tauri로 구축된 간단한 Man In The Middle 프록시입니다.

> 이 프로젝트는 [Proxelar](https://github.com/emanuele-em/proxelar)에서 포크하여 시작되었습니다.

## 주요 기능

- **TLS 지원**: HTTPS 트래픽 가로채기 및 분석
- **인증서 관리**: 자동 인증서 생성 및 설치
- **크로스 플랫폼**: macOS 지원 (Windows 지원 예정)
- **사용자 친화적**: 직관적인 GUI 인터페이스

## 사용하기

Cheolsu Proxy를 사용하는 방법을 단계별로 안내합니다.

- [기본 사용법](/usage/basic-usage) - 처음 사용자를 위한 간단한 가이드
- [프록시 설정](/usage/proxy-setup) - 포트 변경 등 기본 설정
- [인증서 설치](/usage/certificate-setup) - HTTPS 사이트 사용을 위한 인증서
- [문제 해결](/usage/troubleshooting) - 자주 발생하는 문제 해결

## 가이드

개발 및 고급 사용을 위한 기술 가이드입니다.

- [시작하기](/guide/getting-started) - 개발 환경 설정 및 빌드
- [주요 기능](/guide/features) - 상세 기능 설명

## 기여하기

프로젝트에 기여하고 싶으시다면 [기여자 가이드](/contributing/)를 참조하세요.

- [개발 환경 설정](/contributing/development-setup)
- [프로젝트 구조](/contributing/code-structure)
- [테스트](/contributing/testing)

## 기능 문서

- [TLS 1.0/1.1 지원](/features/tls-support) - 레거시 TLS 클라이언트 지원

## 라이센스

이 프로젝트는 MIT 및 Apache 2.0 라이센스 하에 배포됩니다.

- [MIT 라이센스](https://github.com/ohah/cheolsu-proxy/blob/master/LICENSE-MIT)
- [Apache 2.0 라이센스](https://github.com/ohah/cheolsu-proxy/blob/master/LICENSE-APACHE)
