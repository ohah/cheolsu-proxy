#!/bin/bash
# MCP 서버 바이너리를 빌드하고 Tauri externalBin 경로에 복사하는 스크립트
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_TRIPLE=$(rustc -vV | grep host | awk '{print $2}')
BINARIES_DIR="$SCRIPT_DIR/src-tauri/binaries"

PROFILE="${1:-debug}"
if [ "$PROFILE" = "release" ]; then
    CARGO_FLAGS="--release"
    PROFILE_DIR="release"
else
    CARGO_FLAGS=""
    PROFILE_DIR="debug"
fi

echo "Building cheolsu-proxy-mcp ($PROFILE)..."
cargo build -p cheolsu-proxy-mcp $CARGO_FLAGS --manifest-path "$WORKSPACE_ROOT/Cargo.toml"

mkdir -p "$BINARIES_DIR"
cp "$WORKSPACE_ROOT/target/$PROFILE_DIR/cheolsu-proxy-mcp" "$BINARIES_DIR/cheolsu-proxy-mcp-$TARGET_TRIPLE"
echo "Copied to $BINARIES_DIR/cheolsu-proxy-mcp-$TARGET_TRIPLE"
