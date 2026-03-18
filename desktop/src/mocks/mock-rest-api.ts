import type { HttpTransaction } from "@/entities/proxy";

import { jsonBody, tx } from "./mock-helpers";

/** REST API CRUD 트랜잭션 (GET, POST, PUT, DELETE, PATCH) */
export const MOCK_REST_API_TRANSACTIONS: HttpTransaction[] = [
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
          {
            id: 3,
            name: "Charlie",
            email: "charlie@example.com",
            role: "user",
          },
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
      headers: {
        "content-type": "application/json",
        authorization: "Bearer token...",
      },
      ...jsonBody({ name: "Bob Updated", role: "admin" }),
    },
    {
      status: 200,
      ...jsonBody({
        id: 2,
        name: "Bob Updated",
        email: "bob@example.com",
        role: "admin",
      }),
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
      ...jsonBody({
        theme: "dark",
        notifications: true,
        updated_at: "2026-03-06T10:05:00Z",
      }),
    },
  ),
];
