---
title: 테스트
description: Cheolsu Proxy 테스트 작성 및 실행 가이드
---

# 테스트

Cheolsu Proxy의 테스트 전략과 실행 방법을 설명합니다.

## 테스트 전략

### 테스트 피라미드

```
        E2E Tests (소수)
       /              \
   Integration Tests (중간)
  /                      \
Unit Tests (다수)
```

- **Unit Tests**: 개별 함수/메서드 테스트
- **Integration Tests**: 모듈 간 상호작용 테스트
- **E2E Tests**: 전체 애플리케이션 테스트

## Rust 테스트

### 1. 단위 테스트

#### 기본 구조

```rust
// src/lib.rs 또는 각 모듈 파일 내
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Given
        let input = "test_input";

        // When
        let result = function_to_test(input);

        // Then
        assert_eq!(result, expected_output);
    }
}
```

#### 실제 예제

```rust
// proxyapi_v2/src/certificate_authority/rcgen_authority.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_certificate() {
        // Given
        let ca = RcgenAuthority::new();
        let domain = "example.com";

        // When
        let cert = ca.generate_certificate(domain).unwrap();

        // Then
        assert_eq!(cert.domain(), domain);
        assert!(cert.not_after() > SystemTime::now());
    }

    #[test]
    fn test_generate_certificate_invalid_domain() {
        // Given
        let ca = RcgenAuthority::new();
        let invalid_domain = "";

        // When & Then
        assert!(ca.generate_certificate(invalid_domain).is_err());
    }
}
```

### 2. 통합 테스트

#### 테스트 구조

```
tests/
├── common/
│   └── mod.rs                 # 공통 테스트 유틸리티
├── integration_error_handling_tests.rs
├── logging_handler_error_tests.rs
├── openssl_ca.rs
├── rcgen_ca.rs
└── websocket.rs
```

#### 공통 유틸리티

```rust
// tests/common/mod.rs
use proxyapi_v2::certificate_authority::{CertificateAuthority, RcgenAuthority};
use std::time::Duration;

pub struct TestHelper;

impl TestHelper {
    pub fn create_test_ca() -> RcgenAuthority {
        RcgenAuthority::new()
    }

    pub fn create_test_domain() -> String {
        "test.example.com".to_string()
    }

    pub fn wait_for_condition<F>(mut condition: F, timeout: Duration) -> bool
    where
        F: FnMut() -> bool
    {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }
}
```

#### CA 테스트

```rust
// tests/rcgen_ca.rs
use proxyapi_v2::certificate_authority::{CertificateAuthority, RcgenAuthority};
use tests::common::TestHelper;

#[tokio::test]
async fn test_rcgen_ca_certificate_generation() {
    // Given
    let ca = TestHelper::create_test_ca();
    let domain = TestHelper::create_test_domain();

    // When
    let cert = ca.generate_certificate(&domain).await.unwrap();

    // Then
    assert_eq!(cert.domain(), domain);
    assert!(cert.not_after() > std::time::SystemTime::now());
}

#[tokio::test]
async fn test_rcgen_ca_pkcs12_generation() {
    // Given
    let ca = TestHelper::create_test_ca();
    let domain = TestHelper::create_test_domain();

    // When
    let identity = ca.gen_pkcs12_identity(&domain).await.unwrap();

    // Then
    assert!(!identity.is_empty());
}
```

#### 프록시 통합 테스트

```rust
// tests/proxy_integration.rs
use proxyapi_v2::proxy::{Proxy, ProxyBuilder};
use proxyapi_v2::certificate_authority::RcgenAuthority;
use std::net::{SocketAddr, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_proxy_http_forwarding() {
    // Given
    let test_server = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = test_server.local_addr().unwrap();

    let ca = RcgenAuthority::new();
    let proxy = ProxyBuilder::new()
        .listen_addr("127.0.0.1:0".parse().unwrap())
        .certificate_authority(Box::new(ca))
        .build()
        .unwrap();

    // When
    let proxy_handle = tokio::spawn(async move {
        proxy.run().await
    });

    // Then
    // 클라이언트 연결 및 요청 테스트
    // ...

    proxy_handle.abort();
}
```

### 3. 벤치마크 테스트

```rust
// benches/certificate_authorities.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use proxyapi_v2::certificate_authority::{CertificateAuthority, RcgenAuthority};

fn bench_certificate_generation(c: &mut Criterion) {
    let ca = RcgenAuthority::new();

    c.bench_function("generate_certificate", |b| {
        b.iter(|| {
            let domain = black_box("example.com");
            ca.generate_certificate(domain)
        })
    });
}

criterion_group!(benches, bench_certificate_generation);
criterion_main!(benches);
```

### 4. 테스트 실행

