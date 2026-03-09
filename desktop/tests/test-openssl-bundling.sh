#!/bin/bash
# OpenSSL dylib 번들링 결과 검증 테스트
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DESKTOP_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
OPENSSL_BUNDLE_DIR="$DESKTOP_DIR/src-tauri/openssl-bundle"
BINARIES_DIR="$DESKTOP_DIR/src-tauri/binaries"
TARGET_TRIPLE=$(rustc -vV | grep host | awk '{print $2}')

PASS=0
FAIL=0

assert() {
    local description="$1"
    local result="$2"
    if [ "$result" = "0" ]; then
        echo "  PASS: $description"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $description"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== OpenSSL dylib 번들링 테스트 ==="
echo ""

# 1. openssl-bundle 디렉토리 구조 확인
echo "[1] openssl-bundle 디렉토리 구조 확인"
test -f "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib"
assert "libssl.3.dylib 존재" $?
test -f "$OPENSSL_BUNDLE_DIR/lib/libcrypto.3.dylib"
assert "libcrypto.3.dylib 존재" $?
test -d "$OPENSSL_BUNDLE_DIR/include/openssl"
assert "include/openssl 헤더 디렉토리 존재" $?

# 2. install_name이 @rpath 기반으로 변경되었는지 확인
echo ""
echo "[2] dylib install_name 확인"
SSL_ID=$(otool -D "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib" | tail -1)
test "$SSL_ID" = "@rpath/libssl.3.dylib"
assert "libssl install_name = @rpath/libssl.3.dylib (actual: $SSL_ID)" $?

CRYPTO_ID=$(otool -D "$OPENSSL_BUNDLE_DIR/lib/libcrypto.3.dylib" | tail -1)
test "$CRYPTO_ID" = "@rpath/libcrypto.3.dylib"
assert "libcrypto install_name = @rpath/libcrypto.3.dylib (actual: $CRYPTO_ID)" $?

# 3. libssl → libcrypto 참조가 @rpath 기반인지 확인
echo ""
echo "[3] libssl → libcrypto 참조 경로 확인"
CRYPTO_REF=$(otool -L "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib" | grep libcrypto | awk '{print $1}')
test "$CRYPTO_REF" = "@rpath/libcrypto.3.dylib"
assert "libssl → libcrypto = @rpath/libcrypto.3.dylib (actual: $CRYPTO_REF)" $?

# 4. dylib에 절대 경로 참조가 없는지 확인
echo ""
echo "[4] dylib 절대 경로 참조 없음 확인"
ABS=$(otool -L "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib" | grep -c "/opt/homebrew\|/usr/local" || true)
test "$ABS" = "0"
assert "libssl에 절대 경로 참조 없음 (found: $ABS)" $?

ABS=$(otool -L "$OPENSSL_BUNDLE_DIR/lib/libcrypto.3.dylib" | grep -c "/opt/homebrew\|/usr/local" || true)
test "$ABS" = "0"
assert "libcrypto에 절대 경로 참조 없음 (found: $ABS)" $?

# 5. dylib 코드 서명 유효성 확인
echo ""
echo "[5] dylib 코드 서명 확인"
codesign --verify "$OPENSSL_BUNDLE_DIR/lib/libssl.3.dylib" 2>/dev/null
assert "libssl 코드 서명 유효" $?
codesign --verify "$OPENSSL_BUNDLE_DIR/lib/libcrypto.3.dylib" 2>/dev/null
assert "libcrypto 코드 서명 유효" $?

# 6. 메인 바이너리의 OpenSSL 참조 확인 (빌드 시 OPENSSL_DIR로 @rpath 참조가 기록되는지)
echo ""
echo "[6] 메인 바이너리 OpenSSL 참조 확인"
WORKSPACE_ROOT="$(cd "$DESKTOP_DIR/.." && pwd)"
MAIN_BIN="$WORKSPACE_ROOT/target/debug/cheolsu-proxy"
if [ -f "$MAIN_BIN" ]; then
    SSL_REF=$(otool -L "$MAIN_BIN" | grep libssl | awk '{print $1}')
    test "$SSL_REF" = "@rpath/libssl.3.dylib"
    assert "메인 바이너리 libssl = @rpath/libssl.3.dylib (actual: $SSL_REF)" $?

    CRYPTO_REF=$(otool -L "$MAIN_BIN" | grep libcrypto | awk '{print $1}')
    test "$CRYPTO_REF" = "@rpath/libcrypto.3.dylib"
    assert "메인 바이너리 libcrypto = @rpath/libcrypto.3.dylib (actual: $CRYPTO_REF)" $?

    RPATH_EXISTS=$(otool -l "$MAIN_BIN" | grep -A2 LC_RPATH | grep -c "@executable_path/../Frameworks" || true)
    test "$RPATH_EXISTS" -ge 1
    assert "메인 바이너리에 @executable_path/../Frameworks rpath 존재" $?

    ABS=$(otool -L "$MAIN_BIN" | grep -E "libssl|libcrypto" | grep -c "/opt/homebrew\|/usr/local" || true)
    test "$ABS" = "0"
    assert "메인 바이너리에 절대 경로 참조 없음 (found: $ABS)" $?
else
    echo "  SKIP: 메인 바이너리 없음 (cargo build -p cheolsu-proxy 필요)"
fi

# 7. sidecar 바이너리의 OpenSSL 참조 확인
echo ""
echo "[7] sidecar 바이너리 확인"
for bin_name in "cheolsu-proxy-mcp" "cheolsu"; do
    bin_path="$BINARIES_DIR/${bin_name}-${TARGET_TRIPLE}"
    if [ -f "$bin_path" ]; then
        SSL_REF=$(otool -L "$bin_path" | grep libssl | awk '{print $1}')
        test "$SSL_REF" = "@rpath/libssl.3.dylib"
        assert "$bin_name libssl = @rpath/libssl.3.dylib (actual: $SSL_REF)" $?

        CRYPTO_REF=$(otool -L "$bin_path" | grep libcrypto | awk '{print $1}')
        test "$CRYPTO_REF" = "@rpath/libcrypto.3.dylib"
        assert "$bin_name libcrypto = @rpath/libcrypto.3.dylib (actual: $CRYPTO_REF)" $?

        RPATH_EXISTS=$(otool -l "$bin_path" | grep -A2 LC_RPATH | grep -c "@executable_path/../Frameworks" || true)
        test "$RPATH_EXISTS" -ge 1
        assert "$bin_name에 @executable_path/../Frameworks rpath 존재" $?

        ABS=$(otool -L "$bin_path" | grep -E "libssl|libcrypto" | grep -c "/opt/homebrew\|/usr/local" || true)
        test "$ABS" = "0"
        assert "$bin_name에 절대 경로 참조 없음 (found: $ABS)" $?
    else
        echo "  SKIP: $bin_name 바이너리 없음"
    fi
done

# 8. .cargo/config.toml의 OPENSSL_DIR 설정 확인
echo ""
echo "[8] .cargo/config.toml 설정 확인"
CARGO_CONFIG="$WORKSPACE_ROOT/.cargo/config.toml"
test -f "$CARGO_CONFIG"
assert ".cargo/config.toml 존재" $?
grep -q "OPENSSL_DIR" "$CARGO_CONFIG"
assert "OPENSSL_DIR 설정이 config.toml에 존재" $?

# 결과 요약
echo ""
echo "=== 결과: $PASS passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
