---
title: Getting Started
description: Cheolsu Proxy installation and basic usage
---

# Getting Started

Learn how to use Cheolsu Proxy to monitor and analyze network traffic.

## System Requirements

- **macOS**: 10.15 (Catalina) or higher
- **Windows**: Windows 10 or higher (coming soon)

## Installation

### 1. Download Binary

Download the latest version from GitHub Releases:

```bash
# macOS
curl -L https://github.com/ohah/cheolsu-proxy/releases/latest/download/cheolsu-proxy-macos.tar.gz | tar -xz
```

### 2. Run

```bash
./cheolsu-proxy
```

## First Run Setup

### 1. Automatic Certificate Generation

Cheolsu Proxy automatically generates a unique CA certificate on first run. You'll see a message like this in the console:

```
🔐 CA certificate has been generated.
📁 Path: ~/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer
```

### 2. Certificate Installation

To monitor HTTPS traffic, you need to install the CA certificate in your system.

**Installation on macOS:**

1. Launch Keychain Access app
2. Select the 'login' keychain
3. Choose File > Import Items... menu
4. Select the `~/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer` file
5. Double-click the certificate and set it to 'Always Trust'

### 3. System Proxy Settings

Set your system proxy to `127.0.0.1:8100`.

**Settings on macOS:**

1. System Preferences > Network
2. Advanced > Proxies tab
3. Check "Web Proxy (HTTP)" and "Secure Web Proxy (HTTPS)"
4. Server: `127.0.0.1`, Port: `8100`

## Basic Usage

### 1. Start Proxy

1. Launch Cheolsu Proxy
2. The proxy will start on `127.0.0.1:8100` with default settings
3. You can change the address or port if needed

### 2. Monitor Traffic

1. Start making network requests from your web browser or other applications
2. View the real-time request list on Cheolsu Proxy's main screen
3. Click on each request to see detailed information

### 3. Filter Requests

- Filter by method: GET, POST, PUT, DELETE, etc.
- Filter by status code: 200, 404, 500, etc.
- Search by URL pattern

### 4. Manage Requests

- **Delete**: Remove individual requests from the list
- **Clear**: Delete all requests at once
- **Export**: Save request data to file (coming soon)

## Troubleshooting

### Certificate Issues

**"Certificate cannot be trusted" error:**

1. Verify the certificate is properly installed
2. Check trust settings in Keychain Access
3. Verify browser certificate store

**Cannot access HTTPS sites:**

1. Verify system proxy settings are correct
2. Check if firewall is blocking port 8100
3. Ensure no conflicts with other proxy software

### Connection Issues

**Cannot connect to proxy:**

1. Verify Cheolsu Proxy is running
2. Check if port 8100 is not in use
3. Verify firewall settings

## Next Steps

- [Key Features](/en/guide/features) - Explore all Cheolsu Proxy features
- [Certificate Setup](/en/guide/certificate-setup) - Detailed certificate setup guide
- [Contributing](/en/contributing/) - How to contribute to the project
