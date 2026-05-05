# EasyCursorSwap - 次世代マウスカーソル管理ツール

## プロジェクト概要
Windows 専用のカスタムマウスカーソル管理ツール。
Tauri v2 + Nuxt 4 + Rust のハイブリッドアーキテクチャ。

## 技術スタック
- **Frontend:** Nuxt 4 / Vue.js（SPA モード）
- **Framework:** Tauri v2
- **Backend:** Rust（windows-rs, winreg, image, tracing）
- **ビルドターゲット:** Windows 10 22H2+ / Windows 11（x64 / ARM64）

## ディレクトリ構成
```
easy-cursor-swap/
├── app/                    # Nuxt フロントエンド
│   ├── assets/css/         # グローバル CSS
│   ├── components/         # Vue コンポーネント
│   ├── layouts/            # レイアウト
│   └── pages/              # ページ
├── src-tauri/              # Tauri + Rust バックエンド
│   ├── src/
│   │   ├── main.rs         # エントリポイント
│   │   ├── lib.rs          # モジュール定義
│   │   ├── commands.rs     # Tauri IPC コマンド
│   │   ├── config.rs       # 設定管理
│   │   ├── cursor.rs       # .cur バイナリ生成
│   │   ├── darkmode.rs     # ダークモード監視
│   │   ├── errors.rs       # エラー型定義
│   │   ├── registry.rs     # レジストリ操作
│   │   ├── theme.rs        # テーマ管理
│   │   └── tray.rs         # システムトレイ
│   ├── Cargo.toml
│   └── tauri.conf.json
├── nuxt.config.ts
└── package.json
```

## 開発コマンド
```bash
# フロントエンド開発サーバー
cd easy-cursor-swap && npx nuxt dev

# Tauri アプリ全体の開発
cd easy-cursor-swap && npx tauri dev

# Rust のみビルドチェック
cargo check --manifest-path src-tauri/Cargo.toml

# プロダクションビルド
npx tauri build
```

## 重要な設計判断
- **ssr: false の代わりに routeRules を使用:** Nuxt 4.4.4 の IPC バグ回避
- **設定は Rust が Source of Truth:** config.json は Rust 側で管理
- **カーソルファイルは `~/.custom_cursors/` に保存:** アンインストール時の削除を回避
- **レジストリは HKCU のみ操作:** UAC 不要の設計

## コーディング規約
- Rust コードのコメントは日本語
- フロントエンドのコンポーネントは Vue SFC (Composition API + `<script setup>`)
- CSS は Vanilla CSS（Tailwind 不使用）
- `v-html` の使用禁止（XSS 対策）
