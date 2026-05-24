# EasyCursorSwap

**次世代マウスカーソル管理ツール — Windows 専用**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2022H2%2B-blue)](https://github.com/nishiuriraku/easy-cursor-swap)
[![Tauri](https://img.shields.io/badge/Tauri-v2-orange)](https://tauri.app)

[English README is here](README.md)

---

EasyCursorSwap は Windows 専用のデスクトップアプリです。
`.cursorpack` 形式のカーソルテーマをインポート・作成・切り替えできます。
Windows の全 17 カーソル役割 / 6 DPI サイズに対応し、Ed25519 署名付きテーマ配布を
サポートします。

## 主な機能

- **テーマライブラリ** — `.cursorpack` をドラッグ＆ドロップで取り込み。フィルタ・ソート対応
- **ワンクリック適用** — 17 スロット × 6 解像度をスナップショット/ロールバック付きでレジストリへ書き込み
- **クリエイターモード** — PNG/SVG からカーソルテーマを制作し、署名付き `.cursorpack` を出力
- **公式インデックス** — Ed25519 検証済みテーマのコミュニティインデックスを閲覧・インストール
- **パニックボタン** — Windows 既定またはインストール前スナップショットにいつでも一発復元
- **カーソルサイズ調整** — 設定画面に 15 段階スライダーを統合し、`HKCU\Control Panel\Cursors\CursorBaseSize` (DWORD 32-256) を更新する。テーマ切替とは独立した OS 全体設定として保持。Windows アクセシビリティ側でサイズが拡大されているとき (ease-of-access pipeline active) はスライダーを `disabled` にし、Windows 設定への deep-link を表示する。Windows 設定 UI 側のスライダー位置と本アプリのスライダー位置は意図的に同期しない
- **トレイ常駐** — バックグラウンドで静かに動作。OS 起動時のサイレント起動にも対応
- **セキュリティ多層防御** — Ed25519 署名、ZIP 爆弾検出、Magic Byte 検証、パストラバーサル防止、SVG サニタイズ、PNG メタデータ除去
- **自動アップデート** — 署名済み Tauri Updater で差分配信。メジャーバージョン跨ぎは手動確認

## 動作環境

| 項目           | 最低要件                                                                     |
| -------------- | ---------------------------------------------------------------------------- |
| OS             | Windows 10 22H2 (ビルド 19045) 以降、または Windows 11                       |
| アーキテクチャ | x64（ARM64 は計画中）                                                        |
| WebView2       | Evergreen ランタイム（Windows 11 は標準搭載、Windows 10 は自動インストール） |
| ディスク容量   | インストーラー約 30 MB、テーマライブラリ標準 ~100 MB                         |

> **動作対象外:** リモートデスクトップ (RDP)、Citrix、Windows Server、UAC セキュアデスクトップ、ロック画面、マルチユーザー同時セッション。

## インストール

[Releases ページ](https://github.com/nishiuriraku/easy-cursor-swap/releases) から最新のインストーラーをダウンロードしてください。

| ファイル                       | 説明                                                        |
| ------------------------------ | ----------------------------------------------------------- |
| `EasyCursorSwap_x64-setup.exe` | NSIS インストーラー（ユーザーインストール、管理者権限不要） |
| `EasyCursorSwap_x64_ja-JP.msi` | MSI インストーラー                                          |

どちらも minisign 鍵で署名されています（組み込みアップデーターが検証）。
署名の手動検証方法は [docs/updater_signing.md](docs/updater_signing.md) を参照してください。

> **SmartScreen について:** 現状このアプリは **Authenticode コード署名を未取得** です。
> OSS 向け署名サービスの [SignPath Foundation](https://signpath.org/) に申請しましたが、
> 外部認知 (GitHub stars / 紹介記事 / 言及) がまだ十分でないため一次審査が保留となり
> (2026-05-21)、認知度が伸びた段階での再申請を予定しています。そのため Windows
> SmartScreen が「不明な発行元」の警告を出すことがあります。
> **「詳細情報」→「実行」** で続行可能です。
> 配布物は GitHub の公開ワークフローでビルドされ、Tauri Updater 用の Ed25519
> (minisign) 署名は付与されています。ソースは MIT ライセンスで全公開、署名ポリシー
> (チーム構成・プライバシー・ビルド再現性・現在の署名状況) は
> [docs/code_signing_policy.md](docs/code_signing_policy.md) を参照してください。

### 自動アップデート

EasyCursorSwap は設定 → 更新 で **自動アップデートが有効** の場合、
アプリ起動時に更新確認を行います。GitHub の rate limit を考慮し、
**24 時間に 1 回まで** に制限されています。

新しいリリースが見つかると Windows トースト通知が表示されます。
メジャーバージョンアップ (例: v1.x → v2.0) は **トースト通知の対象外**
です — 互換性のない変更を確認していただくため、設定 → 更新 から
ユーザーが意識的に DL する経路に誘導しています。

すべての更新パッケージは Ed25519 (minisign) で署名されており、
ビルド時に埋め込まれた公開鍵で検証してからインストールされます。

連続 3 回起動に失敗するとロールバックダイアログが表示され、
**自動で旧版をダウンロードして再インストール** することができます。
ロールバック用のインストーラも同じ方式で署名検証されてから
サイレント起動されます。

## 開発環境セットアップ

### 前提条件

- [Rust](https://rustup.rs/)（stable、1.82 以降。`src-tauri/Cargo.toml` の `rust-version` で固定）
- [Node.js](https://nodejs.org/) 20 以降
- [WebView2](https://developer.microsoft.com/ja-jp/microsoft-edge/webview2/)（Windows 11 は標準搭載）

### クイックスタート

```bash
git clone https://github.com/nishiuriraku/easy-cursor-swap.git
cd easy-cursor-swap
npm install

# 開発モードで起動（Tauri dev ウィンドウ + Nuxt HMR）
npx tauri dev

# Rust の型チェックのみ
cargo check --manifest-path src-tauri/Cargo.toml

# Rust テスト実行
cargo test --manifest-path src-tauri/Cargo.toml

# プロダクションビルド → src-tauri/target/release/bundle/ に .msi / .exe を生成
npx tauri build
```

### Marketplace 自動提出を開発する場合 (任意)

Marketplace 自動提出フローは GitHub OAuth Device Flow を使います。
`npm run tauri:dev` でこの機能を試すには、まず
<https://github.com/settings/applications/new> で GitHub OAuth App を登録し
(コールバック URL は不要)、その Client ID を環境変数に設定してください:

```powershell
$env:EASY_CURSOR_SWAP_GITHUB_OAUTH_CLIENT_ID = "Iv1.xxxxxxxx"
npm run tauri:dev
```

`EASY_CURSOR_SWAP_GITHUB_OAUTH_CLIENT_ID` 未設定でもビルド/起動は通り、自動提出 IPC のみ
「OAuth Client ID 未設定」エラーで失敗します。その場合ユーザーは手動提出
タブ (advanced) に切り替えて従来通り PR を作成できます。

### ディレクトリ構成

```
easy-cursor-swap/
├── app/                        # Nuxt 4 フロントエンド（SPA モード）
│   ├── assets/css/             # デザイントークン + グローバル CSS
│   ├── components/             # Vue SFC（Composition API + <script setup>）
│   ├── composables/            # 共有リアクティブロジック（useThemes, useAppConfig, …）
│   ├── locales/                # i18n キー: ja.ts / en.ts（完全一致必須）
│   ├── pages/                  # ルートページ（index, creator, marketplace, settings, …）
│   └── types/                  # IPC ペイロードの TypeScript 型定義
├── src-tauri/                  # Tauri + Rust バックエンド
│   ├── src/
│   │   ├── main.rs             # エントリポイント: トレイ / ヘルスチェック
│   │   ├── lib.rs              # モジュール宣言（23 モジュール）
│   │   ├── commands/           # Tauri IPC コマンドハンドラー（9 サブモジュール / 52 エンドポイント）
│   │   ├── config.rs           # 設定マネージャー（RwLock / スキーママイグレーション / バックアップ）
│   │   ├── cursor/             # PNG → .cur / .ani パイプライン（6 サイズ / ホットスポット / ANI 入出力）
│   │   ├── registry/           # HKCU レジストリ読み書き / Schemes / SPI_SETCURSORS
│   │   ├── theme/              # テーママネージャー（.cursorpack 入出力 / sanitize）
│   │   ├── bulk_import/        # フォルダ・ファイルの一括取り込み + cursorpack 解析
│   │   ├── marketplace.rs      # HTTP インデックス取得 / SHA-256 + Ed25519 検証
│   │   ├── keystore.rs         # Ed25519 鍵生成 / DPAPI 暗号化 / .cfkey
│   │   ├── health.rs           # 起動失敗カウンタ / ロールバック検出
│   │   └── …                   # tray / logging / backup / accessibility / …
│   ├── benches/                # Criterion マイクロベンチマーク
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/                       # アーキテクチャ / セキュリティ / 配布 / 署名ドキュメント
└── .github/workflows/          # ci.yml / performance.yml / release.yml
```

## アーキテクチャ

EasyCursorSwap は **Rust がシステム状態の唯一の信頼源 (Source of Truth)** となる階層構造です。

```
Vue (UI) ──IPC──▶ Tauri コマンド ──▶ Rust モジュール ──▶ Windows レジストリ / ファイルシステム
```

- フロントエンドは型付き IPC コマンド（`invoke()`）経由でのみ通信します。
- レジストリへの書き込みはすべてトランザクション形式です。適用前にスナップショットを保存し、クラッシュ時は次回起動時に自動ロールバックします。
- カーソルファイルは `%USERPROFILE%\.custom_cursors\` に保存され、アンインストール後も残ります。

詳細は [docs/architecture.md](docs/architecture.md) を参照してください。

## セキュリティモデル

| 層                 | 仕組み                                                              |
| ------------------ | ------------------------------------------------------------------- |
| テーマ完全性       | Ed25519 署名（ed25519-dalek）、key_id = 公開鍵 SHA-256 先頭 16 文字 |
| 秘密鍵保存         | Windows DPAPI（`CryptProtectData`） — ユーザーアカウントに紐付き    |
| 鍵エクスポート     | XChaCha20-Poly1305 + Argon2id パスフレーズ暗号化（`.cfkey` 形式）   |
| ダウンロード安全性 | SHA-256 ハッシュ照合 + 50MB / 200MB / 10MB の三段階サイズ上限       |
| アーカイブ安全性   | パストラバーサル防止 / シンボリックリンク拒否 / ZIP 爆弾検出        |
| 画像安全性         | PNG メタデータ除去（eXIf / iTXt / zTXt）/ SVG サニタイズ            |
| 通信               | rustls-tls（OS の TLS スタックに依存しない）                        |

詳細は [docs/architecture.md#security](docs/architecture.md#security) を参照してください。

## 公式インデックスへのテーマ提出

1. クリエイターモードでカーソルテーマを制作し、署名付き `.cursorpack` をエクスポートします。
2. ファイルを GitHub Release または安定した CDN URL にアップロードします。
3. EasyCursorSwap の **インデックス → インデックスに提出** で GitHub ユーザー名とダウンロード URL を入力し、エントリ JSON をプレビューしてから **GitHub PR を開く** をクリックします。
4. アプリが `entries/{id}.json` を事前入力した GitHub ウェブエディタを開きます。
5. PR がマージされると、CI が署名・SHA-256 ハッシュ・VirusTotal スキャンを検証し、公開インデックスに掲載されます。

署名鍵のローテーション手順は [docs/key_rotation.md](docs/key_rotation.md) を参照してください。

## 既知の制限事項

| 制限                        | 補足                                                          |
| --------------------------- | ------------------------------------------------------------- |
| `.ani` 新規作成不可         | アニメーションカーソルのインポートは可能だが制作はできない    |
| ライブプレビューなし        | 適用はレジストリへの即時書き込み。プレビューモードは未実装    |
| Undo なし                   | 適用は意図的に一方向。パニックボタンで復元してください        |
| UAC セキュアデスクトップ    | 昇格ダイアログ表示中は Windows 標準カーソルになる             |
| ロック画面 / サインイン画面 | Windows 標準カーソルが表示される                              |
| マルチユーザーセッション    | Windows ユーザーアカウントごとに独立した設定                  |
| リモートデスクトップ (RDP)  | 未対応。カーソルレンダリングは RDP ホストが制御する           |
| ARM64                       | 未検証。ARM64 Windows では x64 バイナリのエミュレーション動作 |

## コントリビュート

プルリクエスト歓迎です。詳細なワークフローは **[CONTRIBUTING.md](CONTRIBUTING.md)** を参照してください。要点:

1. 検証ゲートを通過させること: `bash scripts/verify-gate.sh`
2. i18n キーのパリティを維持: `node scripts/check-i18n.mjs` がゼロ終了すること
3. [CLAUDE.md](CLAUDE.md) のコーディング規約に従うこと:
   - Rust コードのコメントは日本語
   - Vue: Composition API + `<script setup>`
   - CSS: Tailwind v4 ユーティリティクラス (`app/assets/css/tailwind.css` 参照)
   - `v-html` 禁止（XSS 対策）

## コミュニティ

- [サポート / 質問](SUPPORT.md) — 質問先の案内
- [行動規範](CODE_OF_CONDUCT.md) — Contributor Covenant 2.1
- [セキュリティポリシー](SECURITY.md) — GitHub Private Vulnerability Reporting で受付
- [Changelog](CHANGELOG.md) — リリースノート (Keep a Changelog 形式)

## ライセンス

MIT — [LICENSE](LICENSE) を参照してください。
