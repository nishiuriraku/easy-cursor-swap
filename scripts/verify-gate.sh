#!/usr/bin/env bash
# 検証ゲート: リファクタの一区切りごとに実行する
# fmt + clippy + cargo test + prettier + vue-tsc + i18n + vitest
set -e
echo "=== cargo fmt ==="
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
echo "=== cargo clippy ==="
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
echo "=== cargo test --lib ==="
cargo test --manifest-path src-tauri/Cargo.toml --lib --quiet
echo "=== prettier --check ==="
npm run --silent format:check
echo "=== vue-tsc ==="
npx vue-tsc --noEmit
echo "=== i18n parity ==="
node scripts/check-i18n.mjs
echo "=== vitest ==="
npm test --silent
echo "=== ALL GREEN ==="
