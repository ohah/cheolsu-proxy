# Cheolsu Proxy 문서

이 디렉토리는 Cheolsu Proxy 프로젝트의 문서 사이트 소스입니다.
[Rspress](https://rspress.dev/)로 빌드되며 GitHub Pages에 배포됩니다.

## 문서 구조

```
document/
├── ko/              # 한국어 문서 (기본 언어)
│   ├── features/    # 기능 문서
│   ├── guide/       # 사용자 가이드
│   ├── contributing/ # 기여 가이드
│   └── releases/    # 릴리즈 노트
├── en/              # 영문 문서
│   ├── features/
│   ├── guide/
│   ├── contributing/
│   └── releases/
├── public/          # 정적 에셋 (이미지 등)
└── rspress.config.ts # Rspress 설정
```

## 로컬 개발

```bash
bun run --filter cheolsu-proxy-document dev
```

## 빌드

```bash
bun run --filter cheolsu-proxy-document build
```

## 배포

`main` 브랜치에 `document/**` 변경이 푸시되면 CI가 자동으로 GitHub Pages에 배포합니다.

**저장소 설정**: Settings > Pages > Source: **GitHub Actions**
