# EasyCursorSwap - Tauri Updater 署名鍵管理

## 現在の鍵

| 種別 | 場所 | コミット対象 |
|---|---|---|
| 公開鍵 (base64 minisign) | `src-tauri/signing/easycursorswap.pub` | ✅ YES |
| 秘密鍵 | `src-tauri/signing/easycursorswap` | ❌ NO (`.gitignore` 登録済み) |
| `tauri.conf.json` pubkey フィールド | `src-tauri/tauri.conf.json` | ✅ YES |

> [!WARNING]
> 秘密鍵 (`src-tauri/signing/easycursorswap`) は絶対にリポジトリに push しないこと。
> CI は GitHub Secrets から読み込む。

## GitHub Actions に秘密鍵を登録する手順

1. **秘密鍵の内容を取得**:
   ```pwsh
   Get-Content src-tauri\signing\easycursorswap -Raw
   ```

2. **GitHub の Settings → Secrets → New repository secret** で登録:
   | Secret 名 | 値 |
   |---|---|
   | `TAURI_SIGNING_PRIVATE_KEY` | 秘密鍵ファイルの内容 (改行含む全体) |
   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 生成時に設定したパスフレーズ (なしなら空) |

3. `.github/workflows/release.yml` がこれらの Secret を `env:` で参照して署名を行う。

## 鍵のローテーション手順

1. 新しい鍵を生成:
   ```pwsh
   npx tauri signer generate --ci -w src-tauri/signing/easycursorswap -f
   ```

2. `src-tauri/tauri.conf.json` の `plugins.updater.pubkey` を新しい公開鍵に更新。

3. GitHub Secrets の `TAURI_SIGNING_PRIVATE_KEY` を新しい秘密鍵に差し替え。

4. 旧公開鍵で署名されたインストーラーからのアップデートは自動更新できなくなるため、
   メジャーバージョン変更 (v1 → v2) 時のみローテーションを推奨。

## `tauri.conf.json` の現在の pubkey

```json
"pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDdFOTEzM0YxRjc1OTlDMQpSV1RCbVhVZlB4UHBCd1o5enQvdzdvQ0JzMGV1K3RrS2lCYk1nZTMvOUV1ZGlTbHNnTkhxZVg3dQo="
```

Base64 デコードすると minisign フォーマット:
```
untrusted comment: minisign public key: 7E9133F1F7599C1
RWTB...
```

## コードサイニング (EV/OV 証明書) との関係

Tauri Updater 署名 (minisign) と Windows コードサイニング (Authenticode) は**独立した別物**。

| 署名種別 | 目的 | 状態 (2026-05-21) |
|---|---|---|
| Tauri Updater (minisign) | 更新ファイルの改ざん防止 | ✅ 設定済み (有効) |
| Windows Authenticode | SmartScreen レピュテーション | ⏸️ SignPath Foundation 一次審査保留、再申請準備中 |

Authenticode 証明書の調達戦略と再申請ロードマップは
[`authenticode_signing.md`](authenticode_signing.md) を参照。配布形態と
SmartScreen 緩和策の概要は [`distribution.md`](distribution.md) を参照。
