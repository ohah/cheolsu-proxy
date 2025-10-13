# 🚀 Pull Request

## 📚 PR 제목

```
feat: RSPress 기반 다국어 문서 사이트 구축 및 GitHub Pages 자동 배포 설정
```

## 📝 PR 설명

### 🎯 개요

**빌드와 배포 확인을 위한 테스트 PR입니다.**

Cheolsu Proxy 프로젝트를 위한 공식 문서 사이트를 RSPress로 구축하고, GitHub Pages 자동 배포를 설정했습니다. 이 PR은 빌드 프로세스와 배포 파이프라인이 정상적으로 작동하는지 확인하기 위한 목적으로 생성되었습니다.

### ✨ 주요 기능

#### 🌐 다국어 지원

- **한국어 (기본)**: `https://ohah.github.io/cheolsu-proxy/`
- **영어**: `https://ohah.github.io/cheolsu-proxy/en/`
- 언어별 네비게이션 및 사이드바 구성

#### 📖 문서 구조

- **사용자 가이드**: 시작하기, 주요 기능, 인증서 설정
- **기여자 가이드**: 기여 방법, 개발 환경 설정, 프로젝트 구조, 테스트
- **기능 문서**: TLS 1.0/1.1 지원 등

#### 🚀 자동 배포

- GitHub Actions를 통한 자동 빌드 및 배포
- `main`/`master` 브랜치 push 시 자동 트리거
- `docs/` 디렉토리 변경사항 감지

### 🏗️ 기술 스택

- **RSPress**: 정적 사이트 생성기
- **pnpm 워크스페이스**: 모노레포 구조
- **GitHub Actions**: CI/CD 파이프라인
- **GitHub Pages**: 호스팅

### 📁 프로젝트 구조

```
cheolsu-proxy/
├── docs/                    # RSPress 문서 사이트
│   ├── ko/                 # 한국어 문서
│   ├── en/                 # 영어 문서
│   ├── assets/             # 이미지 및 에셋
│   └── rspress.config.ts   # RSPress 설정
├── .github/workflows/       # GitHub Actions
└── package.json            # pnpm 워크스페이스 설정
```

### 🔧 주요 변경사항

#### 1. pnpm 워크스페이스 설정

- 루트 `package.json`에 워크스페이스 구성
- `tauri-ui`, `docs` 프로젝트 통합 관리
- 루트에서 `pnpm dev` 실행 가능

#### 2. RSPress 문서 사이트

- 다국어 지원 설정 (한국어/영어)
- 네비게이션 및 사이드바 구성
- GitHub Pages 배포를 위한 `base` 경로 설정
- 로컬 개발과 프로덕션 환경 분리

#### 3. 문서 콘텐츠 마이그레이션

- 기존 `README.md`, `CONTRIBUTING_KO.md` 내용을 구조화된 문서로 변환
- 한국어/영어 번역본 제공
- 스크린샷 및 로고 에셋 추가

#### 4. GitHub Actions 자동 배포

- RSPress 빌드 및 GitHub Pages 배포 워크플로우
- `docs/` 디렉토리 변경사항 감지
- 자동 빌드 및 배포 파이프라인

#### 5. 개발 환경 개선

- 빌드 아티팩트 gitignore 처리
- Cursor Rules에 다국어 문서 동기화 규칙 추가
- 파일별 커밋으로 변경사항 추적성 향상

### 🎨 UI/UX 특징

- 다크/라이트 모드 지원
- 반응형 디자인
- 검색 기능
- 언어 전환 기능
- GitHub 링크 통합

### 📊 커밋 히스토리

```
ff5b0e8 docs: 다국어 문서 동기화 규칙 추가
d69402e chore: RSPress 빌드 디렉토리 gitignore 추가
77cdfb7 feat: 다국어 설정 및 네비게이션 복원
b04bbb5 fix: RSPress root 경로 설정 수정
8e6fdb3 fix: RSPress 다국어 라우팅 설정 수정
d1d5477 docs: 루트 문서 및 설정 업데이트
c1bd36f feat: GitHub Actions 자동 배포 설정
4c22393 feat: 영어 문서 콘텐츠 추가
5feb352 feat: 한국어 문서 콘텐츠 추가
5c2e6c3 feat: 문서 사이트 에셋 파일 추가
808762d feat: RSPress 문서 프로젝트 기본 설정
1cbf935 feat: pnpm 워크스페이스 설정
```

### 🔗 관련 링크

- **문서 사이트**: https://ohah.github.io/cheolsu-proxy/
- **RSPress**: https://rspress.rs/
- **GitHub Actions**: `.github/workflows/deploy-docs.yml`

### ✅ 테스트 완료

- [x] 로컬 개발 서버 정상 작동 (`http://localhost:3000/`)
- [x] 다국어 라우팅 정상 작동
- [x] 네비게이션 및 사이드바 정상 작동
- [x] GitHub Actions 워크플로우 설정 완료
- [x] 빌드 아티팩트 gitignore 처리 완료

### 🚀 배포 후 예상 결과

- GitHub Pages에 자동 배포
- `https://ohah.github.io/cheolsu-proxy/`에서 접근 가능
- 다국어 지원으로 국제 사용자 접근성 향상
- 구조화된 문서로 사용자 경험 개선

---

**Breaking Changes**: 없음  
**Dependencies**: RSPress 추가 (dev dependency)  
**Documentation**: 완전히 새로운 문서 사이트 구축

## 📋 체크리스트

- [x] 코드 리뷰 완료
- [x] 테스트 통과
- [x] 문서 업데이트
- [x] Breaking Changes 없음
- [x] 의존성 변경사항 확인
