#!/usr/bin/env bash
# 検証ゲート (統合) — コミット前に必ず実行する
# 分割版 (verify-gate-backend.sh / verify-gate-frontend.sh) を順次呼ぶ
set -e
bash "$(dirname "$0")/verify-gate-backend.sh"
bash "$(dirname "$0")/verify-gate-frontend.sh"
echo "=== architecture.json drift ==="
# vault が無い環境 (CI runner 等) では gen-architecture.mjs が自前で skip (exit 0)。
# 構造ドリフト (count 不一致 / 未記載 IPC・module) のみ赤にする。
node "$(dirname "$0")/gen-architecture.mjs" --check
echo "=== ALL GREEN ==="
