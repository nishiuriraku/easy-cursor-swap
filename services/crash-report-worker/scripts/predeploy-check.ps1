# EasyCursorSwap Crash Report Worker — predeploy validation gate (PowerShell).
#
# Run **before** `wrangler login` / `wrangler deploy` to confirm the bundle
# type-checks and `wrangler deploy --dry-run` resolves all bindings (KV ids,
# vars, secrets) without actually publishing.
#
# Recorded baseline (2026-05-08, after Turnstile + WAF + Logpush wiring):
#   - tsc --noEmit:    pass
#   - dry-run upload:  8.23 KiB / gzip 2.84 KiB, 0 warnings
#
# Usage (Windows / PowerShell 7+):
#   pwsh ./scripts/predeploy-check.ps1
#
# This script is intentionally read-only against Cloudflare. It never calls
# `wrangler deploy` (without --dry-run), `wrangler login`, or `wrangler secret`.

$ErrorActionPreference = 'Stop'

Set-Location (Join-Path $PSScriptRoot '..')

Write-Host '==> npm install (silent)'
npm install --silent
if ($LASTEXITCODE -ne 0) { throw "npm install failed (exit $LASTEXITCODE)" }

Write-Host '==> tsc --noEmit'
npx tsc --noEmit
if ($LASTEXITCODE -ne 0) { throw "tsc --noEmit failed (exit $LASTEXITCODE)" }

Write-Host '==> wrangler deploy --dry-run --outdir=dist'
npx wrangler deploy --dry-run --outdir=dist
if ($LASTEXITCODE -ne 0) { throw "wrangler dry-run failed (exit $LASTEXITCODE)" }

Write-Host '==> predeploy check passed.'
