# CursorForge

**次世代マウスカーソル管理ツール — Windows 専用**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2022H2%2B-blue)](https://github.com/cursorforge/cursor-forge)
[![Tauri](https://img.shields.io/badge/Tauri-v2-orange)](https://tauri.app)

[English README is here](README.md)

---

CursorForge は Windows 専用のデスクトップアプリです。
`.cursorpack` 形式のカーソルテーマをインポート・作成・切り替えできます。
Windows の全 17 カーソル役割 / 6 DPI サイズに対応し、Ed25519 署名付きテーマ配布と
OS ダーク/ライトモード自動切替をサポートします。

## 主な機能

- **テーマライブラリ** — `.cursorpack` をドラッグ＆ドロップで取り込み。フィルタ・ソート対応
- **ワンクリック適用** — 17 スロット × 6 解像度をスナップショット/ロールバック付きでレジストリへ書き込み
- **クリエイターモード** — PNG/SVG からカーソルテーマを制作し、署名付き `.cursorpack` を出力
- **ダークモード自動切替** — 2 テーマを OS ライト/ダーク状態にペアリング。自動で切り替わる
- **公式インデックス** — Ed25519 検証済みテーマのコミュニティインデックスを閲覧・インストール
- **パニックボタン** — Windows 既定またはインストール前スナップショットにいつでも一発復元
- **トレイ常駐** — バックグラウンドで静かに動作。OS 起動時のサイレント起動にも対応
- **セキュリティ多層防御** — Ed25519 署名、ZIP 爆弾検出、Magic Byte 検証、パストラバーサル防止、SVG サニタイズ、PNG メタデータ除去
- **自動アップデート** — 署名済み Tauri Updater で差分配信。メジャーバージョン跨ぎは手動確認

## 動作環境

| 項目 | 最低要件 |
|---|---|
| OS | Windows 10 22H2 (ビルド 19045) 以降、または Windows 11 |
| アーキテクチャ | x64（ARM64 は計画中） |
| WebView2 | Evergreen ランタイム（Windows 11 は標準搭載、Windows 10 は自動インストール） |
| ディスク容量 | インストーラー約 30 MB、テーマライブラリ標準 ~100 MB |

> **動作対象外:** リモートデスクトップ (RDP)、Citrix、Windows Server、UAC セキュアデスクトップ、ロック画面、マルチユーザー同時セッション。

## インストール

[Releases ページ](https://github.com/cursorforge/cursor-forge/releases) から最新のインストーラーをダウンロードしてください。

| ファイル | 説明 |
|---|---|
| `CursorForge_x64-setup.exe` | NSIS インストーラー（ユーザーインストール、管理者権限不要） |
| `CursorForge_x64_ja-JP.msi` | MSI インストーラー |

どちらも minisign 鍵で署名されています（組み込みアップデーターが検証）。
署名の手動検証方法は [docs/signing.md](docs/signing.md) を参照してください。

> **SmartScreen について:** ダウンロード数が少ない初期段階では「不明な発行元」の警告が出ることがあります。
> **「詳細情報」→「実行」** で続行してください。
> 本アプリは [SignPath.io Foundation](https://about.signpath.io/foundation) の OV 署名で配布されています。

## 開発環境セットアップ

### 前提条件

- [Rust](https://rustup.rs/)（stable、1.77.2 以降）
- [Node.js](https://nodejs.org/) 20 以降
- [WebView2](https://developer.microsoft.com/ja-jp/microsoft-edge/webview2/)（Windows 11 は標準搭載）

### クイックスタート

```bash
git clone https://github.com/cursorforge/cursor-forge.git
cd cursor-forge
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

### ディレクトリ構成

```
cursor-forge/
├── app/                        # Nuxt 4 フロントエンド（SPA モード）
│   ├── assets/css/             # デザイントークン + グローバル CSS
│   ├── components/             # Vue SFC（Composition API + <script setup>）
│   ├── composables/            # 共有リアクティブロジック（useThemes, useAppConfig, …）
│   ├── locales/                # i18n キー: ja.ts / en.ts（完全一致必須）
│   ├── pages/                  # ルートページ（index, creator, marketplace, settings, …）
│   └── types/                  # IPC ペイロードの TypeScript 型定義
├── src-tauri/                  # Tauri + Rust バックエンド
│   ├── src/
│   │   ├── main.rs             # エントリポイント: トレイ / ダークモード監視 / ヘルスチェック
│   │   ├── lib.rs              # モジュール宣言
│   │   ├── commands.rs         # Tauri IPC コマンドハンドラー（約 25 エンドポイント）
│   │   ├── config.rs           # 設定マネージャー（RwLock / スキーママイグレーション / バックアップ）
│   │   ├── cursor.rs           # PNG → .cur バイナリ生成（6 サイズ / ホットスポット）
│   │   ├── registry.rs         # HKCU レジストリ読み書き / SPI_SETCURSORS
│   │   ├── theme.rs            # テーママネージャー（.cursorpack インポート/エクスポート）
│   │   ├── marketplace.rs      # HTTP インデックス取得 / SHA-256 + Ed25519 検証
│   │   ├── keystore.rs         # Ed25519 鍵生成 / DPAPI 暗号化
│   │   ├── health.rs           # 起動失敗カウンタ / ロールバック検出
│   │   └── …                   # darkmode / tray / logging / backup / accessibility / …
│   ├── benches/                # Criterion マイクロベンチマーク
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/                       # アーキテクチャ / セキュリティ / 配布 / 署名ドキュメント
├── scripts/marketplace/        # validate.mjs — インデックス提出の CI 検証スクリプト
└── .github/workflows/          # ci.yml / performance.yml / marketplace-validate.yml
```

## アーキテクチャ

CursorForge は **Rust がシステム状態の唯一の信頼源 (Source of Truth)** となる階層構造です。

```
Vue (UI) ──IPC──▶ Tauri コマンド ──▶ Rust モジュール ──▶ Windows レジストリ / ファイルシステム
```

- フロントエンドは型付き IPC コマンド（`invoke()`）経由でのみ通信します。
- レジストリへの書き込みはすべてトランザクション形式です。適用前にスナップショットを保存し、クラッシュ時は次回起動時に自動ロールバックします。
- カーソルファイルは `%USERPROFILE%\.custom_cursors\` に保存され、アンインストール後も残ります。

詳細は [docs/02_architecture_and_core.md](docs/02_architecture_and_core.md) を参照してください。

## セキュリティモデル

| 層 | 仕組み |
|---|---|
| テーマ完全性 | Ed25519 署名（ed25519-dalek）、key_id = 公開鍵 SHA-256 先頭 16 文字 |
| 秘密鍵保存 | Windows DPAPI（`CryptProtectData`） — ユーザーアカウントに紐付き |
| 鍵エクスポート | XChaCha20-Poly1305 + Argon2id パスフレーズ暗号化（`.cfkey` 形式） |
| ダウンロード安全性 | SHA-256 ハッシュ照合 + 50MB / 200MB / 10MB の三段階サイズ上限 |
| アーカイブ安全性 | パストラバーサル防止 / シンボリックリンク拒否 / ZIP 爆弾検出 |
| 画像安全性 | PNG メタデータ除去（eXIf / iTXt / zTXt）/ SVG サニタイズ |
| 通信 | rustls-tls（OS の TLS スタックに依存しない） |

詳細は [docs/03_security_and_ecosystem.md](docs/03_security_and_ecosystem.md) を参照してください。

## 公式インデックスへのテーマ提出

1. クリエイターモードでカーソルテーマを制作し、署名付き `.cursorpack` をエクスポートします。
2. ファイルを GitHub Release または安定した CDN URL にアップロードします。
3. CursorForge の **インデックス → インデックスに提出** で GitHub ユーザー名とダウンロード URL を入力し、エントリ JSON をプレビューしてから **GitHub PR を開く** をクリックします。
4. アプリが `entries/{id}.json` を事前入力した GitHub ウェブエディタを開きます。
5. PR がマージされると、CI が署名・SHA-256 ハッシュ・VirusTotal スキャンを検証し、公開インデックスに掲載されます。

署名鍵のローテーション手順は [docs/key_rotation.md](docs/key_rotation.md) を参照してください。

## 既知の制限事項（v1.0）

| 制限 | 補足 |
|---|---|
| `.ani` 新規作成不可 | アニメーションカーソルのインポートは可能だが制作はできない |
| ライブプレビューなし | 適用はレジストリへの即時書き込み。プレビューモードは未実装 |
| Undo なし | 適用は意図的に一方向。パニックボタンで復元してください |
| 自動切替はダークモードのみ | 自動切替は OS ダーク/ライト切替専用 |
| UAC セキュアデスクトップ | 昇格ダイアログ表示中は Windows 標準カーソルになる |
| ロック画面 / サインイン画面 | Windows 標準カーソルが表示される |
| マルチユーザーセッション | Windows ユーザーアカウントごとに独立した設定 |
| リモートデスクトップ (RDP) | 未対応。カーソルレンダリングは RDP ホストが制御する |
| ARM64 | 未検証。ARM64 Windows では x64 バイナリのエミュレーション動作 |

## コントリビュート

プルリクエスト歓迎です。提出前に以下を確認してください。

1. 検証ゲートを通過させること: `cargo check` + `cargo test` + `npx tauri build`
2. i18n キーのパリティを維持: `node scripts/check-i18n.mjs` がゼロ終了すること
3. [CLAUDE.md](CLAUDE.md) のコーディング規約に従うこと:
   - Rust コードのコメントは日本語
   - Vue: Composition API + `<script setup>`
   - CSS: Vanilla CSS（Tailwind 不使用）
   - `v-html` 禁止（XSS 対策）

開発フローの詳細は [docs/04_implementation_guide.md](docs/04_implementation_guide.md) を参照してください。

## ライセンス

MIT — [LICENSE](LICENSE) を参照してください。
