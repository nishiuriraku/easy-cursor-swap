# EasyCursorSwap Crash Report Worker

EasyCursorSwap の **オプトイン** クラッシュレポートを匿名 POST で受け、
[`nishiuriraku/easy-cursor-swap-index`](https://github.com/nishiuriraku/easy-cursor-swap-index)
の Issue として転送する Cloudflare Worker。

匿名 GitHub API への直 POST はできないので、本 Worker が PAT を保持し
クライアント側からは PAT を見せないように仲介する設計。

## 動作概要

```
[アプリ panic]
   ↓ %LOCALAPPDATA%\EasyCursorSwap\crash\panic-*.json (PII redact 済み)
   ↓ ユーザーが「送信」をオプトインで実行
   ↓
   POST https://easy-cursor-swap-crash-report.<sub>.workers.dev/crash
        Header: X-App-Token: <ALLOWED_ORIGIN>
        Body: { app_version, os, message, location?, timestamp_utc, signature? }
   ↓
[Worker]
   1. X-App-Token 検証
   2. レート制限 (IP × hour, KV)
   3. payload 検証 + サイズ閾値
   4. signature でデデュープ → 既存 Issue ならコメント追記、無ければ新規作成
   5. JSON で { issue_number, action } を返す
```

## 必要な GitHub PAT

設定 → Developer settings → Fine-grained PAT で以下を発行:

- **Repository access**: `nishiuriraku/easy-cursor-swap-index` のみ
- **Repository permissions**: `Issues: Read and write`
- 有効期限: 1 年など

`wrangler secret put GITHUB_TOKEN` で投入する。

## デプロイ手順

KV 名前空間 (`RATE_LIMIT_KV` / `DEDUP_KV`) は Cloudflare MCP 経由で
作成済みで、`wrangler.toml` に既に ID が反映されています。
あとは認証 → secret 設定 → デプロイの 3 手順:

```bash
cd services/crash-report-worker
npm install

# 1. Cloudflare 認証 (初回のみ)
npx wrangler login

# 2. secret 投入 (CLI が値を対話で要求)
npx wrangler secret put GITHUB_TOKEN     # 下記 fine-grained PAT を貼り付け
npx wrangler secret put ALLOWED_ORIGIN   # 任意のランダム文字列。Tauri 側にも同値を埋める

# 3. デプロイ
npx wrangler deploy
```

デプロイすると `https://easy-cursor-swap-crash-report.<sub>.workers.dev` の
URL が払い出される。アプリ側の `submit_pending_reports` (Phase 7-1) で
このエンドポイントに POST する。

## 動作確認

```bash
curl -X POST \
  -H "X-App-Token: <ALLOWED_ORIGIN>" \
  -H "Content-Type: application/json" \
  -d '{"app_version":"0.1.0","os":"windows","message":"panic from curl test","timestamp_utc":"2026-05-06T00:00:00Z"}' \
  https://easy-cursor-swap-crash-report.<sub>.workers.dev/crash
```

レスポンス:

```json
{ "ok": true, "issue_number": 42, "action": "created" }
```

## レート制限と保護

| 仕組み | 値 |
|---|---|
| IP 単位レート制限 | 5 req/h (KV `rl:<ip>:<hour>`) |
| ペイロード上限 | 8 KB (全体) / 4 KB (message + location) |
| デデュープ TTL | 30 日 (signature 一致なら同じ Issue にコメント) |
| App Token | `X-App-Token` ヘッダ必須 (worker secret と一致) |

## 受け付けない情報 (クライアント側で除外済み)

- ユーザーホームパス (`%USERPROFILE%` などは `~` に置換)
- IP アドレス (Worker は CF-Connecting-IP を rate limit にだけ使用、Issue 本文には載せない)
- 端末固有 ID / ハードウェア情報

詳細は [`src-tauri/src/crash.rs`](../../src-tauri/src/crash.rs) と
[`src-tauri/src/logging.rs`](../../src-tauri/src/logging.rs) の
`redact_path` 実装を参照。
