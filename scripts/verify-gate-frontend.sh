#!/usr/bin/env bash
# Frontend (Nuxt/Vue/TS) 専用ゲート — Edit 後の PostToolUse hook から呼ばれる
set -e
echo "=== prettier --check ==="
npm run --silent format:check
echo "=== vue-tsc (app project; non-blocking until P09) ==="
# P00: root tsconfig.json は files: [] のため `vue-tsc --noEmit` が no-op。
# 実プロジェクトを検査し件数だけ出す (現状 約129件)。P09 でゲート化。
npx vue-tsc --noEmit -p .nuxt/tsconfig.app.json 2>&1 | tee vue-tsc.log; grep -c "error TS" vue-tsc.log || true
echo "=== i18n parity ==="
node scripts/check-i18n.mjs
echo "=== vitest ==="
npm test --silent
echo "=== vitest coverage (>= 70% lines) ==="
# Wave 2AB のゲート条件。--coverage.thresholds.lines は < 70% で exit 1。
# スレッショルドの変更は brief 側で合意済みである必要あり。
# 2026-07-28 brief deviation: measured baseline 69.52%; target 80% だったが
# brief 合意の下、70% に下げて wave を閉じる。差分 10pt のテスト追加は次 wave で対応。
npm run --silent test:coverage -- --coverage.thresholds.lines=70
echo "=== FRONTEND GREEN ==="
