#!/usr/bin/env bash
# Wave 4A TDD: gen_msix_manifest.ps1
# Usage: bash src-tauri/build-helpers/test/test-gen_msix_manifest.sh
# Each test is a single pwsh invocation; exit code drives red/green.
set -u

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
HELPER_DIR="$ROOT/src-tauri/build-helpers"
SCRIPT="$HELPER_DIR/gen_msix_manifest.ps1"
TEST_ROOT="$ROOT/.claude/tmp/msix-tdd"
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
{
  "version": "0.0.8"
}
JSON
  cat > "$work/in/AppxManifest.xml" <<'XML'
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
         IgnorableNamespaces="rescap">
  <Identity Name="dev.easycursorswap.app"
            Publisher="${PUBLISHER}"
            Version="${VERSION}"
            ProcessorArchitecture="${ARCH}" />
  <Properties><DisplayName>EasyCursorSwap</DisplayName></Properties>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
XML
  echo "$work"
}

run_helper() {
  local work="$1"
  local arch="$2"
  pwsh -NoProfile -ExecutionPolicy Bypass -File "$SCRIPT" \
    -TauriConfPath "$work/in/tauri.conf.json" \
    -TemplatePath "$work/in/AppxManifest.xml" \
    -OutputDir "$work/out" \
    -Arch "$arch" 2>&1
}

# --- Test 1: x64 で 1 ファイル出力される ---
test_x64_emits_one_file() {
  local work; work="$(prepare_fixture t1)"
  run_helper "$work" x64 || fail "helper exited non-zero: $?"
  [[ -f "$work/out/AppxManifest.xml" ]] || fail "AppxManifest.xml not emitted"
  local count; count=$(ls -1 "$work/out" | wc -l)
  [[ "$count" -eq 1 ]] || fail "expected 1 file, got $count"
  pass "x64 emits a single AppxManifest.xml"
}

# --- Test 2: ${VERSION} が tauri.conf.json の version に置換される ---
test_version_substituted() {
  local work; work="$(prepare_fixture t2)"
  run_helper "$work" x64 || fail "helper exited non-zero: $?"
  grep -q 'Version="0.0.8"' "$work/out/AppxManifest.xml" || fail "Version not substituted"
  pass "Version is substituted from tauri.conf.json"
}

# --- Test 3: ${ARCH} が x64 に置換される ---
test_arch_substituted_x64() {
  local work; work="$(prepare_fixture t3)"
  run_helper "$work" x64 || fail "helper exited non-zero: $?"
  grep -q 'ProcessorArchitecture="x64"' "$work/out/AppxManifest.xml" || fail "arch not substituted"
  pass "Architecture is substituted (x64)"
}

# --- Test 4: ${ARCH} が arm64 に置換される ---
test_arch_substituted_arm64() {
  local work; work="$(prepare_fixture t4)"
  run_helper "$work" arm64 || fail "helper exited non-zero: $?"
  grep -q 'ProcessorArchitecture="arm64"' "$work/out/AppxManifest.xml" || fail "arm64 not substituted"
  pass "Architecture is substituted (arm64)"
}

# --- Test 5: ${PUBLISHER} が MSIX_PUBLISHER env で置換される ---
test_publisher_env_override() {
  local work; work="$(prepare_fixture t5)"
  MSIX_PUBLISHER="CN=Test Publisher" run_helper "$work" x64 || fail "helper exited non-zero: $?"
  grep -q 'Publisher="CN=Test Publisher"' "$work/out/AppxManifest.xml" \
    || fail "MSIX_PUBLISHER env did not override placeholder"
  pass "MSIX_PUBLISHER env overrides placeholder"
}

# --- Test 6: MSIX_PUBLISHER 未設定時はヘルパー既定値が使われる ---
test_publisher_default_kept() {
  local work; work="$(prepare_fixture t6)"
  unset MSIX_PUBLISHER
  pwsh -NoProfile -ExecutionPolicy Bypass -File "$SCRIPT" \
    -TauriConfPath "$work/in/tauri.conf.json" \
    -TemplatePath "$work/in/AppxManifest.xml" \
    -OutputDir "$work/out" \
    -Arch x64 2>&1 || fail "helper exited non-zero: $?"
  grep -q 'Publisher="CN=EasyCursorSwap, O=EasyCursorSwap, C=JP"' "$work/out/AppxManifest.xml" \
    || fail "default publisher not preserved"
  pass "MSIX_PUBLISHER env unset keeps default publisher"
}

# --- Test 7: 存在しない Tauri conf path は exit 2 で停止する ---
test_missing_tauri_conf() {
  local work; work="$(prepare_fixture t7)"
  pwsh -NoProfile -ExecutionPolicy Bypass -File "$SCRIPT" \
    -TauriConfPath "$work/in/missing.json" \
    -TemplatePath "$work/in/AppxManifest.xml" \
    -OutputDir "$work/out" \
    -Arch x64 >/dev/null 2>&1
  local rc=$?
  [[ "$rc" -eq 2 ]] || fail "expected exit 2 on missing conf, got $rc"
  pass "missing tauri.conf.json exits with code 2"
}

# --- Test 8: 不正な arch は exit 3 で停止する ---
test_invalid_arch() {
  local work; work="$(prepare_fixture t8)"
  pwsh -NoProfile -ExecutionPolicy Bypass -File "$SCRIPT" \
    -TauriConfPath "$work/in/tauri.conf.json" \
    -TemplatePath "$work/in/AppxManifest.xml" \
    -OutputDir "$work/out" \
    -Arch riscv >/dev/null 2>&1
  local rc=$?
  [[ "$rc" -eq 3 ]] || fail "expected exit 3 on invalid arch, got $rc"
  pass "invalid arch exits with code 3"
}

# --- Test 9: 生成結果は well-formed XML としてパース可能 ---
test_output_is_well_formed_xml() {
  local work; work="$(prepare_fixture t9)"
  run_helper "$work" x64 || fail "helper exited non-zero: $?"
  pwsh -NoProfile -Command "
    try { [xml]\$(Get-Content -Path '$work/out/AppxManifest.xml' -Raw); 'PARSE_OK' }
    catch { 'PARSE_FAIL: ' + \$_.Exception.Message }
  " 2>&1 | grep -q "PARSE_OK" || fail "output is not well-formed XML"
  pass "output parses as well-formed XML"
}

# --- main ---
case "${1:-all}" in
  t1) test_x64_emits_one_file ;;
  t2) test_version_substituted ;;
  t3) test_arch_substituted_x64 ;;
  t4) test_arch_substituted_arm64 ;;
  t5) test_publisher_env_override ;;
  t6) test_publisher_default_kept ;;
  t7) test_missing_tauri_conf ;;
  t8) test_invalid_arch ;;
  t9) test_output_is_well_formed_xml ;;
  all)
    test_x64_emits_one_file
    test_version_substituted
    test_arch_substituted_x64
    test_arch_substituted_arm64
    test_publisher_env_override
    test_publisher_default_kept
    test_missing_tauri_conf
    test_invalid_arch
    test_output_is_well_formed_xml
    ;;
  *) echo "unknown test: $1" >&2; exit 64 ;;
esac
