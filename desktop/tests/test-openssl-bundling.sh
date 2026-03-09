#!/bin/bash
# OpenSSL dylib 번들링 결과 검증 테스트
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DESKTOP_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
FRAMEWORKS_DIR="$DESKTOP_DIR/src-tauri/frameworks"
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

# 1. frameworks 디렉토리에 dylib 존재 확인
echo "[1] dylib 파일 존재 확인"
test -f "$FRAMEWORKS_DIR/libssl.3.dylib"
assert "libssl.3.dylib 존재" $?
test -f "$FRAMEWORKS_DIR/libcrypto.3.dylib"
assert "libcrypto.3.dylib 존재" $?

# 2. install_name이 @rpath 기반으로 변경되었는지 확인
echo ""
echo "[2] install_name 확인"
SSL_ID=$(otool -D "$FRAMEWORKS_DIR/libssl.3.dylib" | tail -1)
test "$SSL_ID" = "@rpath/libssl.3.dylib"
assert "libssl install_name = @rpath/libssl.3.dylib (actual: $SSL_ID)" $?

CRYPTO_ID=$(otool -D "$FRAMEWORKS_DIR/libcrypto.3.dylib" | tail -1)
test "$CRYPTO_ID" = "@rpath/libcrypto.3.dylib"
assert "libcrypto install_name = @rpath/libcrypto.3.dylib (actual: $CRYPTO_ID)" $?

# 3. libssl → libcrypto 참조가 @rpath 기반인지 확인
echo ""
echo "[3] libssl → libcrypto 참조 경로 확인"
CRYPTO_REF=$(otool -L "$FRAMEWORKS_DIR/libssl.3.dylib" | grep libcrypto | awk '{print $1}')
test "$CRYPTO_REF" = "@rpath/libcrypto.3.dylib"
assert "libssl → libcrypto 참조 = @rpath/libcrypto.3.dylib (actual: $CRYPTO_REF)" $?

# 4. libssl/libcrypto에 절대 경로 참조가 남아있지 않은지 확인
echo ""
echo "[4] 절대 경로 참조 없음 확인"
ABSOLUTE_REFS=$(otool -L "$FRAMEWORKS_DIR/libssl.3.dylib" | grep -c "/opt/homebrew\|/usr/local" || true)
test "$ABSOLUTE_REFS" = "0"
assert "libssl에 Homebrew 절대 경로 참조 없음 (found: $ABSOLUTE_REFS)" $?

ABSOLUTE_REFS=$(otool -L "$FRAMEWORKS_DIR/libcrypto.3.dylib" | grep -c "/opt/homebrew\|/usr/local" || true)
test "$ABSOLUTE_REFS" = "0"
assert "libcrypto에 Homebrew 절대 경로 참조 없음 (found: $ABSOLUTE_REFS)" $?

# 5. 코드 서명 유효성 확인
echo ""
echo "[5] 코드 서명 유효성 확인"
codesign --verify "$FRAMEWORKS_DIR/libssl.3.dylib" 2>/dev/null
assert "libssl 코드 서명 유효" $?
codesign --verify "$FRAMEWORKS_DIR/libcrypto.3.dylib" 2>/dev/null
assert "libcrypto 코드 서명 유효" $?

# 6. sidecar 바이너리의 OpenSSL 참조 확인
echo ""
echo "[6] sidecar 바이너리 OpenSSL 참조 확인"
for bin_name in "cheolsu-proxy-mcp" "cheolsu"; do
    bin_path="$BINARIES_DIR/${bin_name}-${TARGET_TRIPLE}"
    if [ -f "$bin_path" ]; then
        # libssl 참조가 @rpath 기반인지 확인
        SSL_REF=$(otool -L "$bin_path" | grep libssl | awk '{print $1}')
        test "$SSL_REF" = "@rpath/libssl.3.dylib"
        assert "$bin_name libssl 참조 = @rpath/libssl.3.dylib (actual: $SSL_REF)" $?

        # libcrypto 참조가 @rpath 기반인지 확인
        CRYPTO_REF=$(otool -L "$bin_path" | grep libcrypto | awk '{print $1}')
        test "$CRYPTO_REF" = "@rpath/libcrypto.3.dylib"
        assert "$bin_name libcrypto 참조 = @rpath/libcrypto.3.dylib (actual: $CRYPTO_REF)" $?

        # @executable_path/../Frameworks rpath가 존재하는지 확인
        RPATH_EXISTS=$(otool -l "$bin_path" | grep -A2 LC_RPATH | grep -c "@executable_path/../Frameworks" || true)
        test "$RPATH_EXISTS" -ge 1
        assert "$bin_name에 @executable_path/../Frameworks rpath 존재" $?

        # 절대 경로 참조가 남아있지 않은지 확인
        ABS=$(otool -L "$bin_path" | grep -E "libssl|libcrypto" | grep -c "/opt/homebrew\|/usr/local" || true)
        test "$ABS" = "0"
        assert "$bin_name에 Homebrew 절대 경로 참조 없음 (found: $ABS)" $?

        # 코드 서명 유효성 확인
        codesign --verify "$bin_path" 2>/dev/null
        assert "$bin_name 코드 서명 유효" $?
    else
        echo "  SKIP: $bin_name 바이너리 없음 ($bin_path)"
    fi
done

# 7. Tauri 메인 바이너리의 rpath 확인
echo ""
echo "[7] Tauri 메인 바이너리 rpath 확인"
WORKSPACE_ROOT="$(cd "$DESKTOP_DIR/.." && pwd)"
MAIN_BIN="$WORKSPACE_ROOT/target/debug/cheolsu-proxy"
if [ -f "$MAIN_BIN" ]; then
    RPATH_EXISTS=$(otool -l "$MAIN_BIN" | grep -A2 LC_RPATH | grep -c "@executable_path/../Frameworks" || true)
    test "$RPATH_EXISTS" -ge 1
    assert "메인 바이너리에 @executable_path/../Frameworks rpath 존재" $?
else
    echo "  SKIP: 메인 바이너리 없음 (cargo build -p cheolsu-proxy 필요)"
fi

# 결과 요약
echo ""
echo "=== 결과: $PASS passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
