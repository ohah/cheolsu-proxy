# Cheolsu-Query Language

A dedicated query language for filtering network requests in Cheolsu Proxy.

## 🎯 Overview

Cheolsu-Query is a powerful query language based on Monaco Editor, designed to efficiently filter and analyze network traffic.

### Key Features

- **Monaco Editor based**: Uses the same editor engine as VS Code
- **Syntax highlighting**: Color-coded keywords, operators, and strings
- **Auto-completion**: Automatic suggestions for keywords, HTTP methods, and status codes
- **Real-time validation**: Live query syntax validation
- **Theme support**: Automatic light/dark theme switching

## 🔧 Filter Keywords

| Keyword   | Description                     | Example              |
| --------- | ------------------------------- | -------------------- |
| `method`  | HTTP method filtering           | `method="GET"`       |
| `methods` | Multiple HTTP methods filtering | `methods="GET,POST"` |
| `status`  | HTTP status code filtering      | `status="2xx,404"`   |
| `url`     | URL pattern filtering           | `url \|= "api"`      |

## ⚙️ Operators

### Comparison Operators

| Operator | Description                 | Example             |
| -------- | --------------------------- | ------------------- |
| `=`      | Exact match                 | `method="GET"`      |
| `\|=`    | Contains (case-sensitive)   | `url \|= "api"`     |
| `\|~`    | Contains (case-insensitive) | `url \|~ "API"`     |
| `!=`     | Not equal                   | `method!="OPTIONS"` |
| `!~`     | Does not contain            | `url!~="static"`    |

### Logical Operators

| Operator | Description             | Example                         |
| -------- | ----------------------- | ------------------------------- |
| `and`    | AND condition (default) | `method="GET" and status="2xx"` |
| `or`     | OR condition            | `method="GET" or status="5xx"`  |

## 📋 Usage Examples

### Basic Filtering

```cheolsu-query
# Filter only GET requests
method="GET"

# Filter only successful responses
status="2xx"

# Filter API-related requests
url|="api"

# Exclude error responses
status!="4xx,5xx"
```

### Complex Conditions

```cheolsu-query
# Multiple HTTP methods
method="GET,POST"

# Complex AND conditions
method="GET" and status="2xx" and url|="api"

# OR conditions
method="POST" or status="5xx"

# Complex conditions
(method="GET" or method="POST") and status="2xx" and url!~="static"
```

### Real-world Scenarios

```cheolsu-query
# Monitor only API requests
url|="api" and method!="OPTIONS"

# Analyze error responses
status="4xx,5xx"

# Exclude specific domains
url!~="google-analytics" and url!~="ads"

# Development environment APIs only
url|="localhost" or url|="127.0.0.1"
```

## 🎨 Theme Support

- **Light theme**: `cheolsu-light`
- **Dark theme**: `cheolsu-dark`
- **Auto theme switching**: Automatically changes based on system theme

## ⌨️ Keyboard Shortcuts

- **⌘ + Enter**: Apply query
- **Auto-completion**: Triggered by typing `"`, `,`, ` `, `|`, `!`, `=`

## 🔍 Auto-completion Features

### Keyword Suggestions

- `method`, `methods`, `status`, `url`

### HTTP Method Suggestions

- `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`, `CONNECT`, `TRACE`

### Status Code Suggestions

- `1xx` (Informational)
- `2xx` (Success)
- `3xx` (Redirection)
- `4xx` (Client Error)
- `5xx` (Server Error)
- Specific codes: `200`, `201`, `204`, `400`, `401`, `403`, `404`, `500`, `502`, `503`

### Operator Suggestions

- `=`, `|=`, `|~`, `!=`, `!~`

## 💡 Usage Tips

### 1. Efficient Filtering

```cheolsu-query
# Use exact conditions
method="GET"  # ✅ Good
method|="GET" # ❌ Unnecessary pattern matching
```

### 2. Performance Optimization

```cheolsu-query
# Place specific conditions first
url|="api" and method="GET"  # ✅ Good
method="GET" and url|="api"  # ❌ URL pattern is more selective
```

### 3. Improved Readability

```cheolsu-query
# Use parentheses for complex conditions
(method="GET" or method="POST") and status="2xx"
```

## 🚀 Advanced Usage

### Regex Patterns (Future Support)

```cheolsu-query
# URL pattern matching
url~="^/api/v[0-9]+/"

# Complex status code patterns
status~="^[45][0-9][0-9]$"
```

### Time-based Filtering (Future Support)

```cheolsu-query
# Requests within the last hour
time>="1h"

# Specific time range
time>="09:00" and time<="18:00"
```

## 🔧 Troubleshooting

### Common Errors

1. **Syntax Errors**

   ```cheolsu-query
   # ❌ Incorrect
   method=GET

   # ✅ Correct
   method="GET"
   ```

2. **Operator Errors**

   ```cheolsu-query
   # ❌ Incorrect
   method=="GET"

   # ✅ Correct
   method="GET"
   ```

3. **Logical Operator Errors**

   ```cheolsu-query
   # ❌ Incorrect
   method="GET" && status="2xx"

   # ✅ Correct
   method="GET" and status="2xx"
   ```

## 📚 Related Documentation

- [Basic Usage](../guide/basic-usage.md)
- [Troubleshooting](../guide/troubleshooting.md)
- [TLS Support](./tls-support.md)
