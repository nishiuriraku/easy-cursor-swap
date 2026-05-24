#!/usr/bin/env bash
# 検証ゲート (統合) — コミット前に必ず実行する
# 分割版 (verify-gate-backend.sh / verify-gate-frontend.sh) を順次呼ぶ
set -e
bash "$(dirname "$0")/verify-gate-backend.sh"
bash "$(dirname "$0")/verify-gate-frontend.sh"
echo "=== ALL GREEN ==="
