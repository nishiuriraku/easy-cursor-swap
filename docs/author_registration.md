# 新規著者公開鍵の登録ガイド

EasyCursorSwap の公式インデックス (`nishiuriraku/easy-cursor-swap-index` リポジトリ) に
**初めて自分のテーマを提出する著者** 向けの公開鍵登録手順。

すでに登録済みで鍵を更新したい場合は [鍵ローテーション PR ガイド](./key_rotation.md) を参照。

---

## 全体像

```
[アプリで鍵生成]                [公式インデックスへ PR]
   ┌─────────────────┐           ┌──────────────────────────────┐
   │ Settings →     │           │ authors/{your-github}.json    │
   │ Security &     │  →→→→→→   │   { "public_key": "...",      │
   │ Keys → 鍵生成  │           │     "display_name": "..." }   │
   └─────────────────┘           └──────────────────────────────┘
            ↓                                  ↓
   秘密鍵は DPAPI 暗号化で                公開鍵のみ Git 管理。
   ローカルに保管                          PR レビュー → マージで有効化
```

---

## 前提条件

| 項目 | 必須? | 補足 |
|---|---|---|
| GitHub アカウント | ✅ | `authors/<github>.json` のファイル名に使用 |
| EasyCursorSwap v0.1 以上 | ✅ | 鍵生成 IPC が必要 |
| `.cfkey` バックアップ | 推奨 | 端末故障時の復旧用 (Settings → Keys → エクスポート) |

---

## 手順

### 1. アプリ内で鍵ペアを生成

1. EasyCursorSwap を起動
2. **設定 (⚙) → Security & Keys** セクションを開く
3. **「鍵ペアを生成」** ボタンをクリック
4. 表示される `key_id` (16 文字 hex) と `public_key` (Base64) をコピー

> [!IMPORTANT]
> 生成済みの場合は **再生成すると過去テーマがすべて検証失敗になる**。
> 通常は最初の 1 回だけ生成し、以降は同じ鍵を使い続ける。

### 2. 秘密鍵をバックアップ (強く推奨)

1. **設定 → Security & Keys → 「.cfkey をエクスポート」**
2. 8 文字以上のパスフレーズを設定
3. 出力された `.cfkey` ファイルを **オフライン媒体** (USB / 暗号化ストレージ) に保管

> [!WARNING]
> 秘密鍵を失うと過去テーマの追加更新ができなくなる。
> パスフレーズも忘れないよう、信頼できるパスワードマネージャーへ。

### 3. アプリから「公式インデックスに提出」を起動

1. ライブラリで提出したいテーマを選択
2. **「公式インデックスに提出」** ボタン (テーマ詳細パネル内)
3. ブラウザで [nishiuriraku/easy-cursor-swap-index](https://github.com/nishiuriraku/easy-cursor-swap-index) のファイル新規作成 URL が開く
   - URL パラメータでテーマメタデータが事前埋めされる

### 4. `authors/{your-github}.json` を作成

GitHub の Web エディタで `authors/<your-github-username>.json` を新規作成し、
以下を貼り付け:

```json
{
  "github": "your-github-username",
  "display_name": "Your Display Name",
  "public_key": "BASE64_OF_YOUR_ED25519_PUBLIC_KEY"
}
```

| フィールド | 値 |
|---|---|
| `github` | あなたの GitHub ユーザー名 (ファイル名と完全一致) |
| `display_name` | テーマ作者欄に表示される名前 (任意の表示名) |
| `public_key` | 手順 1 でコピーした Base64 公開鍵 |

> [!NOTE]
> `historical_keys` フィールドは初回登録では **不要**。
> 後で鍵をローテーションするときに追加する (key_rotation.md 参照)。

### 5. テーマファイルもアップロード

同じ PR 内で `themes/<your-theme-uuid>.cursorpack` をアップロードし、
`index.json` にエントリを追記:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": { "ja": "テーマ名", "en": "Theme Name" },
  "author_github": "your-github-username",
  "key_id": "abc123def456789a",
  "sha256": "...",
  "version": "1.0.0",
  "size_bytes": 12345,
  "tags": ["minimal"]
}
```

### 6. PR を作成

PR タイトル例: `Add author: your-github-username + initial theme: テーマ名`

PR 本文に以下を記載:

```markdown
## 新規著者登録

- GitHub: @your-github-username
- key_id: abc123def456789a
- public_key (Base64): ...

## 提出テーマ

- ID: 550e8400-e29b-41d4-a716-446655440000
- Name: テーマ名 (Theme Name)
- バージョン: 1.0.0
- ライセンス: MIT (または該当ライセンス)
- 動作確認: Windows 11 (24H2) で確認済み
```

### 7. CI 検証を待つ

PR を作成すると `marketplace-validate.yml` ワークフローが自動起動:

- ✅ JSON スキーマ検証
- ✅ SHA-256 整合性チェック
- ✅ Ed25519 署名検証 (公開鍵レコードを参照)
- ✅ key_id 一致確認
- ✅ サイズ閾値 (50MB)
- ✅ VirusTotal スキャン (該当ハッシュ)

すべて通過するとレビュアーが目視確認 → マージで公開反映。

---

## CI 検証で失敗した場合

| エラー | 原因 | 対処 |
|---|---|---|
| `key_id mismatch` | アプリ表示の key_id と JSON が不一致 | 設定画面で再度コピーして JSON を修正 |
| `signature verification failed` | テーマファイルが鍵生成後に手動編集された | アプリで再エクスポート |
| `sha256 mismatch` | アップロードファイルが破損 / 別バージョン | 再ビルド + 再アップロード |
| `VirusTotal positive` | 含まれるファイルが誤検知 / 真陽性 | コメントで誤検知の根拠を提示、または該当ファイル除外 |

---

## レビュー観点 (メンテナ向け)

レビュアーは以下を確認する:

1. `authors/<github>.json` のファイル名と内部 `github` フィールドが一致するか
2. `public_key` が 44 文字の Base64 (= 32 バイト Ed25519) として正しくデコードできるか
3. 同名既存ファイルを上書きしていないか (上書きは [key_rotation.md](./key_rotation.md) の手順で別 PR にする)
4. 提出テーマのカバレッジ (17 役割中 Arrow が必須)
5. `name` / `description` に i18n フィールド (ja / en) が両方あるか
6. ライセンス・著作権の問題が無いか (一次創作 or 適切な再配布許諾)

---

## 関連ドキュメント

- [鍵ローテーション PR ガイド](./key_rotation.md) - 既存著者の鍵更新
- [署名仕様](./signing.md) - Ed25519 / DPAPI / .cfkey フォーマット
- [配布ドキュメント](./distribution.md) - MSI / MSIX / Microsoft Store
