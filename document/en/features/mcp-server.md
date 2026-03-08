# MCP Server

cheolsu-proxy includes a built-in [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server, allowing AI assistants like Claude Code, Cursor, and Claude Desktop to directly query and manipulate captured traffic.

---

## Setup

### 1. Copy MCP Configuration

Click the **MCP Server** button at the bottom of the left sidebar in the app. A JSON configuration will appear — click the copy button to copy it to your clipboard.

### 2. Register with Your AI Client

Paste the configuration into your AI client's MCP settings file.

**Claude Code**

Register easily with the CLI command:

```bash
claude mcp add cheolsu-proxy -- /path/to/cheolsu-proxy-mcp
```

Or add directly to `.claude/settings.json`:

```json
{
  "mcpServers": {
    "cheolsu-proxy": {
      "command": "/path/to/cheolsu-proxy-mcp"
    }
  }
}
```

**Cursor** (`.cursor/mcp.json`)

```json
{
  "mcpServers": {
    "cheolsu-proxy": {
      "command": "/path/to/cheolsu-proxy-mcp"
    }
  }
}
```

### 3. Use It

With the Cheolsu Proxy app running, simply ask your AI assistant about your traffic.

---

## Available MCP Tools

### Traffic Inspection

| Tool                     | Description                                                            |
| ------------------------ | ---------------------------------------------------------------------- |
| `search_traffic`         | Search captured traffic by host, HTTP method, status code, or URL path |
| `get_transaction`        | Get full request/response headers and body for a specific transaction  |
| `get_websocket_messages` | Get captured WebSocket messages (filterable by connection URI)         |

### Request Sending

| Tool             | Description                                                             |
| ---------------- | ----------------------------------------------------------------------- |
| `replay_request` | Send an HTTP request directly (bypassing proxy). Useful for API testing |

### Intercept Rule Management

| Tool          | Description                                                                              |
| ------------- | ---------------------------------------------------------------------------------------- |
| `list_rules`  | List currently configured intercept rules                                                |
| `add_rule`    | Add a new intercept rule (block, modify_request, modify_response, map_local, map_remote) |
| `remove_rule` | Remove an intercept rule by ID                                                           |

### Status

| Tool            | Description                                      |
| --------------- | ------------------------------------------------ |
| `proxy_status`  | Check proxy daemon status and traffic statistics |
| `clear_traffic` | Clear captured traffic data from memory          |

---

## Usage Examples

You can ask your AI assistant things like:

- "Find any API requests that returned 500 errors"
- "Look at this API's request/response and generate TypeScript interfaces"
- "Add a rule to block requests to example.com"
- "Replay this request with a different body"

---

## Architecture

```
AI Assistant (Claude Code / Cursor)
        │
        │ MCP Protocol (stdio)
        ▼
┌─────────────────────┐
│  cheolsu-proxy-mcp  │  MCP server binary
│  (collects/exposes)  │
└─────────┬───────────┘
          │ Unix Domain Socket
          ▼
┌─────────────────────┐
│   Proxy Daemon      │  Proxy daemon
└─────────────────────┘
```

The MCP server acts as another client of the proxy daemon. It collects traffic in real-time, keeping the most recent 1,000 HTTP transactions and 5,000 WebSocket messages in memory.
