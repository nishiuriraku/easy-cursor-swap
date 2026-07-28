#!/usr/bin/env bash
# Wave 4A.2: x64 / arm64 両 AppxManifest.xml を並列生成する driver。
# Usage: bash src-tauri/build-helpers/gen_msix_manifest_all.sh [path/to/tauri.conf.json]
set -u

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
TEMPLATE="$ROOT/distribution/msix/AppxManifest.xml"
OUT_BASE="$ROOT/distribution/msix/out"
TAURI_CONF="${1:-$ROOT/src-tauri/tauri.conf.json}"
SCRIPT="$ROOT/src-tauri/build-helpers/gen_msix_manifest.ps1"

fail() { echo "FAIL: $*" >&2; exit 1; }
[[ -f "$TAURI_CONF" ]] || fail "tauri.conf.json not found: $TAURI_CONF"
[[ -f "$TEMPLATE" ]] || fail "AppxManifest.xml template not found: $TEMPLATE"

mkdir -p "$OUT_BASE/x64" "$OUT_BASE/arm64"

for arch in x64 arm64; do
  pwsh -NoProfile -ExecutionPolicy Bypass -File "$SCRIPT" \
    -TauriConfPath "$TAURI_CONF" \
    -TemplatePath "$TEMPLATE" \
    -OutputDir "$OUT_BASE/$arch" \
    -Arch "$arch" || fail "manifest generation failed for $arch"
done

echo "OK: wrote $OUT_BASE/x64/AppxManifest.xml and $OUT_BASE/arm64/AppxManifest.xml"
