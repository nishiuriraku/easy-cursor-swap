#!/usr/bin/env bash
# EasyCursorSwap Crash Report Worker — predeploy validation gate.
#
# Run **before** `wrangler login` / `wrangler deploy` to confirm the bundle
# type-checks and `wrangler deploy --dry-run` resolves all bindings (KV ids,
# vars, secrets) without actually publishing.
#
# Recorded baseline (2026-05-08, after Turnstile + WAF + Logpush wiring):
#   - tsc --noEmit:    pass
#   - dry-run upload:  8.23 KiB / gzip 2.84 KiB, 0 warnings
#
# Usage:
#   bash scripts/predeploy-check.sh
#
# This script is intentionally read-only against Cloudflare. It never calls
# `wrangler deploy` (without --dry-run), `wrangler login`, or `wrangler secret`.

set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> npm install (silent)"
npm install --silent

echo "==> tsc --noEmit"
npx tsc --noEmit

echo "==> wrangler deploy --dry-run --outdir=dist"
npx wrangler deploy --dry-run --outdir=dist

echo "==> predeploy check passed."
