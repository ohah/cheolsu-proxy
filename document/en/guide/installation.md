# Installation

Getting Cheolsu Proxy installed and running on your machine.

---

## System Requirements

| Platform                         | Status      |
| -------------------------------- | ----------- |
| **macOS** (Apple Silicon)        | Supported   |
| **macOS** (Intel / x64)          | Not built   |
| **Windows**                      | Coming soon |
| **Linux**                        | Coming soon |

Cheolsu Proxy is a native desktop application built with Rust and Tauri, so it runs with minimal resource overhead compared to Electron-based alternatives. There are no runtime dependencies like Java or Python to install.

## Download

Download the latest release from the [GitHub Releases](https://github.com/ohah/cheolsu-proxy/releases) page:

- **Apple Silicon** (M1/M2/M3/M4): `Cheolsu.Proxy_x.x.x_aarch64.dmg`

> Only the Apple Silicon (aarch64) build is currently published. An Intel (x64) build is not provided.

## macOS Installation

1. Open the downloaded `.dmg` file
2. Drag **Cheolsu Proxy** into the **Applications** folder
3. Eject the disk image

### First Launch and macOS Gatekeeper

Because Cheolsu Proxy is distributed outside the Mac App Store, macOS Gatekeeper may block the first launch with a message like _"Cheolsu Proxy can't be opened because Apple cannot check it for malicious software."_

To open it anyway:

1. Open **System Settings** (or System Preferences on older macOS)
2. Go to **Privacy & Security**
3. Scroll down to the **Security** section — you should see a message about Cheolsu Proxy being blocked
4. Click **Open Anyway**
5. Confirm in the dialog that appears

You only need to do this once. Subsequent launches will work normally.

> The current build is not Apple-notarized/signed. If it still won't open, remove the quarantine attribute from the Terminal:
>
> ```bash
> xattr -cr "/Applications/Cheolsu Proxy.app"
> ```

## Three Ways to Use Cheolsu Proxy

Cheolsu Proxy ships with three interfaces, all powered by the same underlying proxy daemon:

### 1. Desktop GUI

The full graphical interface. Launch it from your Applications folder like any other app. This is the recommended starting point for most users — it provides a visual traffic table, request/response inspectors, and point-and-click configuration.

### 2. TUI (Terminal User Interface)

A terminal-based interface for developers who prefer working in the command line. It provides the same core proxy functionality with keyboard-driven navigation.

To run the TUI:

```bash
cheolsu-tui
```

If you have the CLI installed, you can also launch it via `cheolsu tui --port 8100`.

> The TUI binary (`cheolsu-tui`) is bundled with the desktop app as a sidecar. If it is not in your PATH, find it inside the application bundle, or build it from source with `cargo build -p cheolsu-proxy-tui --release`.

### 3. MCP Server

An AI-integrated interface that exposes captured traffic to AI assistants like Claude Code, Cursor, and Claude Desktop via the [Model Context Protocol](https://modelcontextprotocol.io/). See the [MCP Server](../features/mcp-server.md) documentation for setup details.

## Verifying the Installation

1. Launch Cheolsu Proxy
2. The proxy daemon starts automatically in the background
3. Make a network request from any browser or application configured to use the proxy
4. You should see the request appear in the traffic list

If you run into issues, check the [Troubleshooting](./troubleshooting.md) guide.

## Next Steps

- [SSL Certificates](./ssl-certificates.md) — Install the CA certificate so Cheolsu Proxy can intercept HTTPS traffic
- [Proxying](./proxying.md) — Understand how to configure your system and browsers to route traffic through the proxy
- [Recording](./recording.md) — Learn how to capture, inspect, and export network traffic
- [Basic Usage](./basic-usage.md) — Quick-start walkthrough of the core workflow

---

**Ready to go?** Start with [SSL Certificates](./ssl-certificates.md) to enable HTTPS interception.
