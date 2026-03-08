# WebSocket

Monitor WebSocket connections in real-time and inject messages.

---

## Protocol Detection

Automatically detects the content type of WebSocket messages.

| Type           | Description                                   |
| -------------- | --------------------------------------------- |
| **Plain Text** | Plain text or JSON messages                   |
| **Socket.IO**  | Engine.IO + Socket.IO protocol auto-detection |
| **MQTT**       | MQTT packet detection (v3.1.1, v5.0 support)  |

---

## Key Features

### Connection List

- Display active WebSocket connections in chronological order
- Show connection/disconnection status
- Display URI per connection

### Message Viewer

- Message direction display (Client -> Server, Server -> Client)
- Message type display (Text, Binary, Ping, Pong, Close)
- Binary/text distinction
- Payload content inspection

### Message Injection

Inject messages directly into active WebSocket connections.

- **Direction**: Client -> Server or Server -> Client
- **Text/Binary**: Send text or binary messages

### Message Interception

Use the scripting hook (`cheolsu.onWebSocketMessage`) to modify or drop WebSocket messages.

---

## Usage

### Desktop

1. Select **WebSocket** from the sidebar
2. Select a connection from the connection list
3. View message contents

### TUI

1. Navigate to the **WebSocket** tab
2. Browse connections and messages

### MCP

```
"Show me WebSocket messages related to MQTT"
```
