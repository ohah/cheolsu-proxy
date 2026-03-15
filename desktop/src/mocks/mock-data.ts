import type { HttpRequest, HttpResponse, HttpTransaction } from "@/entities/proxy";
import type { DataType } from "@/entities/proxy";

let idCounter = 0;
function nextId(): string {
  idCounter += 1;
  return `mock-${idCounter}-${Date.now()}`;
}

function makeRequest(
  overrides: Partial<HttpRequest> & { method: string; uri: string },
): HttpRequest {
  const id = overrides.id ?? nextId();
  return {
    method: overrides.method,
    uri: overrides.uri,
    version: overrides.version ?? "HTTP/2.0",
    headers: overrides.headers ?? { "content-type": "application/json" },
    body: overrides.body ?? null,
    time: overrides.time ?? Date.now(),
    id,
    data_type: overrides.data_type ?? "Json",
    body_json: overrides.body_json,
    body_size: overrides.body_size ?? 0,
  };
}

function makeResponse(
  id: string,
  overrides: Partial<HttpResponse> & { status: number },
): HttpResponse {
  return {
    status: overrides.status,
    version: overrides.version ?? "HTTP/2.0",
    headers: overrides.headers ?? { "content-type": "application/json" },
    body: overrides.body ?? null,
    time: overrides.time ?? Date.now() + 50,
    id,
    data_type: overrides.data_type ?? "Json",
    body_json: overrides.body_json,
    body_size: overrides.body_size ?? 0,
  };
}

function tx(
  req: Partial<HttpRequest> & { method: string; uri: string },
  res?: Partial<HttpResponse> & { status: number },
): HttpTransaction {
  const request = makeRequest(req);
  const response = res ? makeResponse(request.id, res) : null;
  return { request, response };
}

const jsonBody = (obj: any): { body_json: any; body_size: number; data_type: DataType } => ({
  body_json: obj,
  body_size: JSON.stringify(obj).length,
  data_type: "Json",
});

const textBody = (
  text: string,
  dataType: DataType = "Text",
): { body: Uint8Array; body_size: number; data_type: DataType } => ({
  body: new TextEncoder().encode(text),
  body_size: text.length,
  data_type: dataType,
});

const htmlSnippet = `<!DOCTYPE html><html><head><title>Example</title></head><body><h1>Hello World</h1></body></html>`;
const cssSnippet = `body { margin: 0; padding: 0; font-family: sans-serif; }`;
const jsSnippet = `(function(){console.log("loaded");})();`;

