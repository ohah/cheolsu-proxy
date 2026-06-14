# Release Notes

Version-by-version update history for Cheolsu Proxy.

## Latest Release

### v0.1.2 (2026-06-14)

Maintenance release with a full dependency upgrade, a large batch of security and stability bug fixes, and documentation accuracy improvements.

📦 [Download from GitHub Release](https://github.com/ohah/cheolsu-proxy/releases/tag/v0.1.2)

:::warning Unsigned Build
This build is not code-signed. After copying to `/Applications`, run the following in Terminal:

```bash
xattr -cr /Applications/Cheolsu\ Proxy.app
```

You only need to do this once.
:::

**Highlights**:

- Upgraded Rust/JavaScript dependencies across the board, including the scripting engine (deno_core/V8/oxc), MCP (rmcp), and Tauri.
- Fixed High-severity security issues (certificate SAN validation, mTLS fail-open) and hardened against panics, request-integrity bugs, and decompression bombs.
- Protected the daemon from runaway scripts (infinite loops / memory exhaustion) and cleaned up hook timeout / forced-termination races.
- Fixed an infinite-block regression when intercepting plaintext GET-over-CONNECT (ws://) and a slow-client DoS vector.
- Fixed many bugs across SSE/WebSocket UTF-8 handling, multipart size calculation, system proxy restore, and throttle preset persistence.
- Synced the MCP tools (48), CLI, scripting/SSE, and installation documentation with the actual implementation.

### v0.1.1 (2026-04-30)

Maintenance release with safer TLS auto-passthrough behavior, certificate-generation stability work, desktop/TUI/CLI usability improvements, documentation updates, and CI cleanup.

📦 [Download from GitHub Release](https://github.com/ohah/cheolsu-proxy/releases/tag/v0.1.1)

:::warning Unsigned Build
This build is not code-signed. After copying to `/Applications`, run the following in Terminal:

```bash
xattr -cr /Applications/Cheolsu\ Proxy.app
```

You only need to do this once.
:::

**Highlights**:

- Hardened TLS auto-passthrough policy and strengthened Never Passthrough handling.
- Improved CA/leaf certificate expiry detection, automatic regeneration, key-type mirroring, and certificate cache stability.
- Expanded OpenAPI/Contract Testing, gRPC, SSE, GraphQL, and AI traffic analysis views.
- Consolidated CLI/MCP business logic into `cheolsu_ops` and added CLI subcommands, JSON output, and completion support.
- Improved desktop workflows including the network table, filter presets, code export, and modify-and-forward breakpoints.
- Cleaned up Rust/JavaScript dependencies, Node 24/Bun 1.3.11 CI, and OpenSSL bundling verification.

### v0.1.0 (2026-03-14)

The first public release, including core proxy features and traffic manipulation tools.

📦 [Download from GitHub Release](https://github.com/ohah/cheolsu-proxy/releases/tag/v0.1.0)

:::warning Unsigned Build
This build is not code-signed. After copying to `/Applications`, run the following in Terminal:

```bash
xattr -cr /Applications/Cheolsu\ Proxy.app
```

You only need to do this once.
:::

**Proxy Core**:

- Real-time HTTP/HTTPS traffic capture and analysis
- Hybrid TLS engine (rustls + OpenSSL auto-switching)
- Upstream Certificate Sniffing — mirrors real server certificates
- TLS Auto-Learn Bypass — automatically bypasses domains that reject MITM
- SOCKS5 proxy support
- HTTP/2 support (client and upstream)
- Upstream Proxy support (HTTP/HTTPS/SOCKS5 chaining)
- Network Throttling — bandwidth limiting and latency simulation
- DNS Host Mapping — custom DNS resolution overrides
- Proxy Authentication — Basic, Bearer, API Key
- SSL Proxying whitelist/blacklist
- TLS Passthrough whitelist/blacklist
- Per-domain TLS version & cipher suite configuration
- Lazy/Eager connection strategies
- Automatic CA certificate generation and system installation
- macOS system proxy auto-configuration

**Traffic Manipulation**:

- Intercept Rules — wildcard pattern-based (Block, Map Local, Map Remote, Rewrite)
- Breakpoint — real-time request/response editing before forwarding
- Traffic Replay — resend with editable headers/body
- Advanced Repeat — bulk replay with configurable iterations
- Traffic Diff — side-by-side comparison of two transactions
- Map Local / Map Remote — redirect to local files or different servers
- JavaScript/TypeScript scripting (deno_core based, async/await, timers, hot reload)
- Server Replay (response caching and reuse)
- Quick Settings — No Caching, Block Cookies, No Gzip toggles

**Protocol Support**:

- WebSocket monitoring and message injection (virtual scroll)
- GraphQL, Socket.IO, MQTT content viewers
- gRPC/Protobuf decoding (raw wire format tree view)
- SSE streaming capture
- multipart/form-data and urlencoded body viewer

**Interfaces**:

- Desktop GUI (Tauri + React) — dark theme, i18n (English/Korean), system tray
- Terminal TUI (ratatui) — Network, WebSocket, Rules, Script, Breakpoint, Settings, Log tabs
- MCP Server — AI assistant integration (traffic inspection, rule management, script control)

**Export & Sessions**:

- HAR import/export (HTTP Archive 1.2)
- Session save/load with auto-restore
- Code export — cURL, fetch, HTTPie, Python requests
- Clipboard copy

**Certificate Management**:

- One-click macOS Keychain installation
- Mobile device certificate distribution (built-in web server + QR code)
- Client SSL Certificate (mTLS) — per-domain configuration, PKCS12 import

**Other**:

- Grafana Loki-style query builder
- Native macOS menubar (Proxy/Tools)
- Global keyboard shortcuts (proxy toggle)
- Auto-updater (Tauri Updater)
- Single instance enforcement
- Daemon architecture (multiple simultaneous client connections)
- CLI install/uninstall (from desktop app)
- Log viewer

**Platform Support**:

- macOS (Apple Silicon / aarch64)
- Windows and Linux support planned for future releases

## Update Notifications

To receive notifications for new releases:

1. Visit the [GitHub Repository](https://github.com/ohah/cheolsu-proxy)
2. Click the **Watch** button
3. Select **Releases only**

## Feedback

For bug reports or feature requests:

- [GitHub Issues](https://github.com/ohah/cheolsu-proxy/issues) — Bug reports and feature requests
