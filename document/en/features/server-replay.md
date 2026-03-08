# Server Replay

Store captured HTTP responses and automatically return them when identical requests arrive, without forwarding to the server.

---

## How It Works

1. Save a captured HTTP transaction's response as a Server Replay entry
2. When an identical request (method + URL match) arrives
3. Automatically return the stored response (bypassing the server)

---

## Use Cases

- **Offline Testing**: Test with previous responses without a server
- **Response Time Reduction**: Instantly return slow API responses
- **Unstable Server Replacement**: Preserve working responses from intermittently failing servers
- **API Mocking**: Remove backend dependencies during development

---

## Usage

### Desktop

1. Select **Server Replay** from the sidebar
2. Add transactions with responses from the network dashboard
3. Control with the enable/disable toggle
4. Delete individual entries or clear all

### Entry Information

Each entry displays:

- HTTP method
- URL
- Response status code
- Header count
- Body size

> Duplicate entries for the same URL are automatically prevented.
