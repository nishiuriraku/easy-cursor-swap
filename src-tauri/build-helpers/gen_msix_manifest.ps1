<#
.SYNOPSIS
  AppxManifest.xml テンプレからアーキ別 (x64 / arm64) manifest を生成する。

.DESCRIPTION
  - Tauri conf の version を埋め込む
  - ${VERSION} / ${ARCH} / ${PUBLISHER} を置換する
  - MSIX_PUBLISHER env が指定されていれば Publisher を上書きする
  - 出力は 1 ファイル: <OutputDir>/AppxManifest.xml

.PARAMETER TauriConfPath
  tauri.conf.json の絶対パス。

.PARAMETER TemplatePath
  AppxManifest.xml テンプレの絶対パス。

.PARAMETER OutputDir
  生成先ディレクトリ。既存ファイルは上書き。

.PARAMETER Arch
  "x64" または "arm64"。

.EXITCODE
  0 正常終了
  2 Tauri conf / テンプレ / OutputDir が不正
  3 アーキが不正
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$TauriConfPath,
  [Parameter(Mandatory = $true)][string]$TemplatePath,
  [Parameter(Mandatory = $true)][string]$OutputDir,
  [Parameter(Mandatory = $true)][string]$Arch
)

$ErrorActionPreference = "Stop"

if ($Arch -ne "x64" -and $Arch -ne "arm64") {
  [Console]::Error.WriteLine("Arch は 'x64' または 'arm64' のみ対応: '$Arch'")
  exit 3
}

if (-not (Test-Path -LiteralPath $TauriConfPath)) {
  [Console]::Error.WriteLine("tauri.conf.json が見つかりません: $TauriConfPath")
  exit 2
}
if (-not (Test-Path -LiteralPath $TemplatePath)) {
  [Console]::Error.WriteLine("AppxManifest.xml テンプレが見つかりません: $TemplatePath")
  exit 2
}
if (-not (Test-Path -LiteralPath $OutputDir)) {
  New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

try {
  $conf = Get-Content -LiteralPath $TauriConfPath -Raw | ConvertFrom-Json
} catch {
  [Console]::Error.WriteLine("tauri.conf.json のパースに失敗: $_")
  exit 2
}
$version = $conf.version
if ([string]::IsNullOrWhiteSpace($version)) {
  [Console]::Error.WriteLine("tauri.conf.json に version フィールドがありません")
  exit 2
}

if ($version -notmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') {
  [Console]::Error.WriteLine('MSIX version requires a stable major.minor.patch version')
  exit 2
}
foreach ($part in $version.Split('.')) {
  $number = 0
  if (-not [int]::TryParse($part, [ref]$number) -or $number -gt 65535) {
    [Console]::Error.WriteLine('MSIX version components must be between 0 and 65535')
    exit 2
  }
}
$version = "$version.0"

$publisher = $env:MSIX_PUBLISHER
if ([string]::IsNullOrWhiteSpace($publisher)) {
  $publisher = 'CN=EasyCursorSwap, O=EasyCursorSwap, C=JP'
}

$content = Get-Content -LiteralPath $TemplatePath -Raw
$content = $content.Replace('${VERSION}', $version)
$content = $content.Replace('${ARCH}', $Arch)
$content = $content.Replace('${PUBLISHER}', $publisher)

$outPath = Join-Path $OutputDir 'AppxManifest.xml'
Set-Content -LiteralPath $outPath -Value $content -Encoding UTF8

Write-Host "wrote $outPath (arch=$Arch version=$version)"
exit 0
