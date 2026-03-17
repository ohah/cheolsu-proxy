# Server-Sent Events (SSE)

## Overview

Cheolsu Proxy captures and analyzes Server-Sent Events (SSE) streams in real-time. SSE is a protocol for server-to-client unidirectional event delivery, widely used for AI chatbot streaming responses, real-time notifications, stock price updates, and more.

SSE responses are delivered with the `text/event-stream` Content-Type. Cheolsu Proxy automatically detects these and parses them into individual events.

---

## Event Structure

Each event is parsed according to the SSE standard:

| Field | Description |
| --- | --- |
| **data** | Event data (can span multiple lines) |
| **event** | Event type (default: `message`) |
| **id** | Event identifier (used as `Last-Event-ID` header on reconnection) |
| **retry** | Reconnection interval (milliseconds) |

Comments (lines starting with `:`) are handled separately and not included in event data.

---

## Key Features

### Connection List

Displays active and closed SSE connections in chronological order.

- Connected/disconnected status
- URI for each connection
- Number of received events

### Event Viewer

Shows all events for a selected connection in chronological order.

- **Event type display**: Classification by `event` field value
- **Data content**: JSON data is auto-formatted
- **Sequence number**: Event receipt order tracking
- **Size information**: Data size of each event

### Scripting Integration

Use the `cheolsu.onSseEvent` hook to programmatically process SSE events. Events can be modified or filtered (dropped).

---

## Use Cases

### AI Streaming Response Debugging

AI APIs like OpenAI and Claude send streaming responses via SSE. Inspect each chunk's content and timing to verify streaming UI behavior.

### Real-time Notification Verification

Examine event types, data, and ordering from the server to verify that notification systems work as intended.

---

## Usage

### Desktop

1. Select **SSE** from the sidebar
2. View SSE connection list in the left panel
3. Select a connection to see its events in the right panel
4. Select an event to view detailed data

### MCP

```
"Show me the SSE connection list"
"Check the recent SSE events"
```
