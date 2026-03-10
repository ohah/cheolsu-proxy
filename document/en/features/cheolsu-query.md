# Cheolsu-Query Language

Cheolsu-Query is a filtering language for narrowing down network traffic displayed in Cheolsu Proxy. When you are capturing hundreds or thousands of requests, the query bar lets you quickly isolate the traffic you care about by filtering on HTTP method, status code, and URL.

The query language is built into the Monaco Editor (the same editor engine used by VS Code), which provides syntax highlighting, auto-completion, and real-time validation as you type.

---

## Filter Keywords

| Keyword | Description | Example |
| --- | --- | --- |
| `method` | Filter by a single HTTP method | `method="GET"` |
| `methods` | Filter by multiple HTTP methods | `methods="GET,POST"` |
| `status` | Filter by HTTP status code or category | `status="2xx,404"` |
| `url` | Filter by URL content | `url\|="api"` |

---

## Operators

### Comparison Operators

| Operator | Meaning | Example |
| --- | --- | --- |
| `=` | Exact match | `method="GET"` |
| `\|=` | Contains (case-sensitive) | `url\|="api"` |
| `\|~` | Contains (case-insensitive) | `url\|~="API"` |
| `!=` | Not equal | `method!="OPTIONS"` |
| `!~` | Does not contain | `url!~="static"` |

### Logical Operators

| Operator | Meaning | Example |
| --- | --- | --- |
| `and` | Both conditions must be true (default) | `method="GET" and status="2xx"` |
| `or` | Either condition must be true | `method="GET" or status="5xx"` |

Parentheses can be used to group conditions:

```text
(method="GET" or method="POST") and status="2xx"
```

---

## Usage Examples

### Filter by HTTP Method

```text
method="GET"
```

Show only GET requests.

```text
methods="GET,POST"
```

Show GET and POST requests.

```text
method!="OPTIONS"
```

Hide preflight OPTIONS requests that clutter the traffic list during CORS development.

### Filter by Status Code

```text
status="2xx"
```

Show only successful responses (200-299).

```text
status="4xx,5xx"
```

Show only error responses. Useful for identifying failing requests.

```text
status="404"
```

Show only 404 Not Found responses.

### Filter by URL

```text
url|="api"
```

Show requests whose URL contains "api" (case-sensitive).

```text
url|~="graphql"
```

Show requests whose URL contains "graphql" (case-insensitive).

```text
url!~="analytics"
```

Hide analytics requests.

### Combined Filters

```text
method="GET" and status="2xx" and url|="api"
```

Show only successful GET requests to API endpoints.

```text
url|="api" and method!="OPTIONS"
```

Show API requests while hiding CORS preflight requests.

```text
url!~="google-analytics" and url!~="ads"
```

Hide analytics and advertising traffic.

```text
(method="POST" or method="PUT") and status="5xx"
```

Find write operations that resulted in server errors.

```text
url|="localhost" or url|="127.0.0.1"
```

Show only requests to local development servers.

---

## Editor Features

### Auto-completion

Auto-completion is triggered when typing `"`, `,`, ` `, `|`, `!`, or `=`. The editor suggests:

- **Keywords** -- `method`, `methods`, `status`, `url`
- **HTTP methods** -- `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`, `CONNECT`, `TRACE`
- **Status codes** -- Categories like `2xx`, `4xx`, `5xx` as well as specific codes like `200`, `401`, `404`, `500`
- **Operators** -- `=`, `|=`, `|~`, `!=`, `!~`

### Keyboard Shortcuts

- **Cmd/Ctrl + Enter** -- Apply the current query.

### Theme Support

The editor supports both light and dark themes and switches automatically based on the system theme.

---

## Common Mistakes

**Missing quotes around values.** Values must always be quoted with double quotes.

```text
method="GET"       # Correct
method=GET         # Incorrect
```

**Using the wrong equality operator.** Use single `=`, not double `==`.

```text
method="GET"       # Correct
method=="GET"      # Incorrect
```

**Using programming-style logical operators.** Use `and` and `or`, not `&&` and `||`.

```text
method="GET" and status="2xx"    # Correct
method="GET" && status="2xx"     # Incorrect
```

---

## Quick Reference

```text
method="GET"                          # Exact method match
methods="GET,POST"                    # Multiple methods
status="2xx"                          # Status code category
status="200,404"                      # Specific status codes
url|="api"                            # URL contains (case-sensitive)
url|~="api"                           # URL contains (case-insensitive)
url!~="ads"                           # URL does not contain
method="GET" and status="2xx"         # AND condition
method="GET" or method="POST"         # OR condition
(method="GET" or method="POST") and status="2xx"  # Grouped conditions
```
