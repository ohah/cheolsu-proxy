# Proxy Connection Feature Roadmap

> A prioritized list of additional proxy server connection features, compared against existing tools like Charles Proxy and mitmproxy.

## Currently Implemented Connection Features

| Feature                          | Description                                                        | Comparison                    |
| -------------------------------- | ------------------------------------------------------------------ | ----------------------------- |
| HTTP/HTTPS Proxy                 | MITM-based traffic interception                                    | On par with Charles/mitmproxy |
| SOCKS5 Proxy                     | Full implementation with RFC 1929 auth                             | On par with Charles           |
| Upstream Proxy                   | HTTP/HTTPS/SOCKS upstream with auth and bypass                     | On par with Charles           |
| TLS 1.0/1.1 Legacy               | Hybrid OpenSSL/rustls handler                                      | **Unique differentiator**     |
| WebSocket Capture                | Bidirectional monitoring/injection, Socket.IO/MQTT detection       | Superior to Charles           |
| SSE Streaming Capture            | `text/event-stream` auto-detection, event parsing, scripting hooks | **Unique differentiator**     |
| gRPC Traffic Decoding            | gRPC frame parsing, metadata extraction, Protobuf decoding         | On par with mitmproxy         |
| Connection Monitoring            | Global/per-domain metrics, error tracking, real-time aggregation   | Superior to Charles           |
| Connection Strategy (Eager/Lazy) | Background server connection after ClientHello analysis            | **Unique differentiator**     |
| Network Throttling               | Token Bucket based, GPRS~WiFi presets                              | On par with Charles           |
| Connection Limit                 | Semaphore-based max connection control                             | Basic feature                 |

---

## Tier 1 — Medium-term (Enterprise Environment Support)

### 1. Proxy Chaining (Multi-hop Proxy)

Configure chains that route traffic through multiple proxies sequentially. Supports corporate network structures like internal proxy → external proxy → internet.

**Scope:**

- Define proxy chains (ordered list of proxies)
- Apply different chains per domain/rule
- Monitor proxy health within chains
- Mixed protocol support (HTTP → SOCKS5 → HTTP, etc.)
- Extend existing UpstreamProxyConfig structure

---

### 2. PAC (Proxy Auto-Configuration) Support

Parse/execute PAC files for conditional automatic proxy selection. Integrates with system PAC settings in enterprise environments.

**Scope:**

- Load PAC files (local file / URL)
- Execute `FindProxyForURL(url, host)` JavaScript function (using Deno Core)
- Route based on PAC results (`DIRECT`, `PROXY`, `SOCKS`)
- Auto-detect system PAC settings (macOS, Windows)
- PAC test tool (input URL → see which proxy is selected)

---

### 3. TCP Keep-Alive / Connection Pool Tuning

Fine-grained settings for server-side connection Keep-Alive and connection pool behavior.

**Scope:**

- Global and per-domain configuration
- Connection pool size (max idle connections)
- Idle timeout (auto-close unused connections)
- Keep-Alive interval and probe count
- Max connection lifetime
- DNS TTL respect

---

## Tier 2 — Long-term (Differentiation / Completeness)

### 4. DNS-over-HTTPS (DoH) / DNS-over-TLS (DoT)

Encrypt DNS queries at the proxy level.

### 5. HTTP/2 Multiplexing Optimization

Optimize HTTP/2 stream multiplexing for same-host connections.

### 6. Happy Eyeballs (IPv6 Dual Stack)

Implement RFC 8305 Happy Eyeballs v2 for simultaneous IPv4/IPv6 connection attempts.

### ~~7. Reverse Proxy Mode~~ ✅

~~Operate as a reverse proxy in front of specific backend servers for API development/testing.~~

Implemented. Host header based backend routing, virtual host pattern matching, Host header rewriting. See [Reverse Proxy](./reverse-proxy.md) for details.

---

## Priority Summary

| Priority     | Feature                | Difficulty | User Impact | Status       |
| ------------ | ---------------------- | ---------- | ----------- | ------------ |
| **Tier 1-1** | Proxy Chaining         | High       | Medium      | 📋 Planned   |
| **Tier 1-2** | PAC File Support       | Medium     | Medium      | 📋 Planned   |
| **Tier 1-3** | Connection Pool Tuning | Low        | Low         | 📋 Planned   |
| Tier 2-1     | DNS-over-HTTPS         | Medium     | Low         | 📋 Planned   |
| Tier 2-2     | HTTP/2 Optimization    | High       | Low         | 📋 Planned   |
| Tier 2-3     | Happy Eyeballs         | Medium     | Low         | 📋 Planned   |
| ~~Tier 2-4~~ | ~~Reverse Proxy Mode~~ | ~~High~~   | ~~Low~~     | ✅ Completed |
