# Intercept Rules

Intercept and modify HTTP requests/responses using wildcard pattern matching.

---

## Action Types

### 1. Block

Block requests and return a specified status code and response body.

- **Status Code**: HTTP status code (default 403)
- **Response Body**: Custom response body

### 2. Modify Request

Modify requests before they are forwarded to the server.

- **Add Headers**: Add new request headers
- **Remove Headers**: Remove existing request headers
- **Set Body**: Replace request body

### 3. Modify Response

Modify server responses before they are delivered to the client.

- **Set Status**: Change HTTP status code
- **Add/Remove Headers**: Manipulate response headers
- **Set Body**: Replace response body

### 4. Map Local

Return the contents of a local file as the response.

- **File Path**: Path to a local file
- **Status Code**: Custom HTTP status code
- **Headers**: Custom response headers

### 5. Map Remote

Redirect requests to a different URL.

- **Target URL**: Destination URL
- **Preserve Path**: Whether to keep the original request path

---

## Rule Configuration

### Pattern Matching

Use wildcard patterns to match URLs.

```
*ads.example.com*       # URLs containing ads.example.com
*api.example.com/v1/*   # Specific API paths
*.json                  # JSON file requests
```

### HTTP Method Filter

Optionally specify an HTTP method to match only specific methods.

- GET, POST, PUT, DELETE, PATCH, etc.
- If not specified, applies to all methods

### Enable/Disable

Each rule can be individually enabled or disabled.

---

## Usage

### Desktop

1. Select **Intercept Rules** from the sidebar
2. Click **Add Rule** button
3. Configure pattern, action type, and options
4. Save

You can also quickly create rules by selecting transactions from the traffic table.

### TUI

1. Navigate to the **Rules** tab
2. Add/edit/delete rules
3. Enter pattern and action type via form

### MCP

Manage rules through AI assistants.

```
"Add a rule to block example.com domain"
"Create a rule to add CORS headers to API responses"
```
