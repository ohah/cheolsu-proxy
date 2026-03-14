# Proxy Connection Feature Roadmap

> A prioritized list of additional proxy server connection features, compared against existing tools like Charles Proxy and mitmproxy.

## Currently Implemented Connection Features

| Feature                          | Description                                                  | Comparison                    |
| -------------------------------- | ------------------------------------------------------------ | ----------------------------- |
| HTTP/HTTPS Proxy                 | MITM-based traffic interception                              | On par with Charles/mitmproxy |
| SOCKS5 Proxy                     | Full implementation with RFC 1929 auth                       | On par with Charles           |
| Upstream Proxy                   | HTTP/HTTPS/SOCKS upstream with auth and bypass               | On par with Charles           |
| TLS 1.0/1.1 Legacy               | Hybrid OpenSSL/rustls handler                                | **Unique differentiator**     |
| WebSocket Capture                | Bidirectional monitoring/injection, Socket.IO/MQTT detection | Superior to Charles           |
| Connection Strategy (Eager/Lazy) | Background server connection after ClientHello analysis      | **Unique differentiator**     |
| Network Throttling               | Token Bucket based, GPRS~WiFi presets, conditional throttle  | Superior to Charles           |
| Connection Limit                 | Semaphore-based max connection control                       | Basic feature                 |
| SSE Streaming Capture            | Backend complete (parsing/scripting hooks/protocol), GUI pending | On par with mitmproxy      |
| gRPC Traffic Decoding            | Frame parsing, metadata, status codes, .proto field mapping  | On par with mitmproxy         |
| Connection Monitoring            | Backend metrics collection/aggregation/query complete, GUI pending | On par with Charles       |

---

## Implemented Features

### 1. SSE (Server-Sent Events) Streaming Capture — Backend Complete, GUI Pending

**Backend (Complete):**

- Auto-detect `text/event-stream` Content-Type
- Real-time SSE event parsing (`event`, `data`, `id`, `retry` fields)
- Scripting hook: `cheolsu.onSSEMessage` (modify/block events)
- SSE connection state events (Connected/Disconnected)
- DaemonMessage protocol ready for GUI communication

**GUI (Not Implemented):**

- Chronological event list display (WebSocket message view-like UX)
- JSON `data` field auto-formatting
- Event type filtering
- SSE connection list management UI

---

### 2. gRPC Traffic Decoding — Mostly Implemented

**Implemented:**

- Detect `application/grpc`, `application/grpc+proto` Content-Types
- Parse gRPC frames (Compressed-Flag + Message-Length + Message structure)
- Display gRPC metadata (service name, method name, status code)
- Map gRPC status codes (0-16 complete, `grpc-status` header)
- Auto-decode Protobuf messages (wire format tree view)
- `.proto` file loading for field name mapping (prost_reflect + protox based)

**Not Implemented:**

- gRPC-Web support (browser-based gRPC clients)
- Streaming type runtime classification (Unary/Server/Client/Bidirectional — enum defined only)

---

### 3. Connection Status Monitoring / Statistics — Backend Complete, GUI Pending

**Backend (Complete):**

- MetricsCollector: Atomic counter-based real-time metrics (active requests, total requests, bytes sent/received, TLS success/failure, connection failures, timeouts)
- MetricsAggregator: Per-domain statistics (request count, error count, response time, bytes), recent error list (max 100)
- Protocol commands: `GetMetrics`, `GetDomainStats`, `GetRecentErrors`

**Not Implemented:**

- GUI dashboard tab (time-series charts + summary tables)
- Histogram (TLS handshake time distribution, response time distribution)
- Connection pool status breakdown (idle / in-use / waiting)
- Connection reuse rate (Keep-Alive efficiency)

---

## Planned Features

## Tier 2 — Medium-term (Enterprise Environment Support)

### 4. Proxy Chaining (Multi-hop Proxy)

Configure chains that route traffic through multiple proxies sequentially. Supports corporate network structures like internal proxy → external proxy → internet.

**Scope:**

- Define proxy chains (ordered list of proxies)
- Apply different chains per domain/rule
- Monitor proxy health within chains
- Mixed protocol support (HTTP → SOCKS5 → HTTP, etc.)
- Extend existing UpstreamProxyConfig structure

---

### 5. PAC (Proxy Auto-Configuration) Support

Parse/execute PAC files for conditional automatic proxy selection. Integrates with system PAC settings in enterprise environments.

**Scope:**

- Load PAC files (local file / URL)
- Execute `FindProxyForURL(url, host)` JavaScript function (using Deno Core)
- Route based on PAC results (`DIRECT`, `PROXY`, `SOCKS`)
- Auto-detect system PAC settings (macOS, Windows)
- PAC test tool (input URL → see which proxy is selected)

---

### 6. TCP Keep-Alive / Connection Pool Tuning

Fine-grained settings for server-side connection Keep-Alive and connection pool behavior.

**Scope:**

- Global and per-domain configuration
- Connection pool size (max idle connections)
- Idle timeout (auto-close unused connections)
- Keep-Alive interval and probe count
- Max connection lifetime
- DNS TTL respect

---

## Tier 3 — Long-term (Differentiation / Completeness)

### 7. DNS-over-HTTPS (DoH) / DNS-over-TLS (DoT)

Encrypt DNS queries at the proxy level.

### 8. HTTP/2 Multiplexing Optimization

Optimize HTTP/2 stream multiplexing for same-host connections.

### 9. Happy Eyeballs (IPv6 Dual Stack)

Implement RFC 8305 Happy Eyeballs v2 for simultaneous IPv4/IPv6 connection attempts.

### 10. Reverse Proxy Mode

Operate as a reverse proxy in front of specific backend servers for API development/testing.

---

## Priority Summary

| Priority     | Feature                     | Difficulty | User Impact | Status                           |
| ------------ | --------------------------- | ---------- | ----------- | -------------------------------- |
| ~~Tier 1-1~~ | ~~SSE Streaming Capture~~   | Medium     | Very High   | ⚠️ Backend complete, GUI pending |
| ~~Tier 1-2~~ | ~~gRPC Traffic Decoding~~   | Medium     | High        | ✅ Mostly implemented            |
| ~~Tier 1-3~~ | ~~Connection Monitoring~~   | Medium     | High        | ⚠️ Backend complete, GUI pending |
| Tier 2-1     | Proxy Chaining              | High       | Medium      | 📋 Planned                      |
| Tier 2-2     | PAC File Support            | Medium     | Medium      | 📋 Planned                      |
| Tier 2-3     | Connection Pool Tuning      | Low        | Low         | 📋 Planned                      |
| Tier 3-1     | DNS-over-HTTPS              | Medium     | Low         | 📋 Planned                      |
| Tier 3-2     | HTTP/2 Optimization         | High       | Low         | 📋 Planned                      |
| Tier 3-3     | Happy Eyeballs              | Medium     | Low         | 📋 Planned                      |
| Tier 3-4     | Reverse Proxy Mode          | High       | Low         | 📋 Planned                      |