```bash
# 모든 테스트 실행
cargo test

# 특정 테스트 실행
cargo test test_name

# 통합 테스트만 실행
cargo test --test integration_test

# 벤치마크 실행
cargo bench

# 테스트 커버리지
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## 프론트엔드 테스트

### 1. 단위 테스트 (Jest + React Testing Library)

#### 테스트 설정

```typescript
// jest.config.js
module.exports = {
  preset: "ts-jest",
  testEnvironment: "jsdom",
  setupFilesAfterEnv: ["<rootDir>/src/setupTests.ts"],
  moduleNameMapping: {
    "^@/(.*)$": "<rootDir>/src/$1",
  },
  collectCoverageFrom: ["src/**/*.{ts,tsx}", "!src/**/*.d.ts", "!src/main.tsx"],
};
```

#### 컴포넌트 테스트

```typescript
// __tests__/components/NetworkTable.test.tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { NetworkTable } from "@/widgets/network-table/ui/NetworkTable";
import { proxyStore } from "@/shared/stores/proxy-store";

// Mock store
jest.mock("@/shared/stores/proxy-store", () => ({
  proxyStore: {
    getState: jest.fn(),
    subscribe: jest.fn(),
  },
}));

describe("NetworkTable", () => {
  const mockTransactions = [
    {
      id: "1",
      method: "GET",
      url: "https://example.com",
      status: 200,
      timestamp: new Date(),
    },
  ];

  beforeEach(() => {
    (proxyStore.getState as jest.Mock).mockReturnValue({
      requests: mockTransactions,
    });
  });

  it("renders transaction list", () => {
    render(<NetworkTable />);

    expect(screen.getByText("GET")).toBeInTheDocument();
    expect(screen.getByText("https://example.com")).toBeInTheDocument();
    expect(screen.getByText("200")).toBeInTheDocument();
  });

  it("filters transactions by method", () => {
    render(<NetworkTable />);

    const filterButton = screen.getByText("GET");
    fireEvent.click(filterButton);

    // 필터링 로직 테스트
    expect(screen.getByText("GET")).toBeInTheDocument();
  });
});
```

#### 훅 테스트

```typescript
// __tests__/hooks/useProxy.test.ts
import { renderHook, act } from "@testing-library/react";
import { useProxy } from "@/features/network-table/hooks/useProxy";

describe("useProxy", () => {
  it("should start and stop proxy", async () => {
    const { result } = renderHook(() => useProxy());

    // 초기 상태 확인
    expect(result.current.isRunning).toBe(false);

    // 프록시 시작
    await act(async () => {
      await result.current.startProxy();
    });

    expect(result.current.isRunning).toBe(true);

    // 프록시 중지
    await act(async () => {
      await result.current.stopProxy();
    });

    expect(result.current.isRunning).toBe(false);
  });
});
```

### 2. 통합 테스트

```typescript
// __tests__/integration/ProxyFlow.test.tsx
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "@/app/App";

describe("Proxy Flow Integration", () => {
  it("should handle complete proxy workflow", async () => {
    const user = userEvent.setup();
    render(<App />);

    // 프록시 시작
    const startButton = screen.getByText("Start Proxy");
    await user.click(startButton);

    // 프록시 상태 확인
    await waitFor(() => {
      expect(screen.getByText("Proxy Running")).toBeInTheDocument();
    });

    // 요청 목록 확인
    await waitFor(() => {
      expect(screen.getByText("No requests yet")).toBeInTheDocument();
    });
  });
});
```

### 3. E2E 테스트 (Playwright)

```typescript
// e2e/proxy.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Cheolsu Proxy E2E", () => {
  test("should start proxy and capture requests", async ({ page }) => {
    // 앱 실행
    await page.goto("http://localhost:1420");

    // 프록시 시작
    await page.click('[data-testid="start-proxy"]');

    // 프록시 상태 확인
    await expect(page.locator('[data-testid="proxy-status"]')).toHaveText(
      "Running"
    );

    // 외부 요청 생성 (실제 HTTP 요청)
    await page.evaluate(() => {
      fetch("https://httpbin.org/get");
    });

    // 요청이 캡처되었는지 확인
    await expect(page.locator('[data-testid="request-list"]')).toContainText(
      "httpbin.org"
    );
  });
});
```

### 4. 테스트 실행

```bash
# 단위 테스트
npm test

# 통합 테스트
npm run test:integration

# E2E 테스트
npm run test:e2e

# 커버리지
npm run test:coverage

# Watch 모드
npm run test:watch
```

## 테스트 데이터 관리

### 1. Mock 데이터

```typescript
// __tests__/mocks/mockData.ts
export const mockTransactions = [
  {
    id: "1",
    method: "GET",
    url: "https://example.com/api/users",
    status: 200,
    timestamp: new Date("2024-01-01T00:00:00Z"),
    requestHeaders: { "User-Agent": "Mozilla/5.0" },
    responseHeaders: { "Content-Type": "application/json" },
    requestBody: null,
    responseBody: '{"users": []}',
  },
  // ... 더 많은 mock 데이터
];

