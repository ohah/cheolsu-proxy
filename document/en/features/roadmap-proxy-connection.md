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
| Network Throttling               | Token Bucket based, GPRS~WiFi presets                        | On par with Charles           |
| Connection Limit                 | Semaphore-based max connection control                       | Basic feature                 |

---

## Tier 1 — Immediate (High Demand + Leverages Existing Infrastructure)

### 1. SSE (Server-Sent Events) Streaming Capture

Most AI streaming APIs, including LLM APIs (Claude, ChatGPT, Gemini, etc.), use SSE. This is a core feature for proxy debugging tools in the AI era.

**Background:**

SSE is a unidirectional protocol that streams events from server to client using the `text/event-stream` Content-Type over standard HTTP responses. Unlike WebSocket, it operates over plain HTTP, making interception straightforward. However, parsing events in real-time and displaying them individually requires dedicated implementation.

**Scope:**

- Auto-detect `text/event-stream` Content-Type
- Real-time SSE event parsing (`event`, `data`, `id`, `retry` fields)
- Chronological event list display (UX similar to WebSocket message view)
- Auto-format JSON `data` fields (LLM streaming response decoding)
- Filter by event type
- Scripting hook: `cheolsu.onSSEMessage` (modify/block events)
- SSE connection list management (active/closed status)
- Real-time event count updates during streaming

**Use Case:**

```
# Debugging Claude API streaming response
POST https://api.anthropic.com/v1/messages
Content-Type: application/json
→ Response: text/event-stream

event: content_block_delta
data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}

event: message_stop
data: {"type":"message_stop"}
```

**Note:** Implementation cost is minimized by reusing WebSocket capture infrastructure (WebSocketRegistry, message viewer).

---

### 2. gRPC Traffic Decoding

gRPC is the de facto standard communication protocol in microservice architectures. By combining the existing HTTP/2 feature with the Protobuf decoder, gRPC traffic can be displayed in a structured format.

**Background:**

gRPC operates over HTTP/2 and uses Protocol Buffers as its serialization format. Cheolsu Proxy already has an HTTP/2 feature (`http2`) and a Protobuf wire-type based decoder, so the key is combining them in the gRPC context.

**Scope:**

- Detect `application/grpc`, `application/grpc+proto`, `application/grpc-web` Content-Types
- Parse gRPC frames (Compressed-Flag + Message-Length + Message structure)
- Display gRPC metadata (service name, method name, status code)
- Map gRPC status codes (`grpc-status` header → human-readable names)
- Distinguish Unary / Server Streaming / Client Streaming / Bidirectional Streaming
- Auto-decode Protobuf messages (using existing decoder)
- gRPC-Web support (browser-based gRPC clients)
- Optional `.proto` file loading for field name mapping

**gRPC Status Codes:**

| Code | Name              | Description          |
| ---- | ----------------- | -------------------- |
| 0    | OK                | Success              |
| 1    | CANCELLED         | Cancelled by client  |
| 2    | UNKNOWN           | Unknown error        |
| 3    | INVALID_ARGUMENT  | Invalid argument     |
| 4    | DEADLINE_EXCEEDED | Timeout              |
| 5    | NOT_FOUND         | Resource not found   |
| 12   | UNIMPLEMENTED     | Unimplemented method |
| 13   | INTERNAL          | Internal error       |
| 14   | UNAVAILABLE       | Service unavailable  |

**Use Case:**

```
# gRPC Unary call
POST /grpc.health.v1.Health/Check HTTP/2
Content-Type: application/grpc

# Decoded display:
Service: grpc.health.v1.Health
Method: Check
Status: 0 (OK)
Request:  { 1: "my-service" }
Response: { 1: 1 }  # SERVING

# With .proto file loaded:
Request:  { service: "my-service" }
Response: { status: SERVING }
```

---

### 3. Connection Status Monitoring / Statistics

Provides real-time status and statistics for network connections passing through the proxy. Gives visibility into "what is happening right now" as a debugging tool.

**Scope:**

#### Real-time Metrics

- Active connection count (HTTP, WebSocket, SSE, tunnels)
- Connection pool status (idle / in-use / waiting)
- TLS handshake time distribution
- Requests/responses per second (RPS)
- Bytes transferred (upload/download)

#### Per-Domain Statistics

- Request count, average response time, error rate per domain
- Connection reuse rate per domain (Keep-Alive efficiency)
- Top N slowest domains
- Top N highest traffic domains

#### Error Tracking

- Connection failures (TCP, TLS handshake, timeout)
- TLS certificate error details (expired, host mismatch, etc.)
- Upstream proxy connection failures
- Client/server disconnections

#### Time-Series Charts

- Traffic trends over time (request count, bytes)
- Response time trends (average, p95, p99)
- Error rate trends

**Data Collection Points:**

```rust
// Add metrics collector to ProxyContext
pub struct ConnectionMetrics {
    pub active_connections: AtomicU64,
    pub total_requests: AtomicU64,
    pub total_bytes_sent: AtomicU64,
    pub total_bytes_received: AtomicU64,
    pub tls_handshake_duration: Histogram,
    pub response_duration: Histogram,
    pub domain_stats: DashMap<String, DomainMetrics>,
    pub error_counts: DashMap<ErrorCategory, AtomicU64>,
}
```

**Interfaces:**

- **GUI**: Dashboard tab (real-time charts + summary tables)
- **TUI**: Top status bar + statistics tab
- **MCP**: Query AI assistant with "What's the slowest API?"
- **CLI**: Periodic output with `--stats` flag

---

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

| Priority     | Feature                     | Difficulty | User Impact | Status     |
| ------------ | --------------------------- | ---------- | ----------- | ---------- |
| **Tier 1-1** | SSE Streaming Capture       | Medium     | Very High   | 📋 Planned |
| **Tier 1-2** | gRPC Traffic Decoding       | Medium     | High        | 📋 Planned |
| **Tier 1-3** | Connection Monitoring/Stats | Medium     | High        | 📋 Planned |
| Tier 2-1     | Proxy Chaining              | High       | Medium      | 📋 Planned |
| Tier 2-2     | PAC File Support            | Medium     | Medium      | 📋 Planned |
| Tier 2-3     | Connection Pool Tuning      | Low        | Low         | 📋 Planned |
| Tier 3-1     | DNS-over-HTTPS              | Medium     | Low         | 📋 Planned |
| Tier 3-2     | HTTP/2 Optimization         | High       | Low         | 📋 Planned |
| Tier 3-3     | Happy Eyeballs              | Medium     | Low         | 📋 Planned |
| Tier 3-4     | Reverse Proxy Mode          | High       | Low         | 📋 Planned |
