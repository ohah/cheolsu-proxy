# CLI

Cheolsu Proxy provides a command-line interface (CLI) that offers the same functionality as the MCP server. It's useful for scripting, automation, and environments where MCP is not available.

## Installation

```bash
# Build from source
cargo build -p cheolsu-cli --release

# The binary is at target/release/cheolsu
```

## Usage

```bash
cheolsu <subcommand> [options]
```

Every command accepts the global `--output` option to choose the output format (`text` default, `json`).

```bash
cheolsu traffic search --host example.com --output json
```

## Subcommands

### Traffic

```bash
# Search captured traffic (host/method/status/path/limit filters)
cheolsu traffic search --host example.com --method GET --status 500 --path /api --limit 20

# Get transaction details
cheolsu traffic get <transaction-id>

# View WebSocket messages
cheolsu traffic ws --connection-id ws://example.com --limit 50

# View SSE events
cheolsu traffic sse --connection-id https://example.com --limit 50

# Compare two transactions
cheolsu traffic diff <id-a> <id-b>

# Clear all captured traffic
cheolsu traffic clear

# Generate OpenAPI spec from traffic
cheolsu traffic openapi --host api.example.com --path-prefix /v1 --title "My API"
```

### Rules

The required options depend on `--action-type`.

```bash
# List intercept rules
cheolsu rule list

# Block rule (optional custom status/body)
cheolsu rule add --name "Block ads" --pattern "*.ads.com" --action-type block \
  --status-code 403 --response-body "blocked"

# Modify-request rule (add/remove headers)
cheolsu rule add --name "Add auth" --pattern "*api.example.com*" \
  --action-type modify_request --add-header "Authorization=Bearer token123" \
  --remove-header "Cookie"

# Modify-response rule
cheolsu rule add --name "Allow CORS" --pattern "*api.example.com*" \
  --action-type modify_response --add-header "Access-Control-Allow-Origin=*"

# Map Local — serve the response from a local file
cheolsu rule add --name "Mock response" --pattern "*api.example.com/users*" \
  --action-type map_local --file-path ./mock/users.json

# Map Remote
cheolsu rule add --name "Dev redirect" --pattern "*api.example.com*" \
  --action-type map_remote --target-url "http://localhost:3000" --preserve-path true

# Remove a rule
cheolsu rule remove <rule-id>
```

### Breakpoints

```bash
# List breakpoint rules
cheolsu breakpoint list

# Add a breakpoint (choose request/response phase)
cheolsu breakpoint add --pattern "*api.example.com*" \
  --break-on-request true --break-on-response false

# List paused (pending) breakpoints
cheolsu breakpoint pending

# Resolve a pending breakpoint (forward / modify_and_forward / drop / abort)
cheolsu breakpoint resolve --id <bp-id> --action modify_and_forward \
  --header "X-Debug=1" --body '{"patched":true}' --status 200

# Remove a breakpoint
cheolsu breakpoint remove <bp-id>
```

### Host Mapping

```bash
# List host mappings
cheolsu host-mapping list

# Add a host mapping (source → target, optional ports)
cheolsu host-mapping add --source-host api.example.com \
  --target-host 127.0.0.1 --target-port 3000

# Remove a host mapping
cheolsu host-mapping remove <mapping-id>
```

### Reverse Proxy

```bash
# List reverse proxy rules
cheolsu reverse-proxy list

# Add a rule (Host pattern → backend)
cheolsu reverse-proxy add --match-host example.com \
  --backend-scheme http --backend-host 127.0.0.1 --backend-port 8080 \
  --rewrite-host true

# Remove a rule
cheolsu reverse-proxy remove <rule-id>
```

### Server Replay

```bash
# List server replay entries
cheolsu server-replay list

# Register a captured transaction for server replay (cached response for matches)
cheolsu server-replay add <transaction-id>

# Remove an entry / clear all
cheolsu server-replay remove <entry-id>
cheolsu server-replay clear
```

### Analysis

