---
title: Cheolsu Proxy
description: A simple Man In The Middle proxy built with Rust and Tauri
---

<div align="center">
<img style="width:100px; margin:auto" src="/assets/logo.png">
<h1> Cheolsu Proxy </h1>
<h2> A simple <i>Man In The Middle</i> proxy</h2>
</div>

[![build](https://github.com/ohah/cheolsu-proxy/actions/workflows/autofix.yml/badge.svg?branch=master)](https://github.com/ohah/cheolsu-proxy/actions/workflows/autofix.yml)
![GitHub](https://img.shields.io/github/license/ohah/cheolsu-proxy/)
![GitHub last commit](https://img.shields.io/github/last-commit/ohah/cheolsu-proxy/)
![GitHub top language](https://img.shields.io/github/languages/top/ohah/cheolsu-proxy/)

## Introduction

Rust-based **Man in the Middle proxy**, an early-stage project aimed at providing visibility into network traffic. Currently, it displays both HTTP and HTTPS requests and responses, but our future goal is to allow for manipulation of the traffic for more advanced use cases.

![Cast](/assets/screenshots/0.gif)

## Key Features

- 🔐 HTTP / HTTPS support
- 🔒 TLS 1.0/1.1 legacy client support (hybrid TLS handler)
- 🖱️ GUI interface
- ⌨️ Possibility of choosing a customised address and listening port
- 🔍 Details for each request and response
- 🎯 Filtering the list of requests by method
- ❌ Deleting a single request from the list
- 🚫 Clear all requests and clean the table
- 🌌 Dark / light theme
- 🔐 **Security**: Automatic generation of unique CA certificates per user (private keys are not included in the binary)

## Quick Start

### 1. Automatic Certificate Generation and Installation

Cheolsu Proxy automatically generates a unique CA certificate on first run.

**Manual certificate installation on macOS:**

1. When you run the app, the certificate file path will be displayed in the console:

   ```
   📁 Path: ~/Library/Application Support/com.cheolsu-proxy/cheolsu-proxy.cer
   ```

2. Launch Keychain Access app
3. Select the 'login' keychain
4. Choose File > Import Items... menu
5. Select the `cheolsu-proxy.cer` file from the above path
6. Double-click the certificate and set it to 'Always Trust'

### 2. System Proxy Settings

Set your local system proxy to `127.0.0.1:8100`.

For detailed setup instructions, see the [Certificate Setup Guide](/en/guide/certificate-setup).

## Screenshots

### Input of Listening Address

![Mitm proxy Screenshot 1](/assets/screenshots/1b.png)
![Mitm proxy Screenshot 1](/assets/screenshots/1w.png)
![Mitm proxy Screenshot 1](/assets/screenshots/2w.png)

### Requests List

![Mitm proxy Screenshot 2](/assets/screenshots/3w.png)
![Mitm proxy Screenshot 2](/assets/screenshots/3b.png)
![Mitm proxy Screenshot 2](/assets/screenshots/4b.png)

### Request and Response Details

![Mitm proxy Screenshot 3](/assets/screenshots/5b.png)
![Mitm proxy Screenshot 3](/assets/screenshots/5w.png)

## License

See [LICENSE-APACHE](https://github.com/ohah/cheolsu-proxy/blob/master/LICENSE-APACHE), [LICENSE-MIT](https://github.com/ohah/cheolsu-proxy/blob/master/LICENSE-MIT) for details.

## Contributing

Contributions are always welcome! See the [Contributing Guide](/en/contributing/) for details.

## Documentation and Help

If you have questions on how to use [cheolsu-proxy](https://github.com/ohah/cheolsu-proxy), please use GitHub Discussions!
![GitHub Discussions](https://img.shields.io/github/discussions/ohah/cheolsu-proxy)
