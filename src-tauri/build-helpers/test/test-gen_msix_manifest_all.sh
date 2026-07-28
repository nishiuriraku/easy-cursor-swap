#!/usr/bin/env bash
# Wave 4A.2 TDD: gen_msix_manifest_all.sh
set -u

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
DRIVER="$ROOT/src-tauri/build-helpers/gen_msix_manifest_all.sh"
SCRIPT="$ROOT/src-tauri/build-helpers/gen_msix_manifest.ps1"
TEST_ROOT="$ROOT/.claude/tmp/msix-tdd-all"
mkdir -p "$TEST_ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }
cleanup() { rm -rf "$TEST_ROOT"; }
trap cleanup EXIT

prepare_fixture() {
  local sub="$1"
  local work="$TEST_ROOT/$sub"
  mkdir -p "$work/in" "$work/out"
  cat > "$work/in/tauri.conf.json" <<'JSON'
{ "version": "0.0.8" }
JSON
  cat > "$work/in/AppxManifest.xml" <<'XML'
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
         IgnorableNamespaces="rescap">
  <Identity Name="dev.easycursorswap.app" Publisher="${PUBLISHER}" Version="${VERSION}" ProcessorArchitecture="${ARCH}" />
  <Properties><DisplayName>EasyCursorSwap</DisplayName></Properties>
  <Capabilities><rescap:Capability Name="runFullTrust" /></Capabilities>
</Package>
XML
  echo "$work"
}

# --- Test 1: x64 と arm64 の 2 ファイルを同時生成する ---
test_emits_both_architectures() {
  local work; work="$(prepare_fixture t1)"
  pwsh -NoProfile -ExecutionPolicy Bypass -File "$SCRIPT" \
    -TauriConfPath "$work/in/tauri.conf.json" \
    -TemplatePath "$work/in/AppxManifest.xml" \
    -OutputDir "$work/out/x64" -Arch x64 || fail "x64 helper failed"
  pwsh -NoProfile -ExecutionPolicy Bypass -File "$SCRIPT" \
    -TauriConfPath "$work/in/tauri.conf.json" \
    -TemplatePath "$work/in/AppxManifest.xml" \
    -OutputDir "$work/out/arm64" -Arch arm64 || fail "arm64 helper failed"
  [[ -f "$work/out/x64/AppxManifest.xml" ]] || fail "x64 AppxManifest.xml missing"
  [[ -f "$work/out/arm64/AppxManifest.xml" ]] || fail "arm64 AppxManifest.xml missing"
  grep -q 'ProcessorArchitecture="x64"' "$work/out/x64/AppxManifest.xml" || fail "x64 arch not set"
  grep -q 'ProcessorArchitecture="arm64"' "$work/out/arm64/AppxManifest.xml" || fail "arm64 arch not set"
  pass "both x64 and arm64 manifests are emitted with correct arch"
}

# --- Test 2: driver-style invocation が両アーキで成功する ---
test_driver_emits_both() {
  local work; work="$(prepare_fixture t2)"
  OUT_BASE="$work/out" TAURI_CONF_OVERRIDE="$work/in/tauri.conf.json" TEMPLATE_OVERRIDE="$work/in/AppxManifest.xml" \
    bash -c '
      set -u
      ROOT="'"$ROOT"'"
      TPL="${TEMPLATE_OVERRIDE:-$ROOT/distribution/msix/AppxManifest.xml}"
      CONF="${TAURI_CONF_OVERRIDE:-$ROOT/src-tauri/tauri.conf.json}"
      mkdir -p "$OUT_BASE/x64" "$OUT_BASE/arm64"
      pwsh -NoProfile -ExecutionPolicy Bypass -File "$ROOT/src-tauri/build-helpers/gen_msix_manifest.ps1" \
        -TauriConfPath "$CONF" -TemplatePath "$TPL" -OutputDir "$OUT_BASE/x64" -Arch x64
      pwsh -NoProfile -ExecutionPolicy Bypass -File "$ROOT/src-tauri/build-helpers/gen_msix_manifest.ps1" \
        -TauriConfPath "$CONF" -TemplatePath "$TPL" -OutputDir "$OUT_BASE/arm64" -Arch arm64
    ' || fail "driver-style invocation failed"
  [[ -f "$work/out/x64/AppxManifest.xml" ]] || fail "x64 missing"
  [[ -f "$work/out/arm64/AppxManifest.xml" ]] || fail "arm64 missing"
  pass "driver-style invocation emits both architectures"
}

# --- Test 3: ドライバ本体に x64 と arm64 の 2 アーキ参照が含まれる ---
test_driver_references_both_archs() {
  grep -q "x64" "$DRIVER" || fail "x64 not referenced in driver"
  grep -q "arm64" "$DRIVER" || fail "arm64 not referenced in driver"
  pass "driver references both x64 and arm64"
}

case "${1:-all}" in
  t1) test_emits_both_architectures ;;
  t2) test_driver_emits_both ;;
  t3) test_driver_references_both_archs ;;
  all)
    test_emits_both_architectures
    test_driver_emits_both
    test_driver_references_both_archs
    ;;
  *) echo "unknown test: $1" >&2; exit 64 ;;
esac
