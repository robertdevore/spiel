#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source scripts/build-env.sh

TARGET_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
HELPER_DIR="src-tauri/binaries"
HELPER_SRC="src-tauri/target/release/spiel-transcribe"
HELPER_DST="$HELPER_DIR/spiel-transcribe-$TARGET_TRIPLE"

cargo build --manifest-path src-tauri/Cargo.toml --release --bin spiel-transcribe
mkdir -p "$HELPER_DIR"
cp "$HELPER_SRC" "$HELPER_DST"
