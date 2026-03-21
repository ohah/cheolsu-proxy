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

## Subcommands

### Traffic

```bash
# Search captured traffic
cheolsu traffic search --host example.com --method GET --limit 20

# Get transaction details
cheolsu traffic get <transaction-id>

# View WebSocket messages
cheolsu traffic ws --connection-id ws://example.com --limit 50

# View SSE events
cheolsu traffic sse --limit 50

# Compare two transactions
cheolsu traffic diff <id-a> <id-b>

# Clear all captured traffic
cheolsu traffic clear

# Generate OpenAPI spec from traffic
cheolsu traffic openapi --host api.example.com --title "My API"
```

### Rules

```bash
# List intercept rules
cheolsu rule list

# Add a block rule
cheolsu rule add --name "Block ads" --pattern "*.ads.com" --action-type block

# Add a map-remote rule
cheolsu rule add --name "Dev redirect" --pattern "*api.example.com*" \
  --action-type map_remote --target-url "http://localhost:3000" --preserve-path true

# Add a modify-request rule
cheolsu rule add --name "Add auth" --pattern "*api.example.com*" \
  --action-type modify_request --add-header "Authorization=Bearer token123"

# Remove a rule
cheolsu rule remove <rule-id>
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

```bash
# Configure upstream proxy
cheolsu settings upstream-proxy --enabled true --host proxy.corp.com --port 8080

# Enable throttling
cheolsu settings throttle --enabled true --download-rate 50000 --latency-ms 200

# Configure proxy authentication
cheolsu settings proxy-auth --enabled true --method basic --username user --password pass

# Set connection strategy
cheolsu settings connection-strategy eager

# Quick settings
cheolsu settings quick --no-caching true --block-cookies true

# Configure client certificate (mTLS)
cheolsu settings client-certificate --enabled true --cert-path ./cert.pem --key-path ./key.pem

# Configure SSL proxying list
cheolsu settings ssl-proxying-list --mode whitelist \
  --entry "api.example.com=true" \
  --entry "api.example.com:8443=true"
```

### Session

```bash
# Save current traffic to a session file
cheolsu session save ./traffic.cheolsu --name "My Session"

# Load a session file
cheolsu session load ./traffic.cheolsu

# Load and append to existing traffic
cheolsu session load ./traffic.cheolsu --append
```

### Other

```bash
# Check proxy status
cheolsu status

# Export traffic as HAR file
cheolsu export-har ./output.har --host example.com

# Launch TUI mode
cheolsu tui --port 8100

# Manage scripts
cheolsu script load --path ./my-script.js
cheolsu script unload
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (daemon not running, invalid arguments, operation failed) |

## Claude Code Integration

Cheolsu Proxy provides Claude Code skills for direct use in conversations:

- `/cheolsu <command>` — Execute any CLI command
- `/dev-url-mapping <domain> <local-addr>` — Set up local dev URL mapping
