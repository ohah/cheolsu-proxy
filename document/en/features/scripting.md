# Scripting

Programmatically manipulate HTTP requests/responses and WebSocket messages using the Deno Core (V8) based JavaScript/TypeScript scripting engine.

---

## Supported File Types

- `.js`, `.ts`, `.mjs`, `.mts`
- TypeScript is automatically transpiled using oxc.

---

## Hook API

Hook functions support both **synchronous** and **asynchronous (async/await)** patterns.

### cheolsu.onRequest(handler)

Called before an HTTP request is forwarded to the server.

```javascript
// Sync hook
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

// Async hook
cheolsu.onRequest(async (request) => {
  await new Promise((resolve) => setTimeout(resolve, 100));
  request.headers["X-Timestamp"] = Date.now().toString();
  return { action: "modify", request };
});
```

### cheolsu.onResponse(handler)

Called before a server response is delivered to the client.

```javascript
cheolsu.onResponse(async (request, response) => {
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

## Timer API

```javascript
// Execute callback after specified delay (ms)
const id = setTimeout(callback, delay);
clearTimeout(id);

// Execute callback repeatedly at specified interval (ms)
const id = setInterval(callback, delay);
clearInterval(id);
```

> **Note:** Timers only fire during hook execution (while the event loop is active). They cannot be used as background timers between hook invocations.

---

## Console API

The following console functions are available in scripts, with logs displayed in real-time on the GUI/TUI console panel.

- `console.log()` - General log
- `console.warn()` - Warning
- `console.error()` - Error
- `console.info()` - Info
- `console.debug()` - Debug

---

## API Reference

### Available APIs

| API                                                    | Description                                  |
| ------------------------------------------------------ | -------------------------------------------- |
| `cheolsu.onRequest(handler)`                           | Register HTTP request hook (sync/async)      |
| `cheolsu.onResponse(handler)`                          | Register HTTP response hook (sync/async)     |
| `cheolsu.onWebSocketMessage(handler)`                  | Register WebSocket message hook (sync/async) |
| `console.log/warn/error/info/debug()`                  | Console logging                              |
| `setTimeout(callback, delay)`                          | Delayed execution                            |
| `clearTimeout(id)`                                     | Cancel timeout                               |
| `setInterval(callback, delay)`                         | Repeated execution                           |
| `clearInterval(id)`                                    | Cancel interval                              |
| `async` / `await`                                      | Asynchronous processing                      |
| `Promise`                                              | Promise API                                  |
| `JSON.parse()` / `JSON.stringify()`                    | JSON processing                              |
| `Math.*`                                               | Math functions                               |
| `Date`                                                 | Date/time                                    |
| `RegExp`                                               | Regular expressions                          |
| `Array` / `Object` / `String` / `Map` / `Set`          | Standard built-in objects                    |
| `Symbol` / `WeakMap` / `WeakSet` / `Proxy` / `Reflect` | ECMAScript standard                          |
| `TextEncoder` / `TextDecoder`                          | Text encoding (V8 built-in)                  |
| `structuredClone()`                                    | Deep clone (V8 built-in)                     |
| TypeScript                                             | Automatic transpilation                      |

### Unavailable APIs

| API                                  | Reason                                           |
| ------------------------------------ | ------------------------------------------------ |
| `fetch()`                            | Network I/O op not registered                    |
| `XMLHttpRequest`                     | Browser-only API                                 |
| `require()`                          | CommonJS module system not supported             |
| `import` / `export` (ESM)            | ES module system not supported                   |
| `fs` / `path` / `os`                 | Node.js built-in modules not supported           |
| `process`                            | Node.js-only global object                       |
| `Buffer`                             | Node.js-only (use `Uint8Array` instead)          |
| `crypto`                             | Web Crypto API not registered                    |
| `WebSocket` (client)                 | Network I/O not supported                        |
| `Worker` / `SharedWorker`            | Worker threads not supported                     |
| `localStorage` / `sessionStorage`    | Browser-only API                                 |
| `DOM API` (`document`, `window`)     | Browser-only                                     |
| `alert()` / `confirm()` / `prompt()` | Browser-only                                     |
| Top-level `await`                    | Not supported in script mode (only inside hooks) |

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
