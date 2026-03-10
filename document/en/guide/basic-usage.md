# Basic Usage

This guide walks you through the basics of Cheolsu Proxy: starting the proxy, capturing traffic, and inspecting requests and responses.

## Starting the Proxy

When you launch Cheolsu Proxy, the daemon process starts automatically and the proxy server becomes active. The default port is `8100`.

### Desktop GUI

1. Launch the Cheolsu Proxy app.
2. The proxy daemon starts automatically when the app opens.
3. On macOS, the system proxy is automatically configured, so traffic capture begins immediately.

### TUI (Terminal)

Run the `cheolsu-proxy-tui` binary from your terminal to start the terminal-based interface. The TUI connects to the same proxy daemon.

## Setting Up Certificates for HTTPS

To capture HTTPS traffic, you need to install and trust Cheolsu Proxy's CA certificate on your system. Without the certificate, your browser will show security warnings on HTTPS sites.

### macOS

1. In Cheolsu Proxy, click **Settings** → **Certificates**.
2. Click the **"Install Certificate"** button.
3. The certificate is installed on your system.
4. Open **Keychain Access**, find the Cheolsu Proxy certificate, and double-click it.
5. Set it to **"Always Trust"**.

For detailed certificate setup instructions, see the [SSL Certificates](./ssl-certificates.md) guide.

## Viewing Traffic

Once the proxy is running, network requests from your browser or application flow through Cheolsu Proxy and are captured in real time.

### Traffic Table

The main network dashboard shows a list of captured traffic. Each entry displays:

- HTTP method (GET, POST, PUT, etc.)
- URL
- Response status code
- Response size
- Duration

### Host/Path Tree View

The tree view in the dashboard lets you browse traffic hierarchically by host and path. This is useful when you want to focus on traffic from a specific domain or API endpoint.

### Request/Response Details

Select an entry in the traffic table to view the full details of that transaction:

- **Request**: method, URL, headers, body
- **Response**: status code, headers, body
- **Timing**: request/response duration

## Filtering Traffic

When you have a lot of captured traffic, use [Cheolsu-Query](/en/features/cheolsu-query) to filter down to the requests you care about. Type in the query bar:

```text
method="GET" and url|="api"
```

This shows only GET requests with "api" in the URL. See the [Cheolsu-Query](/en/features/cheolsu-query) documentation for the full query syntax.

## HAR Export

You can export captured traffic in HAR (HTTP Archive) 1.2 format. HAR files are a standard format that can be opened with Chrome DevTools, Firefox, Charles Proxy, and other tools.

### Desktop

1. Capture traffic on the network dashboard.
2. Click the **Download** button in the header.
3. Choose a save location and a `.har` file will be created.

### TUI

1. Press `e` on the network screen.
2. A `cheolsu-proxy-YYYYMMDD-HHMMSS.har` file is created in the current directory.

## Stopping the Proxy

Click the **"Stop Proxy"** button in the Desktop GUI or close the app. The system proxy settings are automatically restored.

## Next Steps

Now that you know the basics, explore these features:

- [SSL Certificates](./ssl-certificates.md) — Certificate setup for HTTPS capture
- [Proxying](./proxying.md) — Port configuration, upstream proxy, mobile connections
- [Intercept Rules](/en/features/intercept-rules) — Block, modify, or redirect requests
- [Scripting](/en/features/scripting) — Automate traffic manipulation with JavaScript/TypeScript
- [MCP Server](/en/features/mcp-server) — AI assistant integration for traffic analysis
