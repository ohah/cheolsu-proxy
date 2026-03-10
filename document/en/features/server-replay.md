# Server Replay

Server Replay lets you save captured HTTP responses and automatically return them when matching requests arrive, without ever contacting the real server.

This solves a common problem in development and testing: you need consistent, repeatable API responses but the server is unavailable, unstable, slow, or returns different data each time. With Server Replay, you capture a known-good response once and use it indefinitely.

---

## When to Use Server Replay

**Offline development.** You captured API responses while connected to the server yesterday. Today you are working from a location without network access. Server Replay returns those same responses so your app functions normally.

**Eliminating flaky test dependencies.** An integration test fails intermittently because the backend returns slightly different data each run. Save a stable response as a replay entry and the test gets the same data every time.

**Bypassing slow endpoints.** A reporting API takes 10 seconds to respond. During frontend development, you don't need live data -- you need fast iteration. Server Replay returns the saved response instantly.

**Reproducing bugs.** A user reported a rendering bug caused by a specific API response. Save that response as a replay entry and the bug is reproducible on demand.

**Removing backend dependencies.** The backend team hasn't deployed the latest API changes yet. You have a sample response from their documentation. Map it as a replay entry and continue frontend work without waiting.

---

## How Matching Works

When Server Replay is enabled and a new HTTP request arrives, Cheolsu Proxy checks whether the request matches any saved entry. Matching is based on:

1. **HTTP method** -- The request method (GET, POST, PUT, etc.) must match exactly.
2. **URL** -- The full request URL must match exactly.

If a match is found, the stored response is returned immediately to the client. The request is never forwarded to the real server.

If no match is found, the request proceeds normally through the proxy to the server.

Duplicate entries for the same method and URL combination are not allowed. If you save a new response for an endpoint that already has an entry, the existing entry must be removed first.

---

## What Gets Stored

Each Server Replay entry contains the complete response that will be returned:

| Field | Description |
| --- | --- |
| HTTP method | The method of the original request (GET, POST, etc.) |
| URL | The full URL of the original request |
| Status code | The HTTP status code of the saved response |
| Response headers | All response headers, including content type |
| Response body | The complete response body |
| Header count | Number of headers in the stored response |
| Body size | Size of the stored response body |

---

## Usage

### Desktop

1. Select **Server Replay** from the sidebar.
2. To add an entry, go to the traffic table on the network dashboard, find the transaction you want to replay, and add it to Server Replay.
3. The Server Replay panel shows all saved entries with their method, URL, status code, header count, and body size.
4. Use the toggle at the top to enable or disable Server Replay globally.
5. Delete individual entries by selecting them and clicking the delete button, or clear all entries at once.

### Workflow Example

A typical workflow looks like this:

1. Run your application with Cheolsu Proxy active.
2. Exercise the feature you are working on so that the relevant API calls appear in the traffic table.
3. Find the responses you want to preserve and add them to Server Replay.
4. Enable Server Replay.
5. Continue development. The proxy now returns your saved responses for those endpoints instantly, regardless of the server's state.
6. When you need fresh data again, disable Server Replay and the requests will flow through to the real server.
