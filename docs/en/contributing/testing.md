---
title: Testing
description: Cheolsu Proxy test writing and execution guide
---

# Testing

This guide explains Cheolsu Proxy's testing strategy and execution methods.

## Testing Strategy

### Test Pyramid

```
        E2E Tests (Few)
       /              \
   Integration Tests (Medium)
  /                      \
Unit Tests (Many)
```

- **Unit Tests**: Individual function/method testing
- **Integration Tests**: Inter-module interaction testing
- **E2E Tests**: Full application testing

## Rust Testing

### 1. Unit Tests

#### Basic Structure

```rust
// In src/lib.rs or each module file
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

#### Real Example

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

### 2. Integration Tests

#### Test Structure

```
tests/
├── common/
│   └── mod.rs                 # Common test utilities
├── integration_error_handling_tests.rs
├── logging_handler_error_tests.rs
├── openssl_ca.rs
├── rcgen_ca.rs
└── websocket.rs
```

#### Common Utilities

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

#### CA Tests

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

#### Proxy Integration Tests

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
    // Test client connection and requests
    // ...

    proxy_handle.abort();
}
```

### 3. Benchmark Tests

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

### 4. Test Execution

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run integration tests only
cargo test --test integration_test

# Run benchmarks
cargo bench

# Test coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Frontend Testing

### 1. Unit Tests (Jest + React Testing Library)

#### Test Setup

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

#### Component Tests

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

    // Test filtering logic
    expect(screen.getByText("GET")).toBeInTheDocument();
  });
});
```

#### Hook Tests

```typescript
// __tests__/hooks/useProxy.test.ts
import { renderHook, act } from "@testing-library/react";
import { useProxy } from "@/features/network-table/hooks/useProxy";

describe("useProxy", () => {
  it("should start and stop proxy", async () => {
    const { result } = renderHook(() => useProxy());

    // Check initial state
    expect(result.current.isRunning).toBe(false);

    // Start proxy
    await act(async () => {
      await result.current.startProxy();
    });

    expect(result.current.isRunning).toBe(true);

    // Stop proxy
    await act(async () => {
      await result.current.stopProxy();
    });

    expect(result.current.isRunning).toBe(false);
  });
});
```

### 2. Integration Tests

```typescript
// __tests__/integration/ProxyFlow.test.tsx
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "@/app/App";

describe("Proxy Flow Integration", () => {
  it("should handle complete proxy workflow", async () => {
    const user = userEvent.setup();
    render(<App />);

    // Start proxy
    const startButton = screen.getByText("Start Proxy");
    await user.click(startButton);

    // Check proxy status
    await waitFor(() => {
      expect(screen.getByText("Proxy Running")).toBeInTheDocument();
    });

    // Check request list
    await waitFor(() => {
      expect(screen.getByText("No requests yet")).toBeInTheDocument();
    });
  });
});
```

### 3. E2E Tests (Playwright)

```typescript
// e2e/proxy.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Cheolsu Proxy E2E", () => {
  test("should start proxy and capture requests", async ({ page }) => {
    // Launch app
    await page.goto("http://localhost:1420");

    // Start proxy
    await page.click('[data-testid="start-proxy"]');

    // Check proxy status
    await expect(page.locator('[data-testid="proxy-status"]')).toHaveText(
      "Running"
    );

    // Generate external request (actual HTTP request)
    await page.evaluate(() => {
      fetch("https://httpbin.org/get");
    });

    // Check if request was captured
    await expect(page.locator('[data-testid="request-list"]')).toContainText(
      "httpbin.org"
    );
  });
});
```

### 4. Test Execution

```bash
# Unit tests
npm test

# Integration tests
npm run test:integration

# E2E tests
npm run test:e2e

# Coverage
npm run test:coverage

# Watch mode
npm run test:watch
```

## Test Data Management

### 1. Mock Data

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
  // ... more mock data
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

### 2. Test Utilities

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

## Performance Testing

### 1. Load Testing

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

### 2. Memory Testing

```rust
// tests/memory_test.rs
use proxyapi_v2::certificate_authority::RcgenAuthority;
use std::time::Duration;

#[tokio::test]
async fn test_memory_usage() {
    let ca = RcgenAuthority::new();
    let initial_memory = get_memory_usage();

    // Generate large number of certificates
    for i in 0..1000 {
        let domain = format!("test{}.example.com", i);
        let _cert = ca.generate_certificate(&domain).await.unwrap();
    }

    let final_memory = get_memory_usage();
    let memory_increase = final_memory - initial_memory;

    // Check if memory increase is within reasonable range
    assert!(memory_increase < 100 * 1024 * 1024); // Less than 100MB
}

fn get_memory_usage() -> usize {
    // Memory usage measurement logic
    0
}
```

## Test Automation

### 1. CI/CD Pipeline

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

### 2. Test Reports

```bash
# Generate test results as HTML
cargo test --workspace -- --nocapture > test-results.txt

# Coverage report
cargo tarpaulin --out Html --output-dir coverage/

# Frontend coverage
cd tauri-ui && npm run test:coverage
```

## Testing Best Practices

### 1. Test Writing Principles

- **AAA Pattern**: Arrange, Act, Assert
- **Clear Test Names**: Clearly indicate what is being tested
- **Single Responsibility**: One test tests one functionality
- **Independence**: No dependencies between tests

### 2. Mock Usage

```rust
// Mock trait implementation
pub struct MockCertificateAuthority {
    pub should_fail: bool,
}

impl CertificateAuthority for MockCertificateAuthority {
    async fn generate_certificate(&self, domain: &str) -> Result<Certificate, Error> {
        if self.should_fail {
            return Err(Error::CertificateGenerationFailed);
        }

        // Return mock certificate
        Ok(Certificate::mock(domain))
    }
}
```

### 3. Test Data Isolation

```rust
// Use unique port for each test
fn get_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
```

This testing strategy ensures the quality and stability of Cheolsu Proxy.
