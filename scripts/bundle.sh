#!/usr/bin/env bash
set -euo pipefail

CARBONYL_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INSTALL_DIR="${CARBONYL_INSTALL_DIR:-$HOME/Downloads/carbonyl-0.0.3}"
DIST="${CARBONYL_DIST_DIR:-$CARBONYL_ROOT/dist}"

for f in carbonyl libcarbonyl.dylib libEGL.dylib libGLESv2.dylib icudtl.dat v8_context_snapshot.x86_64.bin; do
    if [ ! -f "$INSTALL_DIR/$f" ]; then
        echo "Missing $INSTALL_DIR/$f"
        echo "Run scripts/dev-install.sh first (or set CARBONYL_INSTALL_DIR)."
        exit 1
    fi
done

ARCH="$(uname -m)"
case "$ARCH" in
    arm64) TARGET="aarch64-apple-darwin" ;;
    x86_64) TARGET="x86_64-apple-darwin" ;;
    *) echo "Unsupported arch $ARCH"; exit 1 ;;
esac

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
