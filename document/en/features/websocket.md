# WebSocket

Cheolsu Proxy captures and displays WebSocket traffic alongside regular HTTP requests, giving you a real-time view of bidirectional communication between client and server.

WebSocket debugging matters because many modern applications rely on persistent connections for features like live chat, real-time dashboards, multiplayer games, and IoT device communication. Unlike HTTP request/response pairs, WebSocket messages are continuous and often use compact binary protocols that are difficult to inspect with standard tools. Cheolsu Proxy decodes these protocols automatically so you can see what is actually being sent and received.

---

## Protocol Detection

Cheolsu Proxy analyzes WebSocket message payloads and automatically identifies the application-level protocol in use. This means you see decoded, human-readable content rather than raw bytes.

### Plain Text and JSON

Standard text-based WebSocket messages are displayed as-is. JSON payloads are detected and can be viewed with formatting.

### Socket.IO

Cheolsu Proxy recognizes the Engine.IO transport layer and Socket.IO protocol framing. It decodes the packet type prefixes so you can distinguish between different message types:

| Packet Type | Engine.IO Code | Meaning |
| --- | --- | --- |
| open | 0 | Server sends connection parameters (sid, ping interval) |
| close | 1 | Connection is being closed |
| ping | 2 | Keep-alive ping from server |
| pong | 3 | Keep-alive pong response |
| message | 4 | Application data (Socket.IO events are nested here) |
| upgrade | 5 | Transport upgrade |

Within Engine.IO message packets, Socket.IO event names and arguments are parsed and displayed clearly.

### MQTT

MQTT packets transmitted over WebSocket are detected and decoded. Both MQTT v3.1.1 and v5.0 are supported. Common packet types you will see include:

| Packet Type | Description |
| --- | --- |
| CONNECT | Client requests connection to broker |
| CONNACK | Broker acknowledges connection |
| PUBLISH | Message published to a topic |
| SUBSCRIBE | Client subscribes to topics |
| SUBACK | Broker acknowledges subscription |
| PINGREQ / PINGRESP | Keep-alive mechanism |
| DISCONNECT | Clean disconnection |

For PUBLISH packets, the topic name and payload are extracted and shown directly in the message viewer.

---

## Connection List

All active and closed WebSocket connections are listed in chronological order. Each entry shows:

- The connection URI
- Connection status (connected or disconnected)
- Message count
- Protocol type (if detected)

Selecting a connection opens its message stream in the message viewer.

---

## Message Viewer

The message viewer displays individual frames in the order they were sent or received.

- **Direction** -- Each message is labeled as Client to Server or Server to Client, making it easy to follow the conversation flow.
- **Message type** -- Text, Binary, Ping, Pong, and Close frames are distinguished visually.
- **Payload content** -- Text payloads are shown inline. Binary payloads are displayed in a hex/text view.
- **Timestamps** -- Each message includes a timestamp for correlating events across connections.

---

## Message Injection

You can inject messages into any active WebSocket connection directly from Cheolsu Proxy. This is useful for:

- **Testing server-side handling** -- Send a malformed or unexpected message to the server to see how it responds.
- **Simulating server pushes** -- Inject a message as if it came from the server to test how the client handles specific events.
- **Replaying messages** -- Re-send a previously captured message to reproduce a bug or test idempotency.

When injecting, you choose:

- **Direction** -- Client to Server or Server to Client.
- **Format** -- Text or binary.
- **Payload** -- The message content to send.

---

## Message Interception with Scripting

For automated modification or filtering of WebSocket messages, use the `cheolsu.onWebSocketMessage` scripting hook. This lets you inspect every message flowing through the proxy and decide whether to forward it as-is, modify it, or drop it entirely.

```javascript
cheolsu.onWebSocketMessage((message) => {
  // message.direction: "to_server" or "to_client"
  // message.payload: the message content
  // message.is_binary: boolean

  // Example: Log all messages going to the server
  if (message.direction === "to_server") {
    console.log("Client sent:", message.payload);
  }

  // Example: Modify a Socket.IO event payload
  if (!message.is_binary && message.payload.includes('"temperature"')) {
    const modified = message.payload.replace('"temperature":0', '"temperature":25');
    return { action: "modify", payload: modified, is_binary: false };
  }

  // Example: Drop ping messages to test reconnection behavior
  if (!message.is_binary && message.payload === "2") {
    return { action: "drop" };
  }

  // Forward everything else unchanged
  return { action: "forward" };
});
```

See the [Scripting](./scripting.md) documentation for the full API reference.

---

## Usage

### Desktop

1. Select **WebSocket** from the sidebar.
2. The connection list shows all captured WebSocket connections.
3. Click a connection to open the message viewer.
4. Use the injection panel at the bottom to send messages into the selected connection.

### TUI

1. Navigate to the **WebSocket** tab.
2. Browse the connection list and select a connection to view its messages.

### MCP

When using the MCP server with an AI assistant, you can query WebSocket data:

- "Show me all WebSocket connections"
- "Get messages from the MQTT WebSocket connection"
- "Find WebSocket messages that contain 'error'"

The `get_websocket_messages` tool supports filtering by connection URI. See the [MCP Server](./mcp-server.md) documentation for details.
