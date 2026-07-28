#!/usr/bin/env bash
# 検証ゲート (統合) — コミット前に必ず実行する
# 分割版 (verify-gate-backend.sh / verify-gate-frontend.sh) を順次呼ぶ
set -e

# DTO drift gate (Rust → TypeScript 生成物の整合性チェック)。
# 一時ディレクトリに `cargo run --features typegen --bin gen_types` を実行し、
# コミット済みの `app/types/generated/` と diff する。
# - 欠落 / 余分 / 変更 / 未追跡 のいずれかが残っていると赤。
# - `.cargo/config.toml` の `TS_RS_EXPORT_DIR` 設定はシェル環境変数で上書きされる
#   (cargo の env table は `force=true` を付けない限りシェル env を尊重する)。
# - 終了時に一時ディレクトリは必ず `trap` で削除。
# - このステップは `node scripts/gen-architecture.mjs --check` (frontmatter ↔ source)
#   とは独立。gen-architecture は所有マップのドリフト検知、这里是生成物本体のドリフト検知。
# - `app/types/generated/index.ts` は hand-maintained の stable barrel
#   (`import type { AppConfig } from '~/types/generated'` 用の名前集約) で
#   Rust 側からは生成されないため、diff と未追跡チェックの対象外にする。
#   代わりにトラックされていることを別途 assert する。
echo "=== DTO drift (Rust → TypeScript 生成物) ==="
GEN_TMP="$(mktemp -d)"
# shellcheck disable=SC2064 — we want the current value of $GEN_TMP captured now.
trap "rm -rf '$GEN_TMP'" EXIT
TS_RS_EXPORT_DIR="$GEN_TMP" \
  cargo run --manifest-path src-tauri/Cargo.toml \
    --features typegen --bin gen_types --quiet
if ! diff -ru \
    --exclude="index.ts" \
    "$GEN_TMP" app/types/generated; then
  echo "Generated TypeScript is stale; run the type generator and commit its output." >&2
  exit 1
fi
if test -n "$(git ls-files --others --exclude-standard -- app/types/generated)"; then
  echo "Untracked generated TypeScript detected." >&2
  exit 1
fi
# index.ts は hand-maintained barrel。生成物じゃないが、stable な存在として
# 必ずトラックされていること (誰かが ignore 設定で隠していないか) を確認する。
if ! git ls-files --error-unmatch -- app/types/generated/index.ts >/dev/null 2>&1; then
  echo "app/types/generated/index.ts is hand-maintained and must be tracked." >&2
  echo "If you intended to delete the barrel, also update app/types/{config,marketplace}.ts." >&2
  exit 1
fi

bash "$(dirname "$0")/verify-gate-backend.sh"
bash "$(dirname "$0")/verify-gate-frontend.sh"
echo "=== index.json drift (markdown frontmatter ↔ source) ==="
# vault が無い環境 (CI runner 等) では gen-architecture.mjs が自前で skip (exit 0)。
# 構造ドリフト (count 不一致 / 未記載 IPC・module) のみ赤にする。
node "$(dirname "$0")/gen-architecture.mjs" --check
echo "=== ALL GREEN ==="