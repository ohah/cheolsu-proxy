import { describe, test, expect } from "bun:test";
import { render, screen } from "@testing-library/react";

import { TransactionCertificate } from "./transaction-certificate";
import type { ServerCertInfo } from "@/entities/proxy/model/types";

const createCert = (overrides: Partial<ServerCertInfo> = {}): ServerCertInfo => ({
  subject_cn: "example.com",
  issuer_cn: "Test CA",
  organization: "Test Org",
  sans_dns: ["example.com", "*.example.com"],
  sans_ip: ["127.0.0.1"],
  not_before: "2024-01-01T00:00:00Z",
  not_after: "2030-12-31T23:59:59Z",
  serial_number: "AB:CD:EF:01:23:45",
  fingerprint_sha256: "AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99",
  is_ca: false,
  negotiated_alpn: "h2",
  chain_length: 3,
  ...overrides,
});

describe("TransactionCertificate", () => {
  test("serverCert가 null일 때 HTTP connection 메시지 표시", () => {
    render(<TransactionCertificate serverCert={null} />);
    expect(screen.getByText("HTTP connection (no TLS)")).toBeDefined();
  });

  test("serverCert가 undefined일 때 HTTP connection 메시지 표시", () => {
    render(<TransactionCertificate />);
    expect(screen.getByText("HTTP connection (no TLS)")).toBeDefined();
  });

  test("serverCert가 있을 때 Subject CN 표시", () => {
    render(
      <TransactionCertificate serverCert={createCert({ subject_cn: "test-cn.example.com" })} />,
    );
    expect(screen.getByText("test-cn.example.com")).toBeDefined();
  });

  test("만료된 인증서에 Expired 텍스트 표시", () => {
    const expiredCert = createCert({
      not_after: "2020-01-01T00:00:00Z",
    });
    render(<TransactionCertificate serverCert={expiredCert} />);
    expect(screen.getByText("Expired")).toBeDefined();
  });

  test("유효한 인증서에 Valid 텍스트 표시", () => {
    const validCert = createCert({
      not_after: "2030-12-31T23:59:59Z",
    });
    render(<TransactionCertificate serverCert={validCert} />);
    expect(screen.getByText("Valid")).toBeDefined();
  });

  test("SAN DNS 목록 표시", () => {
    render(<TransactionCertificate serverCert={createCert()} />);
    expect(screen.getByText("*.example.com")).toBeDefined();
  });

  test("SAN IP 목록 표시", () => {
    render(<TransactionCertificate serverCert={createCert()} />);
    expect(screen.getByText("127.0.0.1")).toBeDefined();
  });

  test("Fingerprint 표시", () => {
    render(<TransactionCertificate serverCert={createCert()} />);
    expect(screen.getByText("AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99")).toBeDefined();
  });

  test("Serial Number 표시", () => {
    render(<TransactionCertificate serverCert={createCert()} />);
    expect(screen.getByText("AB:CD:EF:01:23:45")).toBeDefined();
  });

  test("Issuer CN 표시", () => {
    render(<TransactionCertificate serverCert={createCert()} />);
    expect(screen.getByText("Test CA")).toBeDefined();
  });
});
