# Traffic Diff

## Overview

Traffic Diff compares two HTTP transactions side-by-side, visually highlighting differences in headers, body, status codes, and more. Quickly identify exactly what changed between two requests or responses.

Useful for comparing API responses before and after changes, verifying differences across environments (dev/staging/production), and contrasting normal vs. abnormal requests during bug investigation.

---

## Comparison Items

### Request Comparison

| Item            | Description                              |
| --------------- | ---------------------------------------- |
| **HTTP Method** | Detects method changes (GET, POST, etc.) |
| **URL**         | Request URL differences                  |
| **Headers**     | Added, removed, and modified headers     |
| **Body**        | Request body differences                 |

### Response Comparison

| Item            | Description                                   |
| --------------- | --------------------------------------------- |
| **Status Code** | HTTP status code changes                      |
| **Headers**     | Added, removed, and modified response headers |
| **Body**        | Response body differences                     |

---

## Comparison Modes

### Header Comparison

Headers are compared as key-value pairs and classified into three change types:

- **Added**: Newly added headers
- **Removed**: Deleted headers
- **Modified**: Headers with changed values for the same key

### JSON Comparison

When bodies are JSON, structural comparison is performed. Changes are shown with JSON paths (e.g., `.data.users[0].name`), change types, and before/after values.

### Text Comparison

Non-JSON text bodies are compared line-by-line, classified as additions, deletions, or unchanged.

### Binary Comparison

Binary bodies show only size differences.

---

## Usage

### Desktop

1. Select the first transaction to compare from the traffic list
2. Select the second transaction to run the Diff comparison
3. Review differences per request/response

### MCP

```
"Compare transactions A and B"
```
