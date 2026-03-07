# Contributing to the MITM Proxy

We welcome contributions from the community to help improve this project. Whether you're interested in fixing bugs, adding new features, or improving documentation, there are many ways to get involved.

## How to Contribute

Here are some steps to get started with contributing to this project:

- Fork the repository and clone it to your local machine
- Create a new branch for your changes
- Make your changes and test them thoroughly
- Commit your changes with a descriptive commit message
- Push your changes to your fork and submit a pull request

We appreciate contributions of any size, from small bug fixes to major new features. If you're unsure about a change you'd like to make, feel free to open an issue first to discuss it with the maintainers.

### 개발 환경 설정

1. **의존성 설치 (macOS)**

   ```bash
   brew install openssl@3 pkg-config
   ```

2. **인증서 생성**

   ```bash
   cd crates
   bash ../install_cer.sh
   ```

3. **CA 인증서 시스템 신뢰 등록**

   ```bash
   # macOS
   open proxyapi_v2/src/certificate_authority/cheolsu-proxy.cer
   # Keychain Access에서 "항상 신뢰"로 설정
   ```

4. **테스트 실행**

   ```bash
   PKG_CONFIG_PATH="/opt/homebrew/opt/openssl@3/lib/pkgconfig" cargo test -p proxyapi_v2 --lib
   ```

### Contribute to UI with Tauri UI

- start development

```bash
cd desktop
bun install
bun tauri dev
```

- package and release

```bash
cd desktop
bun tauri build
```

## Test request generation

- Install http server and client.
  ```bash
  cargo install echo-server xh
  ```
- Run http server.
  ```bash
  echo-server
  ```
- Run http client.
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
