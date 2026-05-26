#!/usr/bin/env bash
set -euo pipefail

CARBONYL_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INSTALL_DIR="${CARBONYL_INSTALL_DIR:-$HOME/Downloads/carbonyl-0.0.3}"
DIST="${CARBONYL_DIST_DIR:-$CARBONYL_ROOT/dist}"

# Detect target from the v8 snapshot filename in INSTALL_DIR.
if [ -f "$INSTALL_DIR/v8_context_snapshot.arm64.bin" ]; then
    TARGET="aarch64-apple-darwin"
elif [ -f "$INSTALL_DIR/v8_context_snapshot.x86_64.bin" ]; then
    TARGET="x86_64-apple-darwin"
elif [ -f "$INSTALL_DIR/v8_context_snapshot.bin" ]; then
    TARGET="x86_64-unknown-linux-gnu"
else
    echo "Cannot determine target arch from $INSTALL_DIR"
    echo "Run scripts/dev-install.sh first (or set CARBONYL_INSTALL_DIR)."
    exit 1
fi

case "$TARGET" in
    aarch64-apple-darwin|x86_64-apple-darwin)
        REQUIRED=(carbonyl libcarbonyl.dylib libEGL.dylib libGLESv2.dylib icudtl.dat)
        ;;
    x86_64-unknown-linux-gnu)
        REQUIRED=(carbonyl libcarbonyl.so libEGL.so libGLESv2.so libvk_swiftshader.so libvulkan.so.1 vk_swiftshader_icd.json icudtl.dat)
        ;;
esac

for f in "${REQUIRED[@]}"; do
    if [ ! -f "$INSTALL_DIR/$f" ]; then
        echo "Missing $INSTALL_DIR/$f"
        echo "Run scripts/dev-install.sh first (or set CARBONYL_INSTALL_DIR)."
        exit 1
    fi
done

rustup target add "$TARGET" >/dev/null 2>&1 || true

cd "$CARBONYL_ROOT/tools/bundler"
CARBONYL_PAYLOAD_DIR="$INSTALL_DIR" cargo build --release --target "$TARGET"

mkdir -p "$DIST"
OUT="$DIST/carbonyl"
cp "target/$TARGET/release/carbonyl-bundle" "$OUT"
chmod +x "$OUT"

echo "Bundled binary: $OUT"
ls -lh "$OUT"
echo
echo "Run anywhere: $OUT <url>"
echo "First launch extracts to \$XDG_CACHE_HOME/carbonyl/<hash>/ (~165 MB)."
