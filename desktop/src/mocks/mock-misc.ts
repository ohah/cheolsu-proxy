import type { HttpTransaction } from "@/entities/proxy";

import { jsonBody, textBody, tx } from "./mock-helpers";

/** 기타 트랜잭션 (OPTIONS, HEAD, XML, 대용량 JSON, 파일 업로드, 대기중 요청, 텍스트) */
export const MOCK_MISC_TRANSACTIONS: HttpTransaction[] = [
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
      headers: {
        accept: "application/json",
        "accept-encoding": "gzip, deflate, br",
      },
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
