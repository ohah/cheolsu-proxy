<div align="center">
<img style="width:100px; margin:auto" src="assets/logo.png">
<h1> Cheolsu Proxy </h1>
<h2> A simple <i>Man In The Middle</i> proxy</h2>

[한국어](README_KO.md)

</div>

![GitHub](https://img.shields.io/github/license/ohah/cheolsu-proxy)
![GitHub last commit](https://img.shields.io/github/last-commit/ohah/cheolsu-proxy)
![GitHub top language](https://img.shields.io/github/languages/top/ohah/cheolsu-proxy)

## Description

Rust-based **Man in the Middle proxy** for inspecting and manipulating HTTP/HTTPS/WebSocket traffic. Provides a desktop GUI (Tauri + React), a terminal UI (Ratatui), and a headless CLI mode. Supports scriptable traffic manipulation, intercept rules, and integrates with AI assistants via MCP.

## Features

### Core

- HTTP / HTTPS traffic interception
- WebSocket capture and message injection
- TLS 1.0/1.1 legacy client support (hybrid TLS handler)
- Request/response detail inspection (headers, body, media preview)
- Upstream proxy support (HTTP/HTTPS/SOCKS with authentication)
- Network throttling (bandwidth limiting)

### Traffic Manipulation

- **Intercept Rules** — Block, modify request/response, map to local file, or redirect to remote URL
- **Server Replay** — Save captured responses and auto-replay on matching requests
- **Request Replay** — Re-send individual or sequential requests
- **Scripting** — TypeScript-based request/response/WebSocket manipulation (Deno Core / V8)

### Filtering & Export

- **Cheolsu-Query** — Dedicated query language for traffic filtering (method, status, URL with logical operators)
- **HAR Export** — Export traffic in HTTP Archive format

### Interface

- GUI desktop app (Tauri + React)
- TUI terminal interface (Ratatui)
- CLI headless mode
- MCP Server for AI assistant integration
- Dark / light theme
- i18n support (English, Korean)
- Global keyboard shortcuts

### Security

- Auto-generated unique CA certificate per user (private key is never bundled in the binary)

## Getting Started

### 1. Certificate Auto-Generation & Installation

Cheolsu Proxy automatically generates a unique CA certificate on first launch.

**macOS — Manual certificate installation:**

1. Run the app. The console will display the certificate file path:

   ```
   📁 Path: ~/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer
   ```

2. Open **Keychain Access**
3. Select the **login** keychain
4. Go to **File > Import Items...**
5. Select the `cheolsu-proxy.cer` file from the path above
6. Double-click the certificate and set it to **Always Trust**

**Other OS guides:**

- [Ubuntu guide](https://ubuntu.com/server/docs/security-trust-store)
- [Windows guide](https://learn.microsoft.com/en-us/skype-sdk/sdn/articles/installing-the-trusted-root-certificate)

### 2. System Proxy Configuration

Set your local system proxy to `127.0.0.1:8100`.

- [macOS guide](https://support.apple.com/en-us/guide/mac-help/mchlp2591/mac)
- [Ubuntu guide](https://help.ubuntu.com/stable/ubuntu-help/net-proxy.html.en)
- [Windows guide](https://support.microsoft.com/en-us/windows/use-a-proxy-server-in-windows-03096c53-0554-4ffe-b6ab-8b1deee8dae1)

## Documentation

For detailed documentation, visit the [official docs site](https://ohah.github.io/cheolsu-proxy).

- **User Guide**: Installation, configuration, usage
- **Contributor Guide**: Dev environment setup, code structure, how to contribute
- **Feature Docs**: TLS support, certificate configuration, etc.

### Local Documentation

Markdown documentation is available in the [document/](document/) directory.

- [TLS 1.0/1.1 Support](document/features/TLS_1_0_1_1_SUPPORT.md) — Legacy TLS client support
- [Intercept Rules](document/en/features/intercept-rules.md) — Block, modify, and redirect traffic
- [Server Replay](document/en/features/server-replay.md) — Auto-replay captured responses
- [Scripting](document/en/features/scripting.md) — TypeScript-based traffic manipulation
- [WebSocket](document/en/features/websocket.md) — WebSocket capture and injection
- [MCP Server](document/en/features/mcp-server.md) — AI assistant integration
- [Cheolsu-Query](document/en/features/cheolsu-query.md) — Query language for traffic filtering

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/) or npm
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)
- OpenSSL 3.x (required for building)

**macOS:**

```bash
brew install openssl@3 pkg-config
```

### 1. Generate Certificates

The proxy requires a CA certificate to intercept HTTPS traffic. Run the certificate generation script from the `crates/` directory:

```bash
cd crates
bash ../install_cer.sh
```

> The script generates certificate files in `crates/proxyapi_v2/src/certificate_authority/`.
> The private key is converted to PKCS#8 format (for rcgen library compatibility).

### 2. Trust the CA Certificate

Register the generated CA certificate with your OS for HTTPS proxy to work properly.

**macOS:**

```bash
open crates/proxyapi_v2/src/certificate_authority/cheolsu-proxy.cer
```

In Keychain Access, double-click the certificate → Trust section → set to **Always Trust**.

**Linux:**

```bash
sudo cp crates/proxyapi_v2/src/certificate_authority/cheolsu-proxy.cer /usr/local/share/ca-certificates/cheolsu-proxy.crt
sudo update-ca-certificates
```

**Windows:**

Import `cheolsu-proxy.cer` into "Trusted Root Certification Authorities" via Certificate Manager (`certmgr.msc`).

### 3. Run Development Server

```bash
cd desktop
bun install
bun tauri dev
```

If you encounter OpenSSL linking errors, set the environment variable:

```bash
PKG_CONFIG_PATH="/opt/homebrew/opt/openssl@3/lib/pkgconfig" bun tauri dev
```

### 4. CLI (Headless) Mode

Run the proxy without the GUI:

```bash
bun tauri dev -- -- --headless --port 8100
```

| Option          | Short | Description                            |
| --------------- | ----- | -------------------------------------- |
| `--headless`    | `-H`  | Run proxy without GUI                  |
| `--port <PORT>` | `-p`  | Proxy listen port (default: 8100)      |
| `--host <HOST>` | `-b`  | Proxy listen host (default: 127.0.0.1) |
| `--verbose`     | `-v`  | Enable verbose logging                 |

### 5. Testing

```bash
# Unit tests (TLS strategy selection, ClientHello parsing, etc.)
PKG_CONFIG_PATH="/opt/homebrew/opt/openssl@3/lib/pkgconfig" cargo test -p proxyapi_v2 --lib

# Integration tests (requires CA certificate to be trusted by the system)
PKG_CONFIG_PATH="/opt/homebrew/opt/openssl@3/lib/pkgconfig" cargo test -p proxyapi_v2 --test rcgen_ca
```

### Certificate File Locations

- **Development**: `crates/proxyapi_v2/src/certificate_authority/`
- **Production (macOS)**: `~/Library/Application Support/com.cheolsu-proxy/`
- **Production (Windows)**: `%APPDATA%/com.cheolsu-proxy/` (planned)
- **Production (Linux)**: `~/.config/com.cheolsu-proxy/` (planned)

## Contributing

Contributions are always welcome!

## License

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT) for details.