export const MOCK_TRANSACTIONS: HttpTransaction[] = [
  // 1. REST API - GET users
  tx(
    {
      method: "GET",
      uri: "https://api.example.com/v1/users?page=1&limit=20",
      headers: {
        accept: "application/json",
        authorization: "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
        "user-agent": "Mozilla/5.0",
      },
      ...jsonBody(null),
    },
    {
      status: 200,
      headers: {
        "content-type": "application/json",
        "x-total-count": "142",
        "x-request-id": "req-abc-123",
      },
      ...jsonBody({
        data: [
          { id: 1, name: "Alice", email: "alice@example.com", role: "admin" },
          { id: 2, name: "Bob", email: "bob@example.com", role: "user" },
          { id: 3, name: "Charlie", email: "charlie@example.com", role: "user" },
        ],
        pagination: { page: 1, limit: 20, total: 142 },
      }),
    },
  ),

  // 2. REST API - POST create user
  tx(
    {
      method: "POST",
      uri: "https://api.example.com/v1/users",
      headers: {
        "content-type": "application/json",
        authorization: "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
      },
      ...jsonBody({ name: "Dave", email: "dave@example.com", role: "user" }),
    },
    {
      status: 201,
      headers: { "content-type": "application/json", location: "/v1/users/4" },
      ...jsonBody({
        id: 4,
        name: "Dave",
        email: "dave@example.com",
        role: "user",
        created_at: "2026-03-06T10:00:00Z",
      }),
    },
  ),

  // 3. REST API - PUT update
  tx(
    {
      method: "PUT",
      uri: "https://api.example.com/v1/users/2",
      headers: { "content-type": "application/json", authorization: "Bearer token..." },
      ...jsonBody({ name: "Bob Updated", role: "admin" }),
    },
    {
      status: 200,
      ...jsonBody({ id: 2, name: "Bob Updated", email: "bob@example.com", role: "admin" }),
    },
  ),

  // 4. REST API - DELETE
  tx(
    {
      method: "DELETE",
      uri: "https://api.example.com/v1/users/3",
      headers: { authorization: "Bearer token..." },
    },
    { status: 204, body_size: 0, data_type: "Empty" },
  ),

  // 5. PATCH
  tx(
    {
      method: "PATCH",
      uri: "https://api.example.com/v1/users/1/settings",
      headers: { "content-type": "application/json" },
      ...jsonBody({ theme: "dark", notifications: true }),
    },
    {
      status: 200,
      ...jsonBody({ theme: "dark", notifications: true, updated_at: "2026-03-06T10:05:00Z" }),
    },
  ),

  // 6. HTML page
  tx(
    {
      method: "GET",
      uri: "https://www.example.com/",
      headers: { accept: "text/html", "user-agent": "Mozilla/5.0" },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: { "content-type": "text/html; charset=utf-8", "cache-control": "max-age=3600" },
      ...textBody(htmlSnippet, "Html"),
    },
  ),

  // 7. CSS
  tx(
    {
      method: "GET",
      uri: "https://www.example.com/assets/style.css",
      headers: { accept: "text/css" },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: { "content-type": "text/css", "cache-control": "max-age=86400" },
      ...textBody(cssSnippet, "Css"),
    },
  ),

  // 8. JavaScript
  tx(
    {
      method: "GET",
      uri: "https://www.example.com/assets/app.bundle.js",
      headers: { accept: "*/*" },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: { "content-type": "application/javascript" },
      ...textBody(jsSnippet, "Javascript"),
    },
  ),

  // 9. Image (PNG)
  tx(
    {
      method: "GET",
      uri: "https://cdn.example.com/images/logo.png",
      headers: { accept: "image/*" },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: { "content-type": "image/png", "content-length": "24680" },
      data_type: "Image",
      body_size: 24680,
    },
  ),

  // 10. 404 Not Found
  tx(
    {
      method: "GET",
      uri: "https://api.example.com/v1/products/99999",
      headers: { accept: "application/json" },
    },
    {
      status: 404,
      ...jsonBody({ error: "Not Found", message: "Product with id 99999 does not exist" }),
    },
  ),

  // 11. 500 Internal Server Error
  tx(
    {
      method: "POST",
      uri: "https://api.example.com/v1/orders",
      headers: { "content-type": "application/json" },
      ...jsonBody({ product_id: 1, quantity: 999999 }),
    },
    {
      status: 500,
      ...jsonBody({ error: "Internal Server Error", message: "Database connection timeout" }),
    },
  ),

  // 12. 301 Redirect
  tx(
    {
      method: "GET",
      uri: "http://example.com/old-page",
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 301,
      headers: { location: "https://example.com/new-page", "content-type": "text/html" },
      data_type: "Empty",
      body_size: 0,
    },
  ),

  // 13. GraphQL query
  tx(
    {
      method: "POST",
      uri: "https://api.example.com/graphql",
      headers: { "content-type": "application/json" },
      ...jsonBody({
        query: "query { users(first: 10) { edges { node { id name email } } } }",
        variables: {},
      }),
    },
    {
      status: 200,
      ...jsonBody({
        data: {
          users: {
            edges: [
              { node: { id: "1", name: "Alice", email: "alice@example.com" } },
              { node: { id: "2", name: "Bob", email: "bob@example.com" } },
            ],
          },
        },
      }),
    },
  ),

  // 14. OPTIONS preflight
  tx(
    {
      method: "OPTIONS",
      uri: "https://api.example.com/v1/users",
      headers: {
        origin: "https://app.example.com",
        "access-control-request-method": "POST",
        "access-control-request-headers": "content-type,authorization",
      },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 204,
      headers: {
        "access-control-allow-origin": "https://app.example.com",
        "access-control-allow-methods": "GET,POST,PUT,DELETE,PATCH",
        "access-control-allow-headers": "content-type,authorization",
        "access-control-max-age": "86400",
      },
      data_type: "Empty",
      body_size: 0,
    },
  ),

  // 15. HEAD request
  tx(
    {
      method: "HEAD",
      uri: "https://cdn.example.com/files/report-2026.pdf",
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: {
        "content-type": "application/pdf",
        "content-length": "1048576",
        "last-modified": "Thu, 05 Mar 2026 12:00:00 GMT",
      },
      data_type: "Empty",
      body_size: 0,
    },
  ),

  // 16. 401 Unauthorized
  tx(
    {
      method: "GET",
      uri: "https://api.example.com/v1/admin/dashboard",
      headers: { authorization: "Bearer expired-token" },
    },
    {
      status: 401,
      ...jsonBody({ error: "Unauthorized", message: "Token has expired" }),
    },
  ),

  // 17. 403 Forbidden
  tx(
    {
      method: "DELETE",
      uri: "https://api.example.com/v1/admin/system-config",
      headers: { authorization: "Bearer valid-user-token" },
    },
    {
      status: 403,
      ...jsonBody({ error: "Forbidden", message: "Insufficient permissions" }),
    },
  ),

  // 18. 429 Rate Limited
  tx(
    {
      method: "GET",
      uri: "https://api.example.com/v1/search?q=test",
      headers: { accept: "application/json" },
    },
    {
      status: 429,
      headers: {
        "content-type": "application/json",
        "retry-after": "30",
        "x-ratelimit-limit": "100",
        "x-ratelimit-remaining": "0",
      },
      ...jsonBody({
        error: "Too Many Requests",
        message: "Rate limit exceeded. Retry after 30 seconds",
      }),
    },
  ),

  // 19. WebSocket upgrade (CONNECT-like)
  tx(
    {
      method: "GET",
      uri: "wss://ws.example.com/realtime",
      headers: {
        upgrade: "websocket",
        connection: "Upgrade",
        "sec-websocket-key": "dGhlIHNhbXBsZSBub25jZQ==",
        "sec-websocket-version": "13",
      },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 101,
      headers: {
        upgrade: "websocket",
        connection: "Upgrade",
        "sec-websocket-accept": "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
      },
      data_type: "Empty",
      body_size: 0,
    },
  ),

  // 20. XML response
  tx(
    {
      method: "GET",
      uri: "https://api.example.com/v1/feed.xml",
      headers: { accept: "application/xml" },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: { "content-type": "application/xml" },
      ...textBody(
        `<?xml version="1.0"?><feed><title>Example Feed</title><entry><title>Post 1</title></entry></feed>`,
        "Xml",
      ),
    },
  ),

  // 21. Large JSON payload (products listing)
  tx(
    {
      method: "GET",
      uri: "https://api.example.com/v1/products?category=electronics&sort=price",
      headers: { accept: "application/json", "accept-encoding": "gzip, deflate, br" },
    },
    {
      status: 200,
      headers: {
        "content-type": "application/json",
        "content-encoding": "gzip",
        "x-total-count": "856",
      },
      ...jsonBody({
        data: Array.from({ length: 10 }, (_, i) => ({
          id: i + 1,
          name: `Product ${i + 1}`,
          price: Math.round(Math.random() * 100000) / 100,
          category: "electronics",
          in_stock: Math.random() > 0.3,
        })),
        pagination: { page: 1, limit: 10, total: 856 },
      }),
    },
  ),

  // 22. File upload (multipart)
  tx(
    {
      method: "POST",
      uri: "https://api.example.com/v1/files/upload",
      headers: {
        "content-type": "multipart/form-data; boundary=----WebKitFormBoundary",
        authorization: "Bearer token...",
      },
      data_type: "Binary",
      body_size: 512000,
    },
    {
      status: 200,
      ...jsonBody({
        file_id: "file-abc-123",
        filename: "document.pdf",
        size: 512000,
        url: "https://cdn.example.com/files/file-abc-123.pdf",
      }),
    },
  ),

  // 23. Pending request (no response yet)
  tx({
    method: "GET",
    uri: "https://api.example.com/v1/long-running-task/status",
    headers: { accept: "application/json" },
    time: Date.now(),
  }),

  // 24. 502 Bad Gateway
  tx(
    {
      method: "GET",
      uri: "https://api.example.com/v1/health",
      headers: { accept: "application/json" },
    },
    {
      status: 502,
      headers: { "content-type": "text/html" },
      ...textBody("<html><body><h1>502 Bad Gateway</h1></body></html>", "Html"),
    },
  ),

  // 25. Text/plain
  tx(
    {
      method: "GET",
      uri: "https://example.com/robots.txt",
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 200,
      headers: { "content-type": "text/plain" },
      ...textBody("User-agent: *\nDisallow: /admin/\nDisallow: /api/\nAllow: /", "Text"),
    },
  ),
];
