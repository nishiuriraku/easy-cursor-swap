# 4. 実装ガイドとステータス

## 4.1 現在の開発フェーズと進捗 (Implementation Plan v2 準拠)
- ✅ **Phase 1〜4:** 基盤、Rustコアロジック、.cur生成、ダークモード自動監視完了。
- ✅ **Phase 5 (5-1〜5-9):** UI 7画面のVueへのHi-Fi移植、JSXからVue SFCへの変換、全IPCバインド完了。
- ✅ **Phase 6:** .cursorpack 解凍・生成、セキュリティ多層防御（Zip爆弾・トラバーサル・サニタイズ）、Ed25519署名基盤完了。
- ✅ **Phase 9 (一部):** Marketplaceのクライアント側取得、HTTPS/SHA-256/署名検証、ZIP安全展開完了。
- 🔲 **残タスク (優先度高):**
  - i18nの全画面への適用（`t()`置換）。
  - Win32 COM Toast通知の実装（Phase 7）。
  - 秘密鍵のパスフレーズ付きエクスポート/インポート実装 (Argon2 + ChaCha20Poly1305)。
  - パフォーマンス測定ベンチ（criterion導入）、多重起動防止（Named Mutex）。
  - クリエイターモードの残りタブ（プレビュー・公開）の実装。
  - RDP等 動作環境外の検出と警告ダイアログ。
  - MSIX化、`runFullTrust` 等の配布周り。

## 4.2 デザインシステム・コンポーネント構成
`design/` ディレクトリのReact JSXプロトタイプをVue 3 (Nuxt 4) に移植済みです。
- **CSS:** `app/assets/css/global.css` (デザイントークン、Win11風ウィンドウクロム、Glassmorphism)
- **共有コンポーネント:** `AppTitlebar.vue`, `AppSidebar.vue`, `CursorMatrix.vue`, `ThemeCard.vue`
- **アイコン:** `UiIcon.vue` / `CursorIcon.vue` (render functionで安全にSVGを展開し `v-html` 脆弱性を回避)
- **ページ群:** `index.vue` (ライブラリ), `creator.vue`, `marketplace.vue`, `settings.vue`, `appearance.vue`

## 4.3 国際化 (i18n)
- `app/locales/ja.ts`, `en.ts` に文言キーを定義 (`as const`で型保証)。
- `useI18n()` composable を経由して `t(key, params)` で文字列展開。
- OSロケール (`navigator.language`) に応じた自動判定と、設定画面からの上書き機能。

## 4.4 IPC エンドポイント一覧 (全21エンドポイント)
フロントエンド・バックエンド間の通信用。Tauriコマンドとして `commands.rs` に登録済み。

1. **取得・適用:** `get_cursor_roles`, `get_current_cursors`, `get_themes`, `apply_theme` (適用時に `active_theme_id` 保存)
2. **パッケージ:** `inspect_cursorpack`, `import_cursorpack`, `export_cursorpack`, `export_cursorpack_streamed`, `cancel_build`
3. **バックアップ:** `export_profile`, `import_profile`
4. **鍵管理:** `keystore_info`, `keystore_generate`, `keystore_delete`
5. **インデックス:** `marketplace_fetch_index`, `marketplace_install`
6. **リセット:** `reset_to_default`, `reset_to_initial`
7. **設定・状態:** `get_dark_mode_status`, `get_config`, `update_config`, `get_app_info`

## 4.5 既知の問題・制限事項
- Nuxt 4.4.4 では `ssr: false` がIPCエラーを引き起こすため、`routeRules` で回避中。
- `npm run dev` 実行時、Powershell環境で `Set-Location` が必要なケースあり。
- `zip` クレート v2.6.x は yanked されているため、バージョン指定に注意すること。

## 4.6 コントリビューター（AI / 人間）向け開発ルール
1. **Source of Truth の厳守:** アプリの基本状態はRustの `config.json` が保持します。Vue側の状態は必ずIPCでRustと同期し、ローカルだけで完結させないでください。
2. **UAC (管理者権限) の排除:** `HKLM` やシステム全体に影響する操作は禁止です。レジストリ操作は `HKCU` のみを使用してください。
3. **パスと入力のサニタイズ:** 外部ファイルの解凍や読み込みには必ず `sanitize_archive_path` ヘルパーとサイズ検証を通してください。
4. **PII情報のログ除外:** Rustで `tracing!` を記述する際、パスやハッシュの生データは `logging::redact_path` や `logging::short_hash` で必ず保護してください。
5. **メモリ最適化:** Nuxt側での不要なリスナー放置を避け、トレイ格納時にWebViewエンジンがDropされてもシステムが破綻しない状態管理を行ってください。
