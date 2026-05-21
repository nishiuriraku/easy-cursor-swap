#!/usr/bin/env bash
# SessionStart フック: 会話冒頭にプロジェクトの実測カウントを注入
# stdout の内容は Claude の context にそのまま入る
set -e

ARCH_JSON="docs/architecture.json"
if [ ! -f "$ARCH_JSON" ]; then
  exit 0
fi

# 直近の git status と未コミット要約も合わせて
branch=$(git branch --show-current 2>/dev/null || echo "?")
modified=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')
last_commit=$(git log -1 --format='%h %s' 2>/dev/null || echo "?")

# Pending snapshot 検出 (前回 panic 終了の痕跡)
pending=""
if [ -f "$HOME/.custom_cursors/_pending_apply.snapshot" ]; then
  pending="⚠️  _pending_apply.snapshot が残存 — 前回の適用が異常終了した可能性。 reset_to_windows_default で復旧推奨。"
fi

# architecture.json から measured_counts を取得
counts=$(jq -r '
  .meta.measured_counts |
  "- Rust modules (lib.rs): \(.rust_modules_in_lib_rs)\n" +
  "- Tauri IPC commands:    \(.tauri_ipc_commands)\n" +
  "- Composables:           \(.composables)\n" +
  "- Vue pages:             \(.pages_vue) (+ \(.pages_ts_helpers) helpers)\n" +
  "- Components total:      \(.components_total)\n" +
  "- CI workflows:          \(.ci_workflows)"
' "$ARCH_JSON")

generated_at=$(jq -r '.generated_at // .meta.generated_at // "?"' "$ARCH_JSON")
drift_warnings=$(jq -r '.meta.doc_drift_warnings[0] // ""' "$ARCH_JSON" | head -c 200)

cat <<EOF
## EasyCursorSwap — Session start snapshot

**Branch**: \`$branch\` ($modified modified files) | **Last commit**: $last_commit
**Living docs**: \`docs/architecture.json\` (generated: $generated_at)

### Measured counts (Tier 1)
$counts

### Critical invariants (re-check before any change)
1. HKCU only (no HKLM, no UAC)
2. Transactional apply (_pending_apply.snapshot pattern)
3. PII redaction in logs (redact_path / short_hash)
4. Archive sanitisation (theme::sanitize_archive_path)
5. No \`v-html\` anywhere
6. IPC types in app/types/ mirror Rust serde structs

$pending

> Tip: 数値がコードと食い違ったら \`grep -c '#\\[tauri::command\\]' src-tauri/src/commands/**/*.rs\` で実測し、 architecture.json を更新してください。
EOF
