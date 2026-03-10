# Release Notes

Version-by-version update history for Cheolsu Proxy.

## Latest Release

### v0.1.0 (In Development)

The first public release, including core proxy features and traffic manipulation tools.

**Core Features**:

- Real-time HTTP/HTTPS traffic capture and analysis
- Hybrid TLS engine (rustls + native-tls auto-switching)
- Automatic CA certificate generation and system installation
- macOS system proxy auto-configuration

**Traffic Manipulation**:

- Intercept Rules (Block, Modify Request/Response, Map Local, Map Remote)
- JavaScript/TypeScript scripting (Deno Core based)
- Server Replay (response caching and reuse)

**Protocol Support**:

- WebSocket monitoring and message injection (Plain Text, Socket.IO, MQTT)
- Upstream Proxy support (proxy chaining, authentication)
- gRPC/Protobuf decoding

**Interfaces**:

- Desktop GUI (Tauri)
- Terminal TUI
- MCP Server (AI assistant integration)

**Other**:

- Cheolsu-Query filtering language
- HAR export (HTTP Archive 1.2)
- Session save/load
- cURL and various code export
- Daemon architecture (multiple simultaneous client connections)

## Update Notifications

To receive notifications for new releases:

1. Visit the [GitHub Repository](https://github.com/ohah/cheolsu-proxy)
2. Click the **Watch** button
3. Select **Releases only**

## Feedback

For bug reports or feature requests:

- [GitHub Issues](https://github.com/ohah/cheolsu-proxy/issues) — Bug reports and feature requests
- [GitHub Discussions](https://github.com/ohah/cheolsu-proxy/discussions) — Discussion and questions
