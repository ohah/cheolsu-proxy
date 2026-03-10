# Installation

Getting Cheolsu Proxy installed and running on your machine.

---

## System Requirements

| Platform | Status |
| -------- | ------ |
| **macOS** (Apple Silicon & Intel) | Supported |
| **Windows** | Coming soon |
| **Linux** | Coming soon |

Cheolsu Proxy is a native desktop application built with Rust and Tauri, so it runs with minimal resource overhead compared to Electron-based alternatives. There are no runtime dependencies like Java or Python to install.

## Download

Download the latest release from the [GitHub Releases](https://github.com/ohah/cheolsu-proxy/releases) page. Choose the `.dmg` file that matches your Mac's architecture:

- **Apple Silicon** (M1/M2/M3/M4): `cheolsu-proxy_x.x.x_aarch64.dmg`
- **Intel**: `cheolsu-proxy_x.x.x_x64.dmg`

> Not sure which Mac you have? Click the Apple menu and select **About This Mac**. If the Chip field says "Apple M1" or similar, choose the Apple Silicon build.

## macOS Installation

1. Open the downloaded `.dmg` file
2. Drag **Cheolsu Proxy** into the **Applications** folder
3. Eject the disk image

### First Launch and macOS Gatekeeper

Because Cheolsu Proxy is distributed outside the Mac App Store, macOS Gatekeeper may block the first launch with a message like *"Cheolsu Proxy can't be opened because Apple cannot check it for malicious software."*

To open it anyway:

1. Open **System Settings** (or System Preferences on older macOS)
2. Go to **Privacy & Security**
3. Scroll down to the **Security** section — you should see a message about Cheolsu Proxy being blocked
4. Click **Open Anyway**
5. Confirm in the dialog that appears

You only need to do this once. Subsequent launches will work normally.

## Three Ways to Use Cheolsu Proxy

Cheolsu Proxy ships with three interfaces, all powered by the same underlying proxy daemon:

### 1. Desktop GUI

The full graphical interface. Launch it from your Applications folder like any other app. This is the recommended starting point for most users — it provides a visual traffic table, request/response inspectors, and point-and-click configuration.

### 2. TUI (Terminal User Interface)

A terminal-based interface for developers who prefer working in the command line. It provides the same core proxy functionality with keyboard-driven navigation.

To run the TUI:

```bash
cheolsu-proxy-tui
```

> The TUI binary is bundled alongside the desktop app. If it is not in your PATH, you can find it inside the application bundle or in the same directory as the main binary.

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
