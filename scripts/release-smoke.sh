#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

clt_cxx_include="/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/c++/v1"
if [[ -d "$clt_cxx_include" && -z "${CPLUS_INCLUDE_PATH:-}" ]]; then
  export CPLUS_INCLUDE_PATH="$clt_cxx_include"
fi

echo "[spiel] building frontend"
npm run build

echo "[spiel] building transcription helper"
npm run build:helper

echo "[spiel] checking rust format"
cargo fmt --manifest-path src-tauri/Cargo.toml --check

echo "[spiel] running rust tests"
cargo test --manifest-path src-tauri/Cargo.toml

echo "[spiel] running clippy"
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings

echo "[spiel] smoke checks passed"
