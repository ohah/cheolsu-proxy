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
    OPENSSL_PREFIX=$(brew --prefix openssl@3)
    if [ ! -d "$OPENSSL_PREFIX/lib" ]; then
        echo "Error: OpenSSL not found at $OPENSSL_PREFIX/lib"
        echo "Install with: brew install openssl@3"
        exit 1
    fi
    OPENSSL_LIB_DIR="$OPENSSL_PREFIX/lib"
    FRAMEWORKS_DIR="$SCRIPT_DIR/src-tauri/frameworks"
    mkdir -p "$FRAMEWORKS_DIR"

    echo "Preparing OpenSSL dylibs for bundling..."
    rm -f "$FRAMEWORKS_DIR/libssl.3.dylib" "$FRAMEWORKS_DIR/libcrypto.3.dylib"
    cp "$OPENSSL_LIB_DIR/libssl.3.dylib" "$FRAMEWORKS_DIR/"
    cp "$OPENSSL_LIB_DIR/libcrypto.3.dylib" "$FRAMEWORKS_DIR/"
    chmod u+w "$FRAMEWORKS_DIR/libssl.3.dylib" "$FRAMEWORKS_DIR/libcrypto.3.dylib"

    # install_name을 @rpath 기반으로 변경
    install_name_tool -id @rpath/libssl.3.dylib "$FRAMEWORKS_DIR/libssl.3.dylib"
    install_name_tool -id @rpath/libcrypto.3.dylib "$FRAMEWORKS_DIR/libcrypto.3.dylib"

    # libssl이 참조하는 libcrypto의 모든 절대 경로를 @rpath로 변경
    # otool -L로 실제 참조 경로를 추출하여 처리 (readlink -f는 macOS 기본 readlink에서 미지원)
    otool -L "$FRAMEWORKS_DIR/libssl.3.dylib" | grep libcrypto | awk '{print $1}' | while read -r crypto_path; do
        if [[ "$crypto_path" != "@rpath/"* ]]; then
            install_name_tool -change "$crypto_path" @rpath/libcrypto.3.dylib "$FRAMEWORKS_DIR/libssl.3.dylib"
        fi
    done

    # ad-hoc 서명으로 재서명 (install_name_tool이 기존 서명을 무효화하므로)
    codesign --force --sign - "$FRAMEWORKS_DIR/libssl.3.dylib"
    codesign --force --sign - "$FRAMEWORKS_DIR/libcrypto.3.dylib"

    # sidecar 바이너리에도 OpenSSL 참조 경로를 @rpath로 변경하고 rpath 추가
    for bin in "$BINARIES_DIR/cheolsu-proxy-mcp-$TARGET_TRIPLE" "$BINARIES_DIR/cheolsu-$TARGET_TRIPLE"; do
        if [ -f "$bin" ]; then
            echo "Patching sidecar binary: $(basename "$bin")"
            # OpenSSL 참조를 @rpath 기반으로 변경
            otool -L "$bin" | grep libssl | awk '{print $1}' | while read -r ssl_path; do
                if [[ "$ssl_path" != "@rpath/"* ]]; then
                    install_name_tool -change "$ssl_path" @rpath/libssl.3.dylib "$bin"
                fi
            done
            otool -L "$bin" | grep libcrypto | awk '{print $1}' | while read -r crypto_path; do
                if [[ "$crypto_path" != "@rpath/"* ]]; then
                    install_name_tool -change "$crypto_path" @rpath/libcrypto.3.dylib "$bin"
                fi
            done
            # @executable_path/../Frameworks rpath 추가 (이미 있으면 무시)
            install_name_tool -add_rpath @executable_path/../Frameworks "$bin" 2>/dev/null || true
            codesign --force --sign - "$bin"
        fi
    done

    echo "OpenSSL dylibs prepared in $FRAMEWORKS_DIR"
fi
