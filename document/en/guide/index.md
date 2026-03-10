# Guide

Cheolsu Proxy is an HTTP/HTTPS debugging proxy built with Rust and Tauri. It sits between your browser or application and the server, allowing you to monitor and analyze network traffic in real time. It's useful for API development, frontend debugging, mobile app testing, and more.

## Getting Started

### [Installation](./installation.md)

How to download and install Cheolsu Proxy on your system. Cheolsu Proxy supports macOS and provides three interfaces: Desktop GUI, TUI, and MCP Server.

### [Basic Usage](./basic-usage.md)

Learn the basics of starting the proxy and capturing traffic. If you're new to Cheolsu Proxy, start here.

### [SSL Certificates](./ssl-certificates.md)

To capture HTTPS traffic, you need to install and trust Cheolsu Proxy's CA certificate on your system. This guide covers platform-specific installation steps.

## Core Concepts

### [Proxying](./proxying.md)

How Cheolsu Proxy works as a proxy, system proxy configuration, manual proxy setup, upstream proxy, and the daemon architecture.

### [Recording](./recording.md)

Traffic capture, table/tree views, request/response details, HAR export, and more.

### [Sessions](./sessions.md)

Save and load captured traffic as sessions for later review.

## [Troubleshooting](./troubleshooting.md)

Solutions for common issues including proxy startup failures, website access problems, HTTPS certificate issues, and performance problems.

## Advanced Features

Beyond capturing traffic, Cheolsu Proxy offers powerful tools for manipulating and automating requests and responses.

- [Cheolsu-Query](/en/features/cheolsu-query) - Network request filtering query language
- [Intercept Rules](/en/features/intercept-rules) - Block, modify, or redirect requests
- [Scripting](/en/features/scripting) - Automate traffic manipulation with JavaScript/TypeScript
- [WebSocket](/en/features/websocket) - WebSocket traffic monitoring and injection
- [Server Replay](/en/features/server-replay) - Cache and reuse captured responses
- [MCP Server](/en/features/mcp-server) - AI assistant integration

## Security Considerations

**Use only for development and testing purposes.** Cheolsu Proxy is a powerful tool that can intercept and modify network traffic. Please observe the following:

1. **No malicious use** — Do not use Cheolsu Proxy for data tampering, unauthorized interception, or any malicious purpose.
2. **Corporate compliance** — When using in a corporate environment, obtain IT department approval and comply with company policies.
3. **Privacy protection** — Do not intercept traffic containing sensitive personal information. Ensure sensitive data is not recorded in logs.
4. **Legal responsibility** — Users are responsible for all legal consequences of using Cheolsu Proxy.

---

**Get Started**: Begin with [Installation](./installation.md), or if already installed, jump to [Basic Usage](./basic-usage.md).
