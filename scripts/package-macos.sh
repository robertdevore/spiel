#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VERSION="$(node -p 'require("./package.json").version')"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ARCH="x64" ;;
  arm64|aarch64) ARCH="arm64" ;;
esac

APP_PATH="src-tauri/target/release/bundle/macos/Spiel.app"
ARTIFACT_DIR="release/artifacts"
DMG_ROOT="release/macos-dmg-root"
BASE_NAME="Spiel_${VERSION}_macOS_${ARCH}"

npm run tauri -- build --bundles app

rm -rf "$DMG_ROOT"
mkdir -p "$DMG_ROOT" "$ARTIFACT_DIR"
rm -f "$ARTIFACT_DIR/${BASE_NAME}.dmg" "$ARTIFACT_DIR/${BASE_NAME}.app.zip"

cp -R "$APP_PATH" "$DMG_ROOT/"
ln -s /Applications "$DMG_ROOT/Applications"

hdiutil create \
  -volname "Spiel" \
  -srcfolder "$DMG_ROOT" \
  -ov \
  -format UDZO \
  "$ARTIFACT_DIR/${BASE_NAME}.dmg"

ditto -c -k --sequesterRsrc --keepParent \
  "$APP_PATH" \
  "$ARTIFACT_DIR/${BASE_NAME}.app.zip"

echo "macOS artifacts:"
ls -lh "$ARTIFACT_DIR/${BASE_NAME}.dmg" "$ARTIFACT_DIR/${BASE_NAME}.app.zip"
