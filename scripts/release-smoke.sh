#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "[spiel] building frontend"
npm run build

echo "[spiel] checking rust format"
cargo fmt --manifest-path src-tauri/Cargo.toml --check

echo "[spiel] running rust tests"
cargo test --manifest-path src-tauri/Cargo.toml

echo "[spiel] running clippy"
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings

echo "[spiel] smoke checks passed"
