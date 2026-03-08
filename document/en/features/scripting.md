# Scripting

Programmatically manipulate HTTP requests/responses and WebSocket messages using the Deno Core (V8) based JavaScript/TypeScript scripting engine.

---

## Supported File Types

- `.js`, `.ts`, `.mjs`, `.mts`
- TypeScript is automatically transpiled using oxc.

---

## Hook API

### cheolsu.onRequest(handler)

Called before an HTTP request is forwarded to the server.

```javascript
cheolsu.onRequest((request) => {
  // request: { method, url, headers, body }

  // Forward as-is
  return { action: "forward" };

  // Forward with modifications
  return {
    action: "modify",
    request: {
      ...request,
      headers: { ...request.headers, "X-Custom": "value" },
    },
  };

  // Respond directly (skip server request)
  return {
    action: "respond",
    response: {
      status: 200,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ mocked: true }),
    },
  };
});
```

### cheolsu.onResponse(handler)

Called before a server response is delivered to the client.

```javascript
cheolsu.onResponse((request, response) => {
  // response: { status, headers, body }

  // Forward as-is
  return { action: "forward" };

  // Forward with modifications
  return {
    action: "modify",
    response: {
      ...response,
      headers: { ...response.headers, "X-Modified": "true" },
    },
  };
});
```

### cheolsu.onWebSocketMessage(handler)

Called before a WebSocket message is forwarded.

```javascript
cheolsu.onWebSocketMessage((message) => {
  // message: { direction, payload, is_binary }
  // direction: "to_server" | "to_client"

  // Forward as-is
  return { action: "forward" };

  // Forward with modifications
  return { action: "modify", payload: "modified", is_binary: false };

  // Drop the message
  return { action: "drop" };
});
```

---

## Console API

The following console functions are available in scripts, with logs displayed in real-time on the GUI/TUI console panel.

- `console.log()` - General log
- `console.warn()` - Warning
- `console.error()` - Error
- `console.debug()` - Debug

---

## Usage

### Desktop

1. Select **Script** from the sidebar
2. Write code directly in Monaco Editor or enter a file path
3. Run script with `Cmd/Ctrl + Enter`
4. Check logs in the console panel below
5. Full API documentation available in the API Reference tab

### TUI

1. Navigate to the **Script** tab
2. Enter script file path
3. Load/unload

### MCP

```
"Write a script that masks specific fields in API responses"
```

---

## Auto Reload

When a script is loaded from a file, it will automatically reload when the file changes. A 500ms debounce is applied.
