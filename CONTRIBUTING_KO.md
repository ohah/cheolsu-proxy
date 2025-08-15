# MITM 프록시 기여하기

이 프로젝트를 개선하는 데 도움을 주시는 커뮤니티의 기여를 환영합니다. 버그 수정, 새로운 기능 추가, 문서 개선 등에 관심이 있으시다면 참여할 수 있는 다양한 방법이 있습니다.

## 기여 방법
이 프로젝트에 기여하기 시작하는 몇 가지 단계입니다:

* 저장소를 포크하고 로컬 머신에 클론합니다
* 변경사항을 위한 새 브랜치를 생성합니다
* 변경사항을 만들고 철저히 테스트합니다
* 설명적인 커밋 메시지와 함께 변경사항을 커밋합니다
* 변경사항을 포크에 푸시하고 풀 리퀘스트를 제출합니다

작은 버그 수정부터 주요한 새로운 기능까지, 어떤 규모의 기여든 감사합니다. 하고 싶은 변경사항에 대해 확신이 서지 않는다면, 먼저 이슈를 열어서 유지보수자들과 논의해보세요.

## 개발 환경 설정

### Rust 설치

1. **Rust 설치**
   ```bash
   # macOS/Linux
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Windows
   # https://rustup.rs/ 에서 rustup-init.exe 다운로드 후 실행
   ```

2. **설치 확인**
   ```bash
   rustc --version
   cargo --version
   ```

3. **개발 도구 설치**
   ```bash
   # 코드 포맷팅
   rustup component add rustfmt
   
   # 린터
   rustup component add clippy
   ```

### Tauri UI로 UI 기여하기

- 필요한 도구 설치
```bash
cargo install
npm install

```

- 개발 시작
```bash
npm run tauri dev
```

- 패키징 및 릴리즈
```bash
npm run tauri build
```


## 테스트 요청 생성

* HTTP 서버와 클라이언트를 설치합니다.
  ```bash
  cargo install echo-server xh
  ```
* HTTP 서버를 실행합니다.
  ```bash
  echo-server
  ```
* HTTP 클라이언트를 실행합니다.
  ```bash
  xh --proxy http:http://127.0.0.1:8100 OPTIONS  http://127.0.0.1:8080
  xh --proxy http:http://127.0.0.1:8100 GET  http://127.0.0.1:8080
  xh --proxy http:http://127.0.0.1:8100 POST  http://127.0.0.1:8080
  xh --proxy http:http://127.0.0.1:8100 PUT  http://127.0.0.1:8080
  xh --proxy http:http://127.0.0.1:8100 DELETE  http://127.0.0.1:8080
  xh --proxy http:http://127.0.0.1:8100 HEAD  http://127.0.0.1:8080
  xh --proxy http:http://127.0.0.1:8100 TRACE  http://127.0.0.1:8080
  xh --proxy http:http://127.0.0.1:8100 CONNECT  http://127.0.0.1:8080
  xh --proxy http:http://127.0.0.1:8100 PATCH  http://127.0.0.1:8080
  ```
