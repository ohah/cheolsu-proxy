---
title: Testing
description: Cheolsu Proxy test writing and execution guide
---

# Testing

This guide explains Cheolsu Proxy's testing strategy and execution methods.

## Testing Strategy

### Test Pyramid

```
        E2E Tests (Few)
       /              \
   Integration Tests (Medium)
  /                      \
Unit Tests (Many)
```

- **Unit Tests**: Individual function/method testing
- **Integration Tests**: Inter-module interaction testing
- **E2E Tests**: Full application testing

## Rust Tests

The simplest way is the root script, which sets the OpenSSL dylib path automatically.

```bash
# Run the whole workspace (DYLD_LIBRARY_PATH set automatically)
bun run cargo:test

# A single crate
bun run cargo:test -- -p proxy_daemon
```

CI runs it in two steps (`proxyapi_v2` needs the `full` feature).

```bash
cargo test --workspace --exclude proxyapi_v2
cargo test -p proxyapi_v2 --features full
```

> On macOS, if `libssl.3.dylib` can't be found when running directly, add the bundle path (`desktop/src-tauri/openssl-bundle/lib`) to `DYLD_LIBRARY_PATH`, or set `OPENSSL_DIR=/opt/homebrew/opt/openssl@3`.

### Rust Formatting

```bash
# Apply formatting
bun run cargo:fmt
# Check formatting (same as CI)
cargo fmt --all -- --check
```

## Frontend Tests

Tests run with `bun test` (from the `desktop/` directory).

```bash
cd desktop

bun run test            # store/entity/lib unit tests
bun run test:components # features/widgets/pages component tests
bun run test:e2e        # E2E tests
bun run test:all        # run all of the above
```

### JavaScript/TypeScript Lint & Format

Run from the repository root (oxlint / oxfmt).

```bash
bun run lint          # oxlint
bun run format:check  # oxfmt --check
bun run format        # oxfmt (apply)
```

## i18n Catalog Sync

If you changed GUI code (strings), regenerate the catalog and commit it. CI verifies the catalog is up to date.

```bash
cd desktop && bun run extract
```

## CI Checks

A PR must pass the following:

- **Rust Format Check** — `cargo fmt --all -- --check`
- **Rust Unit Test** — workspace + `proxyapi_v2 --features full`
- **JavaScript/TypeScript Lint Check** — `format:check`, `lint`, i18n catalog sync
- **Frontend Unit Test** — `bun test`
