# AGENTS.md

Rust 기반 MITM 프록시. Tauri 데스크톱 앱, React UI.

```
Tauri UI (React/FSD) ↔ Tauri IPC ↔ Rust Proxy Core (proxyapi_v2) ↔ TLS/HTTP
```

## Docs

- [문서 루트](document/README.md) — 문서 목차 및 작성 가이드
- [한국어 가이드](document/ko/guide/index.md) — 사용법, 트러블슈팅
- [기능: TLS 1.0/1.1](document/features/TLS_1_0_1_1_SUPPORT.md) — 하이브리드 TLS, PKCS12
- [기여: 개발 환경](document/ko/contributing/development-setup.md) — 빌드/실행
- [기여: 코드 구조](document/ko/contributing/code-structure.md) — 모노레포, 레이어
- [코드 규칙](.cursorrules) — Rust/TS 컨벤션, 문서·커밋 규칙
