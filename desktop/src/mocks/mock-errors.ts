import type { HttpTransaction } from "@/entities/proxy";

import { jsonBody, textBody, tx } from "./mock-helpers";

/** 에러 응답 트랜잭션 (4xx, 5xx, 3xx 리다이렉트) */
export const MOCK_ERROR_TRANSACTIONS: HttpTransaction[] = [
  // 10. 404 Not Found
  tx(
    {
      method: "GET",
      uri: "https://api.example.com/v1/products/99999",
      headers: { accept: "application/json" },
    },
    {
      status: 404,
      ...jsonBody({
        error: "Not Found",
        message: "Product with id 99999 does not exist",
      }),
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
      ...jsonBody({
        error: "Internal Server Error",
        message: "Database connection timeout",
      }),
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
      headers: {
        location: "https://example.com/new-page",
        "content-type": "text/html",
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
];
