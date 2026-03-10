# Recording Traffic

How to capture, inspect, and export HTTP/HTTPS traffic with Cheolsu Proxy.

---

## What is Recording?

Recording is the core function of Cheolsu Proxy. When recording is active, the proxy captures every HTTP and HTTPS request that flows through it — including headers, bodies, timing information, and WebSocket frames. This gives you a complete picture of the network activity between your application and the servers it communicates with.

You might use recording to:

- **Debug API calls** — see exactly what your app sends and what the server returns
- **Diagnose slow requests** — inspect timing breakdowns to find bottlenecks
- **Verify integrations** — confirm that third-party SDKs or libraries send the expected requests
- **Reproduce issues** — capture a failing sequence of requests to share with teammates

## Start and Stop Recording

### Desktop GUI

Recording starts automatically when the proxy is running. All traffic routed through the proxy appears in the traffic list in real time.

To stop capturing new traffic, stop the proxy from the main interface. Existing captured traffic remains visible for inspection.

### TUI

In the TUI, traffic is captured as long as the proxy daemon is active. The network screen displays incoming requests as they arrive.

## Viewing Traffic in the Desktop GUI

The Desktop GUI provides two ways to browse captured traffic:

### Table View

The default view shows traffic as a flat, sortable table. Each row represents one HTTP transaction with columns for:

- **Status** — HTTP status code (color-coded: green for 2xx, yellow for 3xx, red for 4xx/5xx)
- **Method** — GET, POST, PUT, DELETE, etc.
- **Host** — The target server
- **Path** — The request URL path
- **Content-Type** — Response content type
- **Size** — Response body size
- **Time** — How long the round-trip took

Click any row to open the detail view for that request.

### Host/Path Tree View

An alternative view that organizes traffic hierarchically by host and path, similar to a file tree. This is useful when you want to focus on a specific API or domain without scrolling through unrelated traffic.

## Filtering Traffic

When you are capturing a lot of traffic, filtering helps you find what matters. Cheolsu Proxy includes a powerful query bar powered by [Cheolsu-Query](../features/cheolsu-query.md), a dedicated filtering language.

Some quick examples:

```text
# Show only API requests
url|="api"

# Show only failed requests
status="4xx,5xx"

# Show only POST requests to a specific host
method="POST" and url|="example.com"

# Exclude analytics noise
url!~="google-analytics" and url!~="ads"
```

See the full [Cheolsu-Query](../features/cheolsu-query.md) documentation for all available keywords, operators, and examples.

## Request and Response Detail View

Selecting a request opens a detail panel with tabs for:

### Request

- **Headers** — All request headers (Host, User-Agent, Content-Type, Authorization, etc.)
- **Body** — The request body, with formatting for JSON, XML, form data, and other common content types

### Response

- **Headers** — All response headers (Content-Type, Cache-Control, Set-Cookie, etc.)
- **Body** — The response body, formatted and syntax-highlighted based on content type

### Timing

Timing breakdown showing how long each phase of the request took, helping you identify whether slowness comes from DNS resolution, TLS handshake, server processing, or data transfer.

## Exporting Traffic

### HAR Export

You can export all captured traffic (or filtered traffic) as a HAR (HTTP Archive) 1.2 file. HAR is a standard format supported by Chrome DevTools, Firefox, Charles Proxy, and many other tools.

**Desktop GUI:**

1. Capture the traffic you want to export
2. Click the **Download** button in the header area
3. Choose a save location — a `.har` file will be created

**TUI:**

1. On the network screen, press the `e` key
2. A file named `cheolsu-proxy-YYYYMMDD-HHMMSS.har` is saved in the current working directory

### Export as cURL / Code Snippets

For individual requests, you can export them as:

- **cURL command** — ready to paste into a terminal for quick reproduction
- **Various programming languages** — code snippets that reproduce the request in languages like Python, JavaScript, Go, and others

This is particularly useful when you want to share a specific request with a colleague or convert a captured request into a script.

## Recording in the TUI

The TUI provides the same core recording capabilities in a keyboard-driven interface:

- Traffic appears in a scrollable list as it is captured
- Use arrow keys or `j`/`k` to navigate between requests
- Press `Enter` to view request/response details
- Press `e` to export all traffic as HAR
- Press `q` to go back or quit

The TUI is ideal for remote servers, SSH sessions, or developers who prefer staying in the terminal.

## Related Documentation

- [Cheolsu-Query](../features/cheolsu-query.md) — Full query language reference for filtering traffic
- [Sessions](./sessions.md) — Save and load captured traffic
- [Proxying](./proxying.md) — Configure what traffic gets routed through the proxy
- [WebSocket](../features/websocket.md) — Inspecting WebSocket connections
- [Basic Usage](./basic-usage.md) — Quick-start overview

---

**Next step**: Learn how to save your captured traffic for later in the [Sessions](./sessions.md) guide.