export const mockSessions = [
  {
    id: "session-1",
    name: "Test Session",
    createdAt: new Date("2024-01-01T00:00:00Z"),
    requestCount: 10,
  },
];
```

### 2. 테스트 유틸리티

```typescript
// __tests__/utils/testUtils.tsx
import { ReactElement } from "react";
import { render, RenderOptions } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const AllTheProviders = ({ children }: { children: React.ReactNode }) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

const customRender = (
  ui: ReactElement,
  options?: Omit<RenderOptions, "wrapper">
) => render(ui, { wrapper: AllTheProviders, ...options });

export * from "@testing-library/react";
export { customRender as render };
```

## 성능 테스트

### 1. 부하 테스트

```rust
// tests/load_test.rs
use proxyapi_v2::proxy::{Proxy, ProxyBuilder};
use proxyapi_v2::certificate_authority::RcgenAuthority;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_concurrent_connections() {
    // Given
    let ca = RcgenAuthority::new();
    let proxy = Arc::new(
        ProxyBuilder::new()
            .listen_addr("127.0.0.1:0".parse().unwrap())
            .certificate_authority(Box::new(ca))
            .build()
            .unwrap()
    );

    // When
    let mut handles = vec![];
    for i in 0..100 {
        let proxy_clone = proxy.clone();
        let handle = tokio::spawn(async move {
            let mut stream = TcpStream::connect(proxy_clone.listen_addr()).await.unwrap();
            let request = format!("GET /test{} HTTP/1.1\r\nHost: example.com\r\n\r\n", i);
            stream.write_all(request.as_bytes()).await.unwrap();

            let mut response = String::new();
            stream.read_to_string(&mut response).await.unwrap();

            assert!(response.contains("HTTP/1.1"));
        });
        handles.push(handle);
    }

    // Then
    for handle in handles {
        handle.await.unwrap();
    }
}
```

### 2. 메모리 테스트

```rust
// tests/memory_test.rs
use proxyapi_v2::certificate_authority::RcgenAuthority;
use std::time::Duration;

#[tokio::test]
async fn test_memory_usage() {
    let ca = RcgenAuthority::new();
    let initial_memory = get_memory_usage();

    // 대량의 인증서 생성
    for i in 0..1000 {
        let domain = format!("test{}.example.com", i);
        let _cert = ca.generate_certificate(&domain).await.unwrap();
    }

    let final_memory = get_memory_usage();
    let memory_increase = final_memory - initial_memory;

    // 메모리 증가량이 합리적인 범위 내인지 확인
    assert!(memory_increase < 100 * 1024 * 1024); // 100MB 이하
}

fn get_memory_usage() -> usize {
    // 메모리 사용량 측정 로직
    0
}
```

## 테스트 자동화

### 1. CI/CD 파이프라인

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy

      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: "18"
          cache: "npm"
          cache-dependency-path: tauri-ui/package-lock.json

      - name: Install dependencies
        run: |
          cd tauri-ui
          npm ci

      - name: Run Rust tests
        run: cargo test --workspace

      - name: Run frontend tests
        run: |
          cd tauri-ui
          npm test -- --coverage --watchAll=false

      - name: Run E2E tests
        run: |
          cd tauri-ui
          npm run test:e2e
```

### 2. 테스트 리포트

```bash
# 테스트 결과를 HTML로 생성
cargo test --workspace -- --nocapture > test-results.txt

# 커버리지 리포트
cargo tarpaulin --out Html --output-dir coverage/

# 프론트엔드 커버리지
cd tauri-ui && npm run test:coverage
```

## 테스트 모범 사례

### 1. 테스트 작성 원칙

- **AAA 패턴**: Arrange, Act, Assert
- **명확한 테스트 이름**: 무엇을 테스트하는지 명확히
- **단일 책임**: 하나의 테스트는 하나의 기능만 테스트
- **독립성**: 테스트 간 의존성 없음

### 2. Mock 사용

```rust
// Mock 트레이트 구현
pub struct MockCertificateAuthority {
    pub should_fail: bool,
}

impl CertificateAuthority for MockCertificateAuthority {
    async fn generate_certificate(&self, domain: &str) -> Result<Certificate, Error> {
        if self.should_fail {
            return Err(Error::CertificateGenerationFailed);
        }

        // Mock 인증서 반환
        Ok(Certificate::mock(domain))
    }
}
```

### 3. 테스트 데이터 격리

```rust
// 각 테스트마다 고유한 포트 사용
fn get_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
```

이러한 테스트 전략을 통해 Cheolsu Proxy의 품질과 안정성을 보장할 수 있습니다.