```bash
# Slow request analysis
cheolsu analyze performance --threshold-ms 500 --limit 10

# Error analysis
cheolsu analyze errors

# Endpoint statistics
cheolsu analyze endpoints --sort-by duration --limit 20

# Duplicate request detection
cheolsu analyze duplicates --window-ms 5000

# N+1 query pattern detection
cheolsu analyze n-plus1

# Traffic timeline
cheolsu analyze timeline --bucket-seconds 30

# Full analysis (all of the above)
cheolsu analyze full
```

### Replay

```bash
# Send an HTTP request directly (no daemon required)
cheolsu replay request --method GET --url https://httpbin.org/get

# Send with headers and body
cheolsu replay request --method POST --url https://httpbin.org/post \
  --header "Content-Type=application/json" --body '{"key":"value"}'

# Replay captured transactions in sequence
cheolsu replay sequence <txn-id-1> <txn-id-2> --delay-ms 100

# Load test with concurrent requests
cheolsu replay repeat --method GET --url https://example.com \
  --iterations 100 --concurrency 10
```

### Settings

> `--enabled`, `--no-caching`, etc. are **boolean flags that take no value**. Add the flag to turn it on, omit it to turn it off (e.g. `--enabled true` is an error).

```bash
# Enable upstream proxy (auth optional)
cheolsu settings upstream-proxy --enabled --host proxy.corp.com --port 8080 \
  --username user --password pass

# Disable upstream proxy (omit the flag)
cheolsu settings upstream-proxy --host proxy.corp.com --port 8080

# Configure throttling (rates in bytes/sec)
cheolsu settings throttle --enabled --download-rate 50000 --upload-rate 20000 --latency-ms 200

# Configure proxy authentication (basic / bearer / apikey)
cheolsu settings proxy-auth --enabled --method basic --username user --password pass
cheolsu settings proxy-auth --enabled --method bearer --token "my-token"

# Set connection strategy (lazy / eager / eager_with_fallback)
cheolsu settings connection-strategy eager

# Quick settings (turn on only the flags you need)
cheolsu settings quick --no-caching --block-cookies --no-gzip --block-quic

# Configure client certificate (mTLS)
cheolsu settings client-certificate --enabled --cert-path ./cert.pem --key-path ./key.pem

# Configure SSL proxying list (mode: blacklist default / whitelist)
cheolsu settings ssl-proxying-list --mode whitelist \
  --entry "api.example.com=true" \
  --entry "api.example.com:8443=true"
```

### Session

```bash
# Save current traffic to a session file (.cheolsu.gz to gzip, --filter for URL substring filter)
cheolsu session save ./traffic.cheolsu --name "My Session" \
  --description "Repro case" --filter example.com

# Load a session file (.cheolsu/.cheolsu.gz) or import a HAR (.har)
cheolsu session load ./traffic.cheolsu

# Load and append to existing traffic
cheolsu session load ./traffic.cheolsu --append
```

### Scripts

```bash
# Load a script from a file
cheolsu script load --path ./my-script.js

# Load inline code
cheolsu script load --code "cheolsu.onRequest((req) => ({ action: 'forward' }));"

# Unload the script
cheolsu script unload
```

### Other

```bash
# Check proxy status
cheolsu status

# Export traffic as HAR file (--host / --path filters)
cheolsu export-har ./output.har --host example.com

# Launch TUI mode
cheolsu tui --port 8100 --host 0.0.0.0

# Install / check / uninstall Claude Code skills
cheolsu install-skills
cheolsu install-skills --check
cheolsu uninstall-skills

# Generate shell completion (bash / zsh / fish / powershell)
cheolsu completion zsh > _cheolsu
```

## Exit Codes

| Code | Meaning                                                         |
| ---- | --------------------------------------------------------------- |
| 0    | Success                                                         |
| 1    | Error (daemon not running, invalid arguments, operation failed) |

## Claude Code Integration

Cheolsu Proxy provides Claude Code skills for direct use in conversations:

- `/cheolsu <command>` — Execute any CLI command
- `/dev-url-mapping <domain> <local-addr>` — Set up local dev URL mapping
