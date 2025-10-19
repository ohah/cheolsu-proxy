---
title: Development Setup
description: Complete environment setup guide for Cheolsu Proxy development
---

# Development Setup

Complete environment setup guide for Cheolsu Proxy development.

## Technology Stack

This project uses the following tools and technologies:

- **Rust**: Core backend language with Cargo package manager
- **Tauri**: Desktop application framework
- **pnpm**: Fast, disk space efficient package manager for Node.js
- **oxc**: JavaScript/TypeScript toolchain for parsing and linting
- **Rspress**: Documentation site generator

## System Requirements

### Operating System

- **macOS**: 10.15 (Catalina) or higher
- **Windows**: Windows 10 or higher

### Hardware

- **RAM**: Minimum 8GB (16GB recommended)
- **Storage**: At least 10GB free space
- **CPU**: 64-bit processor

## Essential Software Installation

### 1. Rust Installation

#### macOS

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Load environment variables
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

#### Windows

1. Download `rustup-init.exe` from [rustup.rs](https://rustup.rs/)
2. Run and follow the installation wizard
3. Verify in PowerShell:
   ```powershell
   rustc --version
   cargo --version
   ```

### 2. Node.js Installation

#### macOS (Homebrew)

```bash
# Install with Homebrew
brew install node

# Or use nvm
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install --lts
nvm use --lts
```

#### Windows

1. Download LTS version from [nodejs.org](https://nodejs.org/)
2. Follow the installation wizard
3. Verify in PowerShell:
   ```powershell
   node --version
   pnpm --version
   ```

### 3. Tauri CLI Installation

```bash
# Install Tauri CLI
cargo install tauri-cli

# Verify installation
tauri --version
```

### 4. Development Tools Installation

#### Rust Tools

```bash
# Code formatting
rustup component add rustfmt

# Linter
rustup component add clippy

# Documentation generation
rustup component add rust-docs

# Source code
rustup component add rust-src
```

#### Additional Rust Tools

```bash
# Code analysis
cargo install cargo-audit
cargo install cargo-outdated

# Performance analysis
cargo install flamegraph

# Test coverage
cargo install cargo-tarpaulin
```

## Project Setup

### 1. Repository Clone

```bash
# Clone repository
git clone https://github.com/ohah/cheolsu-proxy.git
cd cheolsu-proxy

# Verify remote repository settings
git remote -v
```

### 2. Dependency Installation

#### Rust Dependencies

```bash
# From root directory
cargo build

# Build all packages
cargo build --workspace
```

#### Node.js Dependencies

```bash
# Navigate to Tauri UI directory
cd tauri-ui

# Install dependencies
pnpm install

```

### 3. Development Server

```bash
# Run Tauri development server
cd tauri-ui
pnpm tauri dev
```

## IDE Setup

### VS Code (Recommended)

#### Essential Extensions

```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "vadimcn.vscode-lldb",
    "ms-vscode.vscode-typescript-next",
    "bradlc.vscode-tailwindcss",
    "esbenp.prettier-vscode",
    "ms-vscode.vscode-json"
  ]
}
```

#### Settings File (`.vscode/settings.json`)

```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "rust-lang.rust-analyzer",
  "[typescript]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  },
  "[typescriptreact]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  }
}
```

### IntelliJ IDEA / CLion

#### Rust Plugin

1. **File** > **Settings** > **Plugins**
2. Search for "Rust" and install
3. Set Rust toolchain path

#### Settings

- **Rust toolchain**: `~/.cargo/bin/rustc`
- **Cargo path**: `~/.cargo/bin/cargo`

## Build and Test

### 1. Build

```bash
# Development build
cargo build

# Release build
cargo build --release

# Build specific package only
cargo build -p proxyapi_v2

# Build with all features
cargo build --features "native-tls-client,rcgen-ca,openssl-ca"
```

### 2. Test

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Integration tests
cargo test --test integration_test

# Benchmarks
cargo bench
```

### 3. Code Quality Checks

```bash
# Code formatting
cargo fmt

# Run linter
cargo clippy

# Security check
cargo audit

# Check outdated dependencies
cargo outdated
```

## Tauri UI Development

### 1. Development Server

```bash
# Tauri UI development server
cd tauri-ui
pnpm run dev

# Run with Tauri app
pnpm run tauri dev
```

### 2. Build

```bash
# Web build
pnpm run build

# Tauri app build
pnpm run tauri build
```

### 3. Test

To be added.

## Debugging

#### Tauri Debugging

```bash
# Run in debug mode
cd tauri-ui
RUST_LOG=debug pnpm run tauri dev
```

Open Developer Tools

Cmd + shift + I

#### Tauri Build Errors

```bash
# Clear cache
cargo clean
# Reinstall Tauri CLI
cargo install tauri-cli --force

# Check system dependencies
# macOS: Xcode Command Line Tools
# Windows: Visual Studio Build Tools
```

### Platform-Specific Issues

#### macOS

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Update Homebrew
brew update && brew upgrade
```

#### Windows

```bash
# Install Visual Studio Build Tools
# https://visualstudio.microsoft.com/visual-cpp-build-tools/

# Verify Windows SDK installation
```

## Next Steps

After setting up your development environment, refer to the following documents:

- [Code Structure](/en/contributing/code-structure) - Understanding the codebase structure
- [Testing](/en/contributing/testing) - Writing and running tests
- [Contributing Guide](/en/contributing/) - Contribution process
