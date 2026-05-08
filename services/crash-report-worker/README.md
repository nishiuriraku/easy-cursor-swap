# EasyCursorSwap Crash Report Worker

EasyCursorSwap の **オプトイン** クラッシュレポートを匿名 POST で受け、
[`nishiuriraku/easy-cursor-swap`](https://github.com/nishiuriraku/easy-cursor-swap)
の Issue として転送する Cloudflare Worker。

> 送信先はテーマライブラリ (`easy-cursor-swap-index`) ではなく、
> アプリ本体リポジトリ (`easy-cursor-swap`) の Issues。
> クラッシュは「アプリのバグ」なので本体側で追跡する。

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
        Header: X-Turnstile-Token: <token>     # TURNSTILE_SECRET 設定時のみ必須
        Body: { app_version, os, message, location?, timestamp_utc, signature? }
   ↓
[Worker]
   1. X-App-Token 検証
   2. Turnstile 検証 (secret 設定時のみ)
   3. レート制限 (IP × hour, KV)
   4. payload 検証 + サイズ閾値
   5. signature でデデュープ → 既存 Issue ならコメント追記、無ければ新規作成
   6. JSON で { issue_number, action } を返す
```

## 必要な GitHub PAT

設定 → Developer settings → Fine-grained PAT で以下を発行:

- **Repository access**: `nishiuriraku/easy-cursor-swap` のみ
- **Repository permissions**: `Issues: Read and write`
- 有効期限: 1 年など

`wrangler secret put GITHUB_TOKEN` で投入する。

## デプロイ手順

KV 名前空間 (`RATE_LIMIT_KV` / `DEDUP_KV`) は Cloudflare MCP 経由で
作成済みで、`wrangler.toml` に既に ID が反映されています。
事前検証 → 認証 → secret 設定 → デプロイの順で実行:

```bash
cd services/crash-report-worker

# 0. 事前検証 (ローカル / CI でも実行可能、Cloudflare には何も送らない)
#    - npm install (silent)
#    - npx tsc --noEmit
#    - npx wrangler deploy --dry-run --outdir=dist
#    バンドルサイズ目安: 8.23 KiB / gzip 2.84 KiB (2026-05-08 時点)
bash scripts/predeploy-check.sh
# Windows (PowerShell 7+) の場合:
#   pwsh ./scripts/predeploy-check.ps1

# 1. Cloudflare 認証 (初回のみ)
npx wrangler login

# 2. 必須 secret
npx wrangler secret put GITHUB_TOKEN     # 下記 fine-grained PAT を貼り付け
npx wrangler secret put ALLOWED_ORIGIN   # 任意のランダム文字列。Tauri 側にも同値を埋める

# 2'. 任意 secret (public 化前に強く推奨)
npx wrangler secret put TURNSTILE_SECRET # Cloudflare Turnstile の secret key

# 3. デプロイ
npx wrangler deploy
```

> `predeploy-check.{sh,ps1}` は `wrangler deploy --dry-run` までしか呼ばないため、
> 認証情報も実 secret も不要。CI でも回せる。
> 実 `wrangler deploy` / `wrangler secret put` は本番投入時のみ手動で実行する。

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
| Turnstile | `TURNSTILE_SECRET` 設定時、`X-Turnstile-Token` ヘッダ必須 |

## Cloudflare Turnstile (推奨: public 化前に有効化)

`X-App-Token` はバイナリ埋め込みのため時間の問題で割れる。Turnstile を
追加で噛ませることで bot 系 spam を一段で弾ける。

1. Cloudflare ダッシュボード → Turnstile → **Site の追加**
   - Widget mode: **Invisible** または **Managed** (UX を優先するなら Invisible)
   - Domain: アプリ webview のオリジン (Tauri は `tauri://localhost` を許可)
2. 発行された Site key (公開) と Secret key (非公開) のうち、
   secret 側を Worker に投入:

   ```bash
   npx wrangler secret put TURNSTILE_SECRET
   ```

3. Tauri 側 (Vue) で送信前に Turnstile widget を表示し、
   コールバックで取得したトークンを `X-Turnstile-Token` ヘッダに乗せて POST。
   実装は `app/composables/useCrashReporter.ts` に集約予定 (Phase 7-1 残)。

未設定 (`TURNSTILE_SECRET` 空) の場合、本 Worker は Turnstile 検証を
完全にスキップする。段階的にロールアウトする際は:

- 旧クライアント: `X-Turnstile-Token` を送らない
- Worker: secret 未設定 → 検証スキップ
- 新クライアント版が普及してから secret を投入する

の順で互換性を維持できる。

## Cloudflare WAF Custom Rule (Free 枠)

`/crash` エンドポイントに対する異常リクエストを WAF レイヤーで弾く。
Worker 内のレート制限より早い段階で reject できるためコスト削減に有効。

ダッシュボード → Security → WAF → **Custom rules** で以下を作成:

| 設定 | 値 |
|---|---|
| Rule name | `crash-endpoint-protect` |
| Expression | `(http.request.uri.path eq "/crash") and (cf.bot_management.score lt 30 or cf.threat_score gt 10)` |
| Action | Block |

> Free プランでは `cf.bot_management.score` は使えないため、
> 代わりに `(ip.src.country in {"XX"})` などで絞るか、
> 後述の Rate limiting rule を併用する。

### Rate limiting rule (Free 枠 1 ルール無料)

ダッシュボード → Security → WAF → **Rate limiting rules**:

| 設定 | 値 |
|---|---|
| Rule name | `crash-endpoint-rate` |
| If incoming requests match | `(http.request.uri.path eq "/crash")` |
| When rate exceeds | `30 requests per 1 minute` (IP 単位) |
| Action | Block (10 分) |

Worker 内の 5 req/h より緩いが、明らかな flooding (1 分 30 req) を
即座に切れる。Worker の KV 書き込みコストも節約できる。

## Logpush (任意)

`/crash` への異常アクセスを長期保管する。Free プランでは Logpush は
使えないため、必要なら Workers Trace Events Logpush (有料) を検討。
代替として **Tail logs** をローカルでストリーム監視する:

```bash
npx wrangler tail --format=json | tee crash-worker-tail.jsonl
```

オンコール時のみ起動 → ファイル保管 → 後で grep する運用。

## プライバシーポリシー文言 (同意ダイアログ草稿)

Tauri 側のクラッシュ送信オプトインダイアログに以下相当の文言を載せる
(`app/locales/ja.ts` / `app/locales/en.ts` に key 追加予定):

> 送信されるクラッシュレポートには、Panic メッセージとアプリのバージョン、
> OS バージョン、UTC タイムスタンプ、SHA-256 短縮シグネチャのみが含まれます。
> ユーザー名・端末固有 ID は送信前にクライアント側で除去されます
> (`redact_path` 処理)。
>
> 送信時、Cloudflare Worker (米国/EU リージョン) のアクセスログに
> 送信元 IP アドレスが **最大 30 日間** 保管されます。これは Cloudflare の
> 仕様によるもので、本アプリ側では参照・利用しません。
> (Cloudflare のプライバシーポリシー: <https://www.cloudflare.com/privacypolicy/>)
>
> GDPR / 個人情報保護法に基づくデータ削除請求は、
> 当該シグネチャを添えて Issue にコメントしてください。

## 受け付けない情報 (クライアント側で除外済み)

- ユーザーホームパス (`%USERPROFILE%` などは `~` に置換)
- IP アドレス (Worker は CF-Connecting-IP を rate limit と Turnstile remoteip
  にだけ使用、Issue 本文には載せない)
- 端末固有 ID / ハードウェア情報

詳細は [`src-tauri/src/crash.rs`](../../src-tauri/src/crash.rs) と
[`src-tauri/src/logging.rs`](../../src-tauri/src/logging.rs) の
`redact_path` 実装を参照。

## ALLOWED_ORIGIN ローテーション (任意)

`X-App-Token` は理論上いずれリバースエンジニアリングで露出する。
リリースごとに新トークンに切り替え、旧トークンを並列許可する仕組みを
入れる場合、Worker 側に複数値受け入れの拡張が必要になる:

```ts
// 例: ALLOWED_ORIGINS を改行区切りで複数受ける
const allowed = (env.ALLOWED_ORIGINS ?? env.ALLOWED_ORIGIN).split('\n')
if (!allowed.includes(appToken)) return cors(json({ error: 'unauthorized' }, 401))
```

実装が必要になったら本 README を更新する。
