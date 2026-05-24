#!/usr/bin/env bash
# Backend (Rust) 専用ゲート — Edit 後の PostToolUse hook から呼ばれる
set -e
echo "=== cargo fmt ==="
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
echo "=== cargo clippy ==="
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
echo "=== cargo test --lib ==="
cargo test --manifest-path src-tauri/Cargo.toml --lib --quiet
echo "=== BACKEND GREEN ==="
