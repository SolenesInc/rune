#!/bin/bash
# Builds <out>/BrowserTiles.app: the prototype binary, and with CEF the framework and five helper apps.
#   scripts/bundle.sh <out-dir> [--no-cef]
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
out=$1
cef=1
[ "${2:-}" = "--no-cef" ] && cef=0
. "$here/env.sh"
cd "$here"
if [ $cef = 1 ]; then cargo build --release --bins; else cargo build --release --no-default-features --bin browser-tiles; fi
app="$out/BrowserTiles.app"
[ -d "$app" ] && rm -r "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Frameworks"
plist() { # path exe id helper
  {
    echo '<?xml version="1.0" encoding="UTF-8"?>'
    echo '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">'
    echo '<plist version="1.0"><dict>'
    echo "<key>CFBundleExecutable</key><string>$2</string>"
    echo "<key>CFBundleIdentifier</key><string>$3</string>"
    echo "<key>CFBundleName</key><string>$2</string>"
    echo '<key>CFBundlePackageType</key><string>APPL</string>'
    echo '<key>CFBundleVersion</key><string>0.0.1</string>'
    echo '<key>NSHighResolutionCapable</key><true/>'
    echo '<key>LSMinimumSystemVersion</key><string>12.0</string>'
    [ "$4" = 1 ] && echo '<key>LSUIElement</key><string>1</string>'
    echo '</dict></plist>'
  } > "$1"
}
cp "$CARGO_TARGET_DIR/release/browser-tiles" "$app/Contents/MacOS/BrowserTiles"
plist "$app/Contents/Info.plist" BrowserTiles dev.attn.browser-tiles 0
if [ $cef = 1 ]; then
  cp -R "$CEF_PATH/Chromium Embedded Framework.framework" "$app/Contents/Frameworks/"
  i=0
  for suffix in "" " (GPU)" " (Renderer)" " (Plugin)" " (Alerts)"; do
    name="BrowserTiles Helper$suffix"
    h="$app/Contents/Frameworks/$name.app"
    mkdir -p "$h/Contents/MacOS"
    cp "$CARGO_TARGET_DIR/release/browser-tiles-helper" "$h/Contents/MacOS/$name"
    plist "$h/Contents/Info.plist" "$name" "dev.attn.browser-tiles.helper$i" 1
    i=$((i + 1))
  done
fi
du -sh "$app"
