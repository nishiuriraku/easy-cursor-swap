# 🖱️ CursorForge

**次世代マウスカーソル管理ツール** - Windows 専用

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

## ✨ 特徴

- 🎨 **簡単なカーソル管理** - .cursorpack をドラッグ＆ドロップするだけ
- 🔄 **ダークモード自動切替** - テーマ A/B のペアリングで自動対応
- 📐 **高DPI完全対応** - 6サイズ自動生成でピクセルパーフェクト
- 🎯 **全17役割サポート** - Windows の全カーソル種類に対応
- 🔒 **セキュリティ多層防御** - Ed25519署名、Magic Byte検証
- 🚨 **パニックボタン** - いつでも Windows 既定に戻せる安心設計
- 💾 **アンインストール耐性** - カーソルは削除されません

## 🚀 クイックスタート

### 必要なもの
- Windows 10 22H2 以降 / Windows 11
- [Rust](https://rustup.rs/) (最新安定版)
- [Node.js](https://nodejs.org/) 20+
- WebView2 (Windows 11 は標準搭載)

### 開発環境セットアップ

```bash
# リポジトリのクローン
git clone https://github.com/your-username/cursor-forge.git
cd cursor-forge

# 依存関係のインストール
npm install

# 開発サーバー起動
npx tauri dev
```

## 📖 ドキュメント

詳細な仕様は [開発仕様書](docs/SPEC.md) を参照してください。

## ⚠️ 既知の制限事項

- **UAC Secure Desktop** (昇格ダイアログ表示中) では Windows 標準カーソルに戻ります
- **ロックスクリーン / サインイン画面** でも Windows 標準カーソルとなります
- **マルチユーザー環境** ではユーザーごとに独立した設定になります
- **リモートデスクトップ (RDP)** 環境は動作対象外です

## 📜 ライセンス

MIT License - 詳細は [LICENSE](LICENSE) を参照してください。
