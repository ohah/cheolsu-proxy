# GraphQL

## Overview

Cheolsu Proxy automatically detects and analyzes GraphQL requests and responses. While all GraphQL requests appear as the same endpoint (`/graphql`) in standard HTTP traffic lists, the GraphQL dashboard classifies traffic by operation name, type, and variables.

---

## Auto-Detection

Requests are automatically detected as GraphQL when any of the following conditions are met:

- POST request body contains a `query` field
- Content-Type includes `graphql`
- GET request query parameters include `query`
- Batch queries (array-format request body)

---

## Analysis Information

### Operation Details

| Field | Description |
| --- | --- |
| **Operation Type** | Query, Mutation, Subscription |
| **Operation Name** | e.g., `GetUser` from `query GetUser { ... }` |
| **Query** | Full GraphQL query string |
| **Variables** | Variables passed to the query (JSON) |
| **Endpoint** | Request URL |

### Response Analysis

| Field | Description |
| --- | --- |
| **data** | Response data |
| **errors** | GraphQL error list (message, locations, path) |
| **extensions** | Server extension data (performance metrics, etc.) |

### Batch Queries

Supports batch queries where multiple GraphQL operations are included in a single HTTP request. Each operation is displayed individually with its batch index.

---

## Use Cases

### Operation Filtering

Distinguish between Query and Mutation operations to monitor only data-changing operations, or filter by specific operation names to track specific API calls.

### Error Tracking

GraphQL errors often return with HTTP status code 200, making them hard to detect with standard HTTP monitoring. The GraphQL dashboard parses the `errors` field in responses to clearly display errors.

### N+1 Query Detection

Integrated with Analytics to detect repeated queries to the same endpoint and identify N+1 problems.

---

## Usage

### Desktop

1. Select **GraphQL** from the sidebar
2. View operations categorized by Query/Mutation/Subscription
3. Select an operation to see query, variables, and response details

### MCP

```
"Show me the GraphQL mutations"
"Check the GetUser query response"
```
