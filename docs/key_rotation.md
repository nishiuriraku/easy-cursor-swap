# 鍵ローテーション PR ガイド

EasyCursorSwap の公式インデックス (`easycursorswap/index` リポジトリ) における
著者公開鍵 (Ed25519) のローテーション手順。

---

## いつローテーションするか

- 秘密鍵の漏洩・誤配布が疑われるとき (**緊急**)
- 鍵を初めて発行してから 12 か月以上経過したとき (定期更新の推奨)
- 端末を紛失したとき / 鍵を保存していたバックアップ媒体を破棄したとき
- アルゴリズム強度に問題が見つかったとき (将来 Ed25519 が破られた場合など)

緊急ローテーションでは、後述の手順 #4 で **historical_keys から該当 key_id を削除する** ことで
過去の署名を即時無効化する選択肢もある。通常更新では historical_keys を残して既存テーマの
互換性を維持する。

---

## 著者レコード (`authors/{github_username}.json`) のスキーマ

```json
{
  "github": "your-github-username",
  "display_name": "Your Display Name",
  "public_key": "BASE64_OF_NEW_ED25519_PUBLIC_KEY",
  "historical_keys": {
    "OLD_KEY_ID_HEX_16CHARS": "BASE64_OF_OLD_ED25519_PUBLIC_KEY",
    "EVEN_OLDER_KEY_ID":      "BASE64_OF_EVEN_OLDER_PUBLIC_KEY"
  }
}
```

| フィールド | 説明 |
|---|---|
| `github` | GitHub ユーザー名 (ファイル名と一致) |
| `display_name` | 表示名 (テーマ作者欄に出る) |
| `public_key` | 現行公開鍵 (32 バイトの Ed25519 を base64 エンコード) |
| `historical_keys` | 過去の `key_id → public_key` マップ (オプション) |

`key_id` は公開鍵 (raw 32 バイト) の SHA-256 を hex で出力した先頭 16 文字。
**必ず小文字 hex** で記述する。

---

## ローテーション手順

### 1. 新しい鍵ペアを生成

EasyCursorSwap アプリで:

```
設定 → 鍵管理 → 「新しい鍵ペアを生成」
```

または `keystore_generate` IPC を直接呼ぶ。生成後、設定画面の `key_id` と
`public_key` (base64) をコピーする。

### 2. 旧鍵の `key_id` を控える

ローテーション PR では旧鍵を `historical_keys` に移すため、
旧鍵の `key_id` (16 文字 hex) と公開鍵 (base64) が必要。

旧 `authors/{user}.json` の `public_key` 値を控えておくこと。
`key_id` は `sha256(base64_decode(public_key))[..16].hex()` で再計算可能。

### 3. `easycursorswap/index` リポジトリで PR を作成

`authors/{your_github_username}.json` を以下のように更新:

**Before:**
```json
{
  "github": "alice",
  "display_name": "Alice",
  "public_key": "OLD_PUBLIC_KEY_B64"
}
```

**After:**
```json
{
  "github": "alice",
  "display_name": "Alice",
  "public_key": "NEW_PUBLIC_KEY_B64",
  "historical_keys": {
    "abc1234567890def": "OLD_PUBLIC_KEY_B64"
  }
}
```

### 4. (緊急時のみ) 旧鍵を即時失効させる

漏洩が確認された鍵は historical_keys に **入れない**。
この場合、過去にその鍵で署名された全テーマが検証失敗となる。
別 PR でそれらのテーマを再署名 (新鍵) して提出し直すこと。

### 5. PR テンプレ

```markdown
## 鍵ローテーション

- 種別: [ ] 定期更新 [ ] 緊急 (漏洩疑い)
- GitHub ユーザー: @your-username
- 旧 key_id: `abc1234567890def`
- 新 key_id: `def0987654321abc`
- historical_keys に旧鍵を残す: [ ] はい [ ] いいえ (緊急時)

## チェックリスト

- [ ] 新鍵で `keystore_info` を呼び `key_id` が一致することを確認
- [ ] 旧 `public_key` を `historical_keys` に正しい `key_id` で登録
- [ ] (緊急時) 旧鍵で署名済みのテーマを再署名して別 PR で提出予定
- [ ] 秘密鍵をコミットしていない (.gitignore で除外されている)
```

---

## CI 自動検証

`scripts/marketplace/validate.mjs` の `verifySignature` ロジックは以下を行う:

1. `entry.author_pubkey_id` が現行 `public_key` の `key_id` と一致 → 現行鍵で検証
2. 一致しない場合は `historical_keys[entry.author_pubkey_id]` を引いて検証
3. どちらにも該当しなければ検証エラー

つまり PR で historical_keys を正しく追記しておけば、過去テーマの署名検証は通り続ける。

---

## トラブルシューティング

| 症状 | 原因 / 対処 |
|---|---|
| `key_id ${id} が著者鍵と一致しません` | 新 `public_key` の `key_id` 計算が違う、または `historical_keys` の key 名が間違っている |
| `Ed25519 公開鍵長が不正` | base64 デコード後 32 バイトでない (改行混入や padding 不足) |
| `署名長が不正` | テーマ署名側の問題。鍵ローテーション時には発生しないはず |

`key_id` の計算は Node で:

```js
import { createHash } from 'node:crypto'
const raw = Buffer.from(pubkeyB64, 'base64')
const keyId = createHash('sha256').update(raw).digest('hex').slice(0, 16)
```
