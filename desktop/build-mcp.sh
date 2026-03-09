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

# macOS: OpenSSL dylib를 번들용으로 준비 (cargo build 전에 실행해야 함)
# openssl-sys가 OPENSSL_DIR의 dylib를 링크하면 @rpath install_name이 바이너리에 기록됨
if [[ "$OSTYPE" == "darwin"* ]]; then
    OPENSSL_PREFIX=$(brew --prefix openssl@3)
    if [ ! -d "$OPENSSL_PREFIX/lib" ]; then
        echo "Error: OpenSSL not found at $OPENSSL_PREFIX/lib"
        echo "Install with: brew install openssl@3"
        exit 1
    fi

    OPENSSL_BUNDLE_DIR="$SCRIPT_DIR/src-tauri/openssl-bundle"
    mkdir -p "$OPENSSL_BUNDLE_DIR/lib"

    echo "Preparing OpenSSL bundle for linking..."
    rm -f "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib" "$OPENSSL_BUNDLE_DIR/lib/libcrypto.3.dylib"
    cp "$OPENSSL_PREFIX/lib/libssl.3.dylib" "$OPENSSL_BUNDLE_DIR/lib/"
    cp "$OPENSSL_PREFIX/lib/libcrypto.3.dylib" "$OPENSSL_BUNDLE_DIR/lib/"
    # openssl-sys가 .dylib 심볼릭 링크를 요구하는 경우 대비
    ln -sf libssl.3.dylib "$OPENSSL_BUNDLE_DIR/lib/libssl.dylib"
    ln -sf libcrypto.3.dylib "$OPENSSL_BUNDLE_DIR/lib/libcrypto.dylib"
    chmod u+w "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib" "$OPENSSL_BUNDLE_DIR/lib/libcrypto.3.dylib"

    # install_name을 @rpath 기반으로 변경
    install_name_tool -id @rpath/libssl.3.dylib "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib"
    install_name_tool -id @rpath/libcrypto.3.dylib "$OPENSSL_BUNDLE_DIR/lib/libcrypto.3.dylib"

    # libssl → libcrypto 참조도 @rpath로 변경
    otool -L "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib" | grep libcrypto | awk '{print $1}' | while read -r crypto_path; do
        if [[ "$crypto_path" != "@rpath/"* ]]; then
            install_name_tool -change "$crypto_path" @rpath/libcrypto.3.dylib "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib"
        fi
    done

    # ad-hoc 서명으로 재서명
    codesign --force --sign - "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib"
    codesign --force --sign - "$OPENSSL_BUNDLE_DIR/lib/libcrypto.3.dylib"

    # 헤더 파일 심볼릭 링크 (openssl-sys 빌드에 필요)
    ln -sfn "$OPENSSL_PREFIX/include" "$OPENSSL_BUNDLE_DIR/include"

    # OPENSSL_DIR 설정 — openssl-sys가 이 경로의 dylib를 링크하게 함
    export OPENSSL_DIR="$OPENSSL_BUNDLE_DIR"
    echo "OPENSSL_DIR=$OPENSSL_DIR"

    echo "OpenSSL bundle prepared in $OPENSSL_BUNDLE_DIR"
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

# macOS: sidecar 바이너리에 @executable_path/../Frameworks rpath 추가
if [[ "$OSTYPE" == "darwin"* ]]; then
    for bin in "$BINARIES_DIR/cheolsu-proxy-mcp-$TARGET_TRIPLE" "$BINARIES_DIR/cheolsu-$TARGET_TRIPLE"; do
        if [ -f "$bin" ]; then
            echo "Adding rpath to sidecar: $(basename "$bin")"
            install_name_tool -add_rpath @executable_path/../Frameworks "$bin" 2>/dev/null || true
            codesign --force --sign - "$bin"
        fi
    done
fi
