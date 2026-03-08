#!/bin/bash
# 타우리 앱 전체 빌드 스크립트
# 사용법: ./build.sh [--dmg]
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# 옵션 파싱
BUNDLES="app"
for arg in "$@"; do
    case $arg in
        --dmg) BUNDLES="app,dmg" ;;
    esac
done

# 1. Xcode Command Line Tools 확인
if ! xcode-select -p &>/dev/null; then
    echo "❌ Xcode Command Line Tools가 설치되어 있지 않습니다."
    echo "   실행: xcode-select --install"
    exit 1
fi

# 2. Rust 툴체인 확인
if ! command -v rustc &>/dev/null; then
    echo "❌ Rust가 설치되어 있지 않습니다."
    echo "   실행: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# 3. bun 확인 및 의존성 설치
cd "$SCRIPT_DIR/desktop"
if ! command -v bun &>/dev/null; then
    echo "❌ bun이 설치되어 있지 않습니다."
    echo "   실행: curl -fsSL https://bun.sh/install | bash"
    exit 1
fi

if [ ! -d "node_modules" ]; then
    echo "📦 의존성 설치 중..."
    bun install
fi

# 4. TAURI_SIGNING_PRIVATE_KEY 확인
if [ -z "$TAURI_SIGNING_PRIVATE_KEY" ]; then
    echo "❌ TAURI_SIGNING_PRIVATE_KEY 환경변수가 설정되어 있지 않습니다."
    echo "   ~/.zshrc에 아래를 추가하세요:"
    echo '   export TAURI_SIGNING_PRIVATE_KEY="키값"'
    echo '   export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""'
    exit 1
fi

# 5. 타우리 빌드 (beforeBuildCommand에서 build-mcp.sh release 자동 실행)
echo "🔨 타우리 앱 빌드 중... (bundles: $BUNDLES)"
bun run tauri build -- --bundles "$BUNDLES"

# 6. 빌드 결과 안내
APP_PATH="$SCRIPT_DIR/desktop/src-tauri/target/release/bundle/macos"
echo ""
echo "✅ 빌드 완료!"
echo "📍 앱 위치: $APP_PATH"
echo ""
echo "설치 후 실행이 안 되면 아래 명령어를 실행하세요:"
echo "   xattr -cr \"$APP_PATH/Cheolsu Proxy.app\""
