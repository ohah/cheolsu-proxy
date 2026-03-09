#!/bin/bash
# MCP 서버 및 TUI 바이너리를 빌드하고 Tauri externalBin 경로에 복사하는 스크립트
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

echo "Building cheolsu TUI ($PROFILE)..."
cargo build -p cheolsu-proxy-tui $CARGO_FLAGS --manifest-path "$WORKSPACE_ROOT/Cargo.toml"

mkdir -p "$BINARIES_DIR"
cp "$WORKSPACE_ROOT/target/$PROFILE_DIR/cheolsu-proxy-mcp" "$BINARIES_DIR/cheolsu-proxy-mcp-$TARGET_TRIPLE"
echo "Copied to $BINARIES_DIR/cheolsu-proxy-mcp-$TARGET_TRIPLE"

cp "$WORKSPACE_ROOT/target/$PROFILE_DIR/cheolsu" "$BINARIES_DIR/cheolsu-$TARGET_TRIPLE"
echo "Copied to $BINARIES_DIR/cheolsu-$TARGET_TRIPLE"

# macOS: OpenSSL dylib를 번들용으로 준비 (install_name을 @rpath 기반으로 변경)
if [[ "$OSTYPE" == "darwin"* ]]; then
    OPENSSL_LIB_DIR=$(brew --prefix openssl@3)/lib
    FRAMEWORKS_DIR="$SCRIPT_DIR/src-tauri/frameworks"
    mkdir -p "$FRAMEWORKS_DIR"

    echo "Preparing OpenSSL dylibs for bundling..."
    cp "$OPENSSL_LIB_DIR/libssl.3.dylib" "$FRAMEWORKS_DIR/"
    cp "$OPENSSL_LIB_DIR/libcrypto.3.dylib" "$FRAMEWORKS_DIR/"

    # install_name을 @rpath 기반으로 변경
    install_name_tool -id @rpath/libssl.3.dylib "$FRAMEWORKS_DIR/libssl.3.dylib"
    install_name_tool -id @rpath/libcrypto.3.dylib "$FRAMEWORKS_DIR/libcrypto.3.dylib"

    # libssl이 참조하는 libcrypto 경로도 @rpath로 변경
    install_name_tool -change "$OPENSSL_LIB_DIR/libcrypto.3.dylib" @rpath/libcrypto.3.dylib "$FRAMEWORKS_DIR/libssl.3.dylib"
    # Cellar 경로로 참조하는 경우도 처리
    OPENSSL_CELLAR_LIB=$(readlink -f "$OPENSSL_LIB_DIR/libcrypto.3.dylib" | xargs dirname)
    if [ "$OPENSSL_CELLAR_LIB" != "$OPENSSL_LIB_DIR" ]; then
        install_name_tool -change "$OPENSSL_CELLAR_LIB/libcrypto.3.dylib" @rpath/libcrypto.3.dylib "$FRAMEWORKS_DIR/libssl.3.dylib"
    fi

    # 코드 서명 제거 (ad-hoc 서명으로 재서명)
    codesign --force --sign - "$FRAMEWORKS_DIR/libssl.3.dylib"
    codesign --force --sign - "$FRAMEWORKS_DIR/libcrypto.3.dylib"

    echo "OpenSSL dylibs prepared in $FRAMEWORKS_DIR"
fi
