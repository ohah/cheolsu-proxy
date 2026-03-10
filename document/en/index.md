# Cheolsu Proxy

An HTTP/HTTPS debugging proxy built with Rust and Tauri. Capture, analyze, and modify network traffic between your browser and the server in real time.

> This project was forked from [Proxelar](https://github.com/emanuele-em/proxelar).

## Key Features

- **HTTP/HTTPS Traffic Capture**: Real-time network traffic monitoring and analysis
- **WebSocket Support**: Plain Text, Socket.IO, MQTT protocol detection and message injection
- **Intercept Rules**: 5 action types - Block, Modify Request/Response, Map Local, Map Remote
- **JavaScript/TypeScript Scripting**: Deno Core based request/response/WebSocket hooks
- **Server Replay**: Cache captured responses and auto-return for identical requests
- **MCP Server**: Integration with AI assistants like Claude Code, Cursor
- **Cheolsu-Query**: Advanced traffic filtering query language
- **TLS Support**: Hybrid TLS engine (rustls + native-tls auto-switching)
- **HAR Export**: HTTP Archive 1.2 format support
- **3 Interfaces**: Desktop GUI (Tauri), Terminal TUI, MCP Server
- **Daemon Architecture**: Multiple clients can connect to the same proxy daemon simultaneously
- **System Proxy Auto-configuration**: macOS networksetup integration
- **Upstream Proxy**: Proxy chaining with authentication support

## Guide

Step-by-step guides to get you started with Cheolsu Proxy.

- [Installation](/en/guide/installation) - Download and install
- [Basic Usage](/en/guide/basic-usage) - Start the proxy, capture traffic, inspect requests
- [SSL Certificates](/en/guide/ssl-certificates) - Certificate installation for HTTPS capture
- [Proxying](/en/guide/proxying) - System proxy, port settings, mobile connections
- [Recording](/en/guide/recording) - Traffic views, filtering, HAR export
- [Sessions](/en/guide/sessions) - Save and load sessions
- [Troubleshooting](/en/guide/troubleshooting) - Resolving common issues

## Feature Documentation

- [MCP Server](/en/features/mcp-server) - AI assistant integration
- [Cheolsu-Query](/en/features/cheolsu-query) - Network request filtering query language
- [Intercept Rules](/en/features/intercept-rules) - Request/response interception and modification
- [Scripting](/en/features/scripting) - Traffic manipulation with JavaScript/TypeScript
- [WebSocket](/en/features/websocket) - WebSocket traffic monitoring and injection
- [Server Replay](/en/features/server-replay) - Cached response reuse
- [TLS 1.0/1.1 Support](/en/features/tls-support) - Legacy TLS client support

## Release Notes

Check the latest update information and development roadmap.

- [Release Notes](/en/releases/) - Version-by-version update history

## Contributing

If you want to contribute to the project, please refer to the [Contributing Guide](/en/contributing/).

- [Development Setup](/en/contributing/development-setup)
- [Code Structure](/en/contributing/code-structure)
- [Testing](/en/contributing/testing)

## License

This project is distributed under the MIT and Apache 2.0 licenses.

- [MIT License](https://github.com/ohah/cheolsu-proxy/blob/main/LICENSE-MIT)
- [Apache 2.0 License](https://github.com/ohah/cheolsu-proxy/blob/main/LICENSE-APACHE)
