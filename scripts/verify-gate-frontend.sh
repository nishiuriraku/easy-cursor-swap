#!/usr/bin/env bash
# Frontend (Nuxt/Vue/TS) 専用ゲート — Edit 後の PostToolUse hook から呼ばれる
set -e
echo "=== prettier --check ==="
npm run --silent format:check
echo "=== vue-tsc ==="
npx vue-tsc --noEmit
echo "=== i18n parity ==="
node scripts/check-i18n.mjs
echo "=== vitest ==="
npm test --silent
echo "=== FRONTEND GREEN ==="
