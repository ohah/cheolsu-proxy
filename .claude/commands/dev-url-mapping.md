# Local Development URL Mapping with Cheolsu Proxy

Redirect production domain traffic to a local dev server so you can test changes in browsers and app WebViews without deploying.

## Usage

`/dev-url-mapping <production-domain> <local-server-address>`

Example: `/dev-url-mapping app.example.com localhost:3000`

## What This Does

Sets up 3 proxy rules automatically:
1. **MapRemote** — Redirects `https://app.example.com/*` → `http://localhost:3000/*`
2. **SSL Proxying** — Enables HTTPS interception for the domain
3. **Header Rewrite** — Rewrites Origin/Host headers so HMR works without dev server config changes

## Execution Steps

### Step 1: Add MapRemote Rule

Redirect production domain requests to the local dev server.

```bash
cheolsu rule add \
  --name "<domain> to localhost" \
  --pattern "*<domain>*" \
  --action-type map_remote \
  --target-url "http://<local-address>" \
  --preserve-path true
```

### Step 2: Configure SSL Proxying Whitelist

HTTPS interception is required for MapRemote to work.
Include the dev server port for HMR WebSocket connections.

```bash
cheolsu settings ssl-proxying-list \
  --mode whitelist \
  --entry "<domain>=true" \
  --entry "<domain>:<port>=true"
```

### Step 3: Rewrite Origin/Host Headers (for HMR)

Prevents dev server from rejecting proxied requests due to mismatched Host header.
Works with all bundlers (webpack, rspack, Vite, Next.js) without any dev server config changes.

```bash
cheolsu rule add \
  --name "rewrite origin for HMR" \
  --pattern "*<domain>*" \
  --action-type modify_request \
  --add-header "Origin=http://<local-address>" \
  --add-header "Host=<local-address>"
```

### Step 4: Verify Setup

```bash
cheolsu rule list
cheolsu status
```

## Verification

```bash
# Verify via curl (should return local dev server content)
curl -x http://127.0.0.1:8100 https://<domain>/ -k -s | head -5

# Check certificate issuer (should be "Cheolsu Proxy Root CA")
curl -x http://127.0.0.1:8100 https://<domain>/ -v -s -o /dev/null 2>&1 | grep issuer
```

## Troubleshooting

### SSL interception not working (original certificate returned)

**Cause**: Domain is recorded in `tls_passthrough.json` as a failed TLS handshake and auto-bypassed.

```bash
# Check (macOS)
grep "<domain>" "$HOME/Library/Application Support/com.cheolsu-proxy/tls_passthrough.json"

# Fix: Remove the entry and restart Cheolsu Proxy
```

### HMR (Hot Module Replacement) not working

1. Verify Step 3 (Origin/Host header rewrite) is configured
2. Verify SSL Proxying whitelist includes the dev server port entry

**Alternative: Configure dev server directly**

Instead of proxy header rewrite, you can allow the host in dev server config:
```js
// rspack / webpack
devServer: { allowedHosts: 'all' }

// Vite 6+
server: { allowedHosts: true }

// Next.js 15.2.3+
module.exports: { allowedDevOrigins: ['<domain>'] }
```

### iOS Simulator SSL error (-1200)

Install CA certificate:
```bash
xcrun simctl keychain booted add-root-cert \
  "$HOME/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer"
```

## Cleanup (after development)

```bash
cheolsu rule list          # Check rule IDs
cheolsu rule remove <id>   # Remove MapRemote rule
cheolsu rule remove <id>   # Remove HMR header rule
```
