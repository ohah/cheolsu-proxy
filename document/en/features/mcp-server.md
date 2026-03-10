# MCP Server

Cheolsu Proxy includes a built-in [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server that exposes captured network traffic and proxy controls to AI assistants. This lets you use tools like Claude Code, Cursor, and Claude Desktop to query, analyze, and manipulate proxy traffic through natural language.

Instead of manually searching through hundreds of requests, you can ask your AI assistant questions like "find all requests to the payments API that returned errors" or "generate TypeScript interfaces from this API response." The MCP server bridges the gap between Cheolsu Proxy's traffic data and your AI-powered development workflow.

---

## Prerequisites

Before setting up the MCP server, make sure you have:

1. **Cheolsu Proxy installed and running.** The MCP server communicates with the proxy daemon over a Unix Domain Socket. The proxy must be running for the MCP server to function.
2. **An MCP-compatible AI client.** Currently supported clients include Claude Code, Cursor, and Claude Desktop.
3. **The `cheolsu-proxy-mcp` binary.** This is included with the Cheolsu Proxy installation. You will need its file path during setup.

---

## Setup

### Step 1: Get the MCP Configuration

In the Cheolsu Proxy desktop app, click the **MCP Server** button at the bottom of the left sidebar. A JSON configuration snippet will appear. Click the copy button to copy it to your clipboard.

### Step 2: Register with Your AI Client

Paste the configuration into your AI client's MCP settings.

**Claude Code**

The simplest approach is to use the CLI:

```bash
claude mcp add cheolsu-proxy -- /path/to/cheolsu-proxy-mcp
```

Alternatively, add it manually to `.claude/settings.json`:

```json
{
  "mcpServers": {
    "cheolsu-proxy": {
      "command": "/path/to/cheolsu-proxy-mcp"
    }
  }
}
```

**Cursor**

Add to `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "cheolsu-proxy": {
      "command": "/path/to/cheolsu-proxy-mcp"
    }
  }
}
```

Replace `/path/to/cheolsu-proxy-mcp` with the actual path shown in the MCP configuration from Step 1.

### Step 3: Verify the Connection

With Cheolsu Proxy running, ask your AI assistant a simple question like "What is the proxy status?" If the MCP server is connected correctly, the assistant will use the `proxy_status` tool and return information about the running proxy.

---

## Available Tools

### Traffic Inspection

| Tool | Description |
| --- | --- |
| `search_traffic` | Search captured traffic by host, HTTP method, status code, or URL path |
| `get_transaction` | Get the full request and response (headers and body) for a specific transaction |
| `get_websocket_messages` | Get captured WebSocket messages, optionally filtered by connection URI |

### Request Sending

| Tool | Description |
| --- | --- |
| `replay_request` | Send an HTTP request directly, bypassing the proxy. Useful for testing APIs without triggering intercept rules |

### Intercept Rule Management

| Tool | Description |
| --- | --- |
| `list_rules` | List all currently configured intercept rules |
| `add_rule` | Add a new intercept rule (block, modify_request, modify_response, map_local, map_remote) |
| `remove_rule` | Remove an intercept rule by its ID |

### Proxy Status

| Tool | Description |
| --- | --- |
| `proxy_status` | Check whether the proxy daemon is running and view traffic statistics |
| `clear_traffic` | Clear all captured traffic data from memory |

---

## Usage Examples

Here are practical ways to use the MCP server during development:

**Debugging API errors**

Ask: "Find any requests that returned 500 errors in the last few minutes."

The assistant uses `search_traffic` with a status code filter, then you can follow up with "Show me the full request and response for that failed request" to get details via `get_transaction`.

**Generating code from traffic**

Ask: "Look at the response from the /api/v1/users endpoint and generate TypeScript interfaces for it."

The assistant fetches the response body and creates type definitions matching the actual API shape.

**Managing intercept rules**

Ask: "Block all requests to tracking.example.com" or "Add a rule that returns a 503 for the payments API."

The assistant uses `add_rule` to create the rule. You can verify with "List all active intercept rules."

**Testing API changes**

Ask: "Replay the last POST request to /api/orders but change the quantity to 0."

The assistant uses `replay_request` to send a modified version of a previously captured request.

**Analyzing WebSocket traffic**

Ask: "Show me the MQTT messages from the IoT device connection."

The assistant uses `get_websocket_messages` with a URI filter to retrieve the relevant messages.

---

## Architecture

```
AI Assistant (Claude Code / Cursor)
        |
        | MCP Protocol (stdio)
        v
+---------------------+
|  cheolsu-proxy-mcp  |  MCP server binary
+----------+----------+
           | Unix Domain Socket
           v
+---------------------+
|    Proxy Daemon     |
+---------------------+
```

The MCP server binary is a standalone process that acts as a client of the proxy daemon. It communicates with the AI assistant over standard input/output using the MCP protocol, and connects to the proxy daemon over a Unix Domain Socket.

The MCP server keeps the most recent 1,000 HTTP transactions and 5,000 WebSocket messages in memory for fast querying. Older data is discarded automatically. Use `clear_traffic` to reset the data manually if needed.
