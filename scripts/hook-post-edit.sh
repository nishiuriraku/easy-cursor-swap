#!/usr/bin/env bash
# PostToolUse フック: Edit/Write 完了後に呼ばれる
# stdin に { "tool_input": { "file_path": "..." }, ... } 形式の JSON が流れてくる
# file_path の拡張子で振り分けて、 軽量チェックだけ走らせる (Blocking on failure)
#
# プロファイル: 軽量 + Blocking + MCP リマインダ有効
#   - 重い処理 (clippy / cargo test / vue-tsc / vitest) は Stop hook (会話終了時) に回す
#   - 失敗時は exit 1 で Claude をブロック (= 自動でエラーを見て修正に向かう)
#   - フロント編集後は Tauri MCP webview_screenshot のリマインダを echo
set -e

# stdin から file_path 抽出
json=$(cat)
file_path=$(echo "$json" | jq -r '.tool_input.file_path // empty')

# 編集対象が特定できない (Edit 以外の Write など) → スキップ
if [ -z "$file_path" ]; then
  exit 0
fi

# 相対化 (repo root からの見た目を整える)
rel_path="${file_path#"$PWD/"}"

case "$rel_path" in
  src-tauri/**/*.rs | src-tauri/*.rs)
    echo "🦀 [post-edit] Rust edited: $rel_path → cargo fmt --check"
    cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
    echo "   ✓ fmt OK (clippy / test は Stop hook で実行)"
    ;;
  app/locales/*.ts)
    echo "🌐 [post-edit] locale edited: $rel_path → i18n parity check"
    node scripts/check-i18n.mjs
    echo "   ✓ ja/en parity OK"
    ;;
  app/**/*.vue | app/**/*.ts | app/**/*.mjs)
    echo "🎨 [post-edit] Frontend edited: $rel_path"
    echo "   → prettier --check"
    npm run --silent format:check
    echo "   ✓ prettier OK (vue-tsc / vitest は Stop hook で実行)"
    echo ""
    echo "💡 Visual regression reminder:"
    echo "   tauri:dev が起動中なら、 mcp___hypothesi_tauri-mcp-server__webview_screenshot で"
    echo "   修正前後のスナップショットを撮って差分を確認してください。"
    echo "   DOM 差分: mcp___hypothesi_tauri-mcp-server__webview_dom_snapshot"
    ;;
  *)
    # その他 (docs/, README, scripts/, *.json など) はスキップ
    exit 0
    ;;
esac
