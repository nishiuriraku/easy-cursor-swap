#!/usr/bin/env bash
# PreToolUse フック: ツール実行前に不変条件違反を検出してブロック
# stdin: { "tool_name": "Edit|Write|Read|...", "tool_input": {...} }
# exit 0  = 許可 (デフォルト)
# exit 2  = ブロック (stderr のメッセージが Claude に渡され自動修正に向かう)
#
# このプロジェクトの致命的不変条件を機械的に強制する:
#   (1) HKLM 書き込み禁止 (UAC 発生防止)
#   (2) v-html 禁止 (XSS 対策)
#   (3) docs/*.html を Read 禁止 (53kトークン無駄遣い)
#   (4) components/ から @tauri-apps/api の invoke 直呼び禁止 (useTauri 経由を強制)
set -e

json=$(cat)
tool_name=$(echo "$json" | jq -r '.tool_name // empty')

# 検査対象ツール以外はスキップ
case "$tool_name" in
  Edit|Write|MultiEdit|Read) ;;
  *) exit 0 ;;
esac

file_path=$(echo "$json" | jq -r '.tool_input.file_path // empty')
rel_path="${file_path#"$PWD/"}"

# 編集内容を抽出 (Edit は new_string、 Write は content、 MultiEdit は edits[].new_string 連結)
case "$tool_name" in
  Edit)      payload=$(echo "$json" | jq -r '.tool_input.new_string // ""') ;;
  Write)     payload=$(echo "$json" | jq -r '.tool_input.content // ""') ;;
  MultiEdit) payload=$(echo "$json" | jq -r '[.tool_input.edits[].new_string] | join("\n")') ;;
  Read)      payload="" ;;
esac

# --- Rule 4: Tier 3 HTML の Read 禁止 ---
if [ "$tool_name" = "Read" ]; then
  case "$rel_path" in
    docs/architecture.html|docs/ui_map.html|*/docs/architecture.html|*/docs/ui_map.html)
      echo "🚫 INVARIANT VIOLATION: docs/*.html (Tier 3) は AI 読み取り禁止 — 53kトークン無駄。" >&2
      echo "   代わりに docs/architecture.json と docs/ui_map.json を読んでください (Tier 1)。" >&2
      echo "   CLAUDE.md 'Documentation map' セクション参照。" >&2
      exit 2
      ;;
  esac
  exit 0
fi

# --- Rule 1: HKLM 書き込み禁止 (Rust ファイルのみ) ---
case "$rel_path" in
  src-tauri/**/*.rs|src-tauri/*.rs)
    if echo "$payload" | grep -qE 'HKLM[\\/]|HKEY_LOCAL_MACHINE|RegOpenKeyEx.*HKLM|hklm\.|registry::HKLM'; then
      echo "🚫 INVARIANT VIOLATION: HKLM 書き込みは禁止です (UAC を発生させる)。" >&2
      echo "   このプロジェクトは HKCU\\Control Panel\\Cursors のみ操作します。" >&2
      echo "   src-tauri/src/registry/mod.rs の既存パターンに従ってください。" >&2
      echo "   ファイル: $rel_path" >&2
      exit 2
    fi
    ;;
esac

# --- Rule 2: v-html 禁止 (Vue ファイル) ---
case "$rel_path" in
  app/**/*.vue|app/*.vue)
    if echo "$payload" | grep -qE 'v-html|innerHTML\s*=|outerHTML\s*='; then
      echo "🚫 INVARIANT VIOLATION: v-html / innerHTML / outerHTML は XSS の温床として禁止です。" >&2
      echo "   SVG は app/components/icons/UiIcon.vue / CursorIcon.vue の render 関数経由で。" >&2
      echo "   ユーザー入力由来の文字列は app/composables/sanitizeSvg を通してください。" >&2
      echo "   ファイル: $rel_path" >&2
      exit 2
    fi
    ;;
esac

# --- Rule 3: components/ で @tauri-apps の invoke 直呼び禁止 ---
case "$rel_path" in
  app/components/**/*.vue|app/components/**/*.ts)
    if echo "$payload" | grep -qE "from\s+['\"]@tauri-apps/api(/core)?['\"]"; then
      if echo "$payload" | grep -qE '\binvoke\b'; then
        echo "🚫 INVARIANT VIOLATION: components/ から @tauri-apps/api の invoke を直呼びしないでください。" >&2
        echo "   app/composables/useTauri.ts の invokeTauri() 経由にしてください。" >&2
        echo "   ファイル: $rel_path" >&2
        exit 2
      fi
    fi
    ;;
esac

exit 0
