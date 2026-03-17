# Network Throttling

## Overview

Network Throttling artificially limits bandwidth and adds latency to traffic passing through the proxy. This simulates slow network conditions, allowing you to test how your application behaves under various network environments.

Reproduce real-world conditions like mobile networks, overseas server connections, or bandwidth-constrained environments to verify loading states, timeout handling, progressive loading, and offline fallback behaviors.

---

## Presets

Common network profiles are available as presets:

| Preset | Download | Upload | Latency | Description |
| --- | --- | --- | --- | --- |
| **GPRS** | 50 KB/s | 20 KB/s | 500ms | 2G mobile network |
| **Slow 3G** | 500 KB/s | 500 KB/s | 400ms | Slow 3G connection |
| **Fast 3G** | 1.6 MB/s | 768 KB/s | 150ms | Fast 3G connection |
| **LTE** | 4 MB/s | 3 MB/s | 50ms | 4G LTE network |
| **WiFi** | 30 MB/s | 15 MB/s | 2ms | Standard WiFi |

---

## Configuration

| Setting | Description |
| --- | --- |
| **Enabled** | Toggle throttling on/off |
| **Download Rate** | Download bandwidth limit (bytes/sec) |
| **Upload Rate** | Upload bandwidth limit (bytes/sec) |
| **Latency** | Artificial delay added to all requests (ms) |

Select a preset to auto-fill values, or manually adjust individual settings.

---

## How It Works

Throttling uses the Token Bucket algorithm to control bandwidth in both directions (download/upload). It applies to all TCP streams passing through the proxy, naturally delaying data transfers that exceed the configured rate.

---

## Use Cases

### Mobile Environment Testing

Apply GPRS or Slow 3G presets to verify UX on slow networks — check image lazy loading, skeleton UIs, and progressive rendering.

### Timeout Verification

Set high latency to test that API call timeout handling works correctly.

### Large File Transfer Testing

Limit download speed to verify download progress bars, retry logic, and partial download behavior.

---

## Usage

### Desktop

Configure throttling presets or custom values in the Settings page.

### MCP

```
"Set network speed to Slow 3G"
"Disable throttling"
```
