---
title: Key Features
description: All features and characteristics of Cheolsu Proxy
---

# Key Features

Cheolsu Proxy provides various features for network traffic monitoring and analysis.

## 🔐 HTTP/HTTPS Support

### HTTP Traffic

- Real-time capture of all HTTP requests and responses
- Analysis of request headers, body, and query parameters
- View response status codes, headers, and body

### HTTPS Traffic

- Decoding of TLS/SSL encrypted traffic
- Safe man-in-the-middle attack using auto-generated CA certificates
- Support for various TLS versions (1.0, 1.1, 1.2, 1.3)

## 🔒 TLS 1.0/1.1 Legacy Support

### Hybrid TLS Handler

- Support for both modern and legacy TLS versions
- Automatic TLS version detection and appropriate handler selection
- Compatibility with legacy systems

### Supported TLS Versions

- TLS 1.0 (legacy)
- TLS 1.1 (legacy)
- TLS 1.2 (current standard)
- TLS 1.3 (latest standard)

## 🖱️ Intuitive GUI

### Dark/Light Theme

- Theme selection based on user preference
- Automatic system theme detection
- Color scheme that reduces eye strain

### Responsive Design

- Optimized for various screen sizes
- Usable on mobile devices
- UI/UX considering accessibility

## ⌨️ Advanced Configuration Options

### Network Settings

- **Listening Address**: Default `127.0.0.1` (localhost)
- **Port**: Default `8100`, customizable
- **Binding Interface**: Select specific network interface

### Proxy Modes

- **Transparent Proxy**: Proxy that clients don't recognize
- **Explicit Proxy**: Proxy that clients directly configure
- **Reverse Proxy**: Proxy that operates in front of servers

## 🔍 Detailed Traffic Analysis

### Request Information

- **HTTP Method**: GET, POST, PUT, DELETE, PATCH, etc.
- **URL**: Full path and query parameters
- **Headers**: All HTTP header information
- **Body**: Request payload (JSON, XML, Form Data, etc.)
- **Timestamp**: Accurate request time

### Response Information

- **Status Code**: HTTP status codes (200, 404, 500, etc.)
- **Response Time**: Time from request to response
- **Headers**: All response headers
- **Body**: Response data
- **Size**: Request/response data size

## 🎯 Powerful Filtering

### Filter by Method

- GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS
- Multiple method selection possible
- Real-time filtering

### Filter by Status Code

- 2xx (Success): 200, 201, 204, etc.
- 3xx (Redirection): 301, 302, 304, etc.
- 4xx (Client Error): 400, 401, 404, etc.
- 5xx (Server Error): 500, 502, 503, etc.

### URL Pattern Search

- Regular expression support
- Case-sensitive/insensitive options
- Partial and exact matching

## ❌ Request Management

### Individual Request Deletion

- Selectively remove specific requests
- Cannot undo after deletion (improvement planned)
- Deleted requests excluded from statistics

### Clear All

- Delete all requests at once
- Memory usage optimization
- Start new session

## 🔐 Security Features

### User-Specific Unique CA Certificates

- Unique CA certificate generation for each user
- Private keys not included in binary
- Enhanced security certificate management

### Automatic Certificate Generation

- Automatic CA certificate generation on first run
- Ready to use without user intervention
- Automatic certificate renewal (planned)

### Local Only

- All data stored locally only
- No data transmission to external servers
- Privacy protection guaranteed

## 📊 Performance Optimization

### Memory Efficiency

- Streaming data processing
- Stable handling of large traffic
- Memory usage monitoring

### Fast Response

- Asynchronous processing to prevent UI blocking
- Real-time updates
- Minimal delay

## 🚀 Extensibility

### Plugin System (Planned)

- Add custom handlers
- Support for new protocols
- Custom analysis tools

### API Provision (Planned)

- External integration through REST API
- Webhook support
- Integration with other tools

## 📱 Platform Support

### Current Support

- **macOS**: Full support
- **Windows**: In development

### Future Plans

- Mobile apps (iOS, Android)
- Web version
- Docker container support
