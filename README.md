<div align="center">
<img style="width:100px; margin:auto" src="assets/logo.png">
<h1> Cheolsu Proxy </h1>
<h2> A simple <i>Man In The Middle</i> proxy</h2>

[한국어](README_KO.md)

</div>

![GitHub](https://img.shields.io/github/license/ohah/cheolsu-proxy/)
![GitHub last commit](https://img.shields.io/github/last-commit/ohah/cheolsu-proxy/)
![GitHub top language](https://img.shields.io/github/languages/top/ohah/cheolsu-proxy/)

## Description

Rust-based **Man in the Middle proxy**, an early-stage project aimed at providing visibility into network traffic. It captures and displays both HTTP and HTTPS requests and responses, with a future goal of allowing traffic manipulation for more advanced use cases.

![Cast](assets/screenshots/0.gif)

## Features

- HTTP / HTTPS traffic interception
- TLS 1.0/1.1 legacy client support (hybrid TLS handler)
- GUI (Tauri + React) and CLI (headless) mode
- Custom listen address and port
- Request/response detail inspection
- Request filtering by method
- Delete individual requests or clear all
- Dark / light theme
- **Security**: Auto-generated unique CA certificate per user (private key is never bundled in the binary)

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

- [macOS guide](https://support.apple.com/it-it/guide/mac-help/mchlp2591/mac)
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

| Option          | Short | Description                              |
| --------------- | ----- | ---------------------------------------- |
| `--headless`    | `-H`  | Run proxy without GUI                    |
| `--port <PORT>` | `-p`  | Proxy listen port (default: 8100)        |
| `--host <HOST>` | `-b`  | Proxy listen host (default: 127.0.0.1)   |
| `--verbose`     | `-v`  | Enable verbose logging                   |

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

## Help & Discussion

If you have questions on how to use Cheolsu Proxy, please use [GitHub Discussions](https://github.com/ohah/cheolsu-proxy/discussions)!

![GitHub Discussions](https://img.shields.io/github/discussions/ohah/cheolsu-proxy)

## Contributing

Contributions are always welcome!

See `contributing.md` for ways to get started.

Please adhere to this project's `code of conduct`.

## License

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT) for details.
