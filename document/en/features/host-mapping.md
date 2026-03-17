# Host Mapping

## Overview

Host Mapping redirects requests for specific domains to different hosts. Similar to editing the `/etc/hosts` file, but operates dynamically at the proxy level without modifying system settings.

Useful for staging server testing, local development server connections, and server migration verification — all without touching the system hosts file.

---

## Configuration

| Setting         | Description                                                             |
| --------------- | ----------------------------------------------------------------------- |
| **Source Host** | Domain pattern to match (wildcard supported, e.g., `*.api.example.com`) |
| **Source Port** | Port to match (unset = any port)                                        |
| **Target Host** | IP address or domain to redirect to                                     |
| **Target Port** | Port to redirect to (unset = keep original)                             |
| **Enabled**     | Toggle rule on/off                                                      |

---

## Wildcard Patterns

Use wildcard patterns in source hosts to map multiple subdomains at once.

| Pattern             | Match Examples                                   |
| ------------------- | ------------------------------------------------ |
| `api.example.com`   | Exact domain match                               |
| `*.example.com`     | `api.example.com`, `cdn.example.com`, etc.       |
| `*.api.example.com` | `v1.api.example.com`, `v2.api.example.com`, etc. |

---

## Use Cases

### Staging Server Testing

```
Source: api.example.com
Target: staging.internal.example.com
```

Route production domain requests to a staging server, testing the staging environment without frontend code changes.

### Local Development

```
Source: api.example.com
Target: 127.0.0.1
Target Port: 3000
```

Route API requests to a local development server while accessing the frontend via the production URL.

### Server Migration Verification

```
Source: *.old-service.com
Target: new-service.com
```

Map old service domains to new ones to verify behavior before migration.

---

## Usage

### Desktop

1. Select **Host Mapping** from the sidebar
2. Click **Add Rule**
3. Configure source host, target host, and ports
4. Save

### MCP

```
"Map api.example.com to localhost:3000"
"Show me the host mapping rules"
```
