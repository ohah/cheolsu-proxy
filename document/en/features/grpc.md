# gRPC / Protobuf

## Overview

Cheolsu Proxy automatically detects gRPC traffic and decodes Protobuf messages into human-readable format. Messages are auto-decoded using Wire Type analysis without requiring `.proto` definition files.

---

## Auto-Detection

gRPC traffic is automatically detected by:

- Content-Type starting with `application/grpc` or `application/grpc+proto`
- Content-Type subtype extraction (e.g., `proto`, `json`)

---

## Analysis Information

### Metadata

| Field | Description |
| --- | --- |
| **Service Name** | Extracted from URI path (e.g., `/package.ServiceName/MethodName`) |
| **Method Name** | Called RPC method |
| **Status Code** | gRPC status code (OK, CANCELLED, UNKNOWN, etc. — 16 codes) |
| **Status Message** | gRPC status message (percent-encoding auto-decoded) |
| **Streaming Type** | Unary, Server Streaming, Client Streaming, Bidirectional |

### gRPC Status Codes

| Code | Name | Description |
| --- | --- | --- |
| 0 | OK | Success |
| 1 | CANCELLED | Request cancelled |
| 2 | UNKNOWN | Unknown error |
| 3 | INVALID_ARGUMENT | Invalid argument |
| 4 | DEADLINE_EXCEEDED | Timeout |
| 5 | NOT_FOUND | Resource not found |
| 7 | PERMISSION_DENIED | Permission denied |
| 13 | INTERNAL | Internal server error |
| 14 | UNAVAILABLE | Service unavailable |
| 16 | UNAUTHENTICATED | Authentication failed |

### Protobuf Frame Parsing

gRPC bodies use the Length-Prefixed Message format:

```
[Compressed-Flag (1 byte)] [Message-Length (4 bytes, Big Endian)] [Message]
```

Each frame's compression status and message data are parsed separately. Wire Type-based field auto-decoding allows inspecting message structure without `.proto` files.

---

## Use Cases

### Microservice Debugging

Monitor inter-service gRPC communication and filter requests by service/method to trace specific RPC calls.

### Error Diagnosis

Check gRPC status codes and messages to identify request failure causes. Track statuses like DEADLINE_EXCEEDED and UNAVAILABLE to diagnose network issues or service outages.

### Payload Inspection

Examine Protobuf message field structures to verify that request/response data is serialized as expected.

---

## Usage

### Desktop

Select a gRPC request in the traffic log to view service/method info, status codes, and decoded Protobuf messages in the transaction detail view.

### MCP

```
"Show me gRPC traffic"
"Find gRPC requests with DEADLINE_EXCEEDED status"
```
