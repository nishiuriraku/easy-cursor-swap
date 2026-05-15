# EasyCursorSwap ファイル機能インベントリ

> 最終更新: 2026-05-13
>
> `src-tauri/src/` と `app/` の **生きたファイル索引**。何がどこにあるかを 1 ファイル単位で把握するための辞書。
> 俯瞰の見取り図は [`architecture.md`](./architecture.md) を参照。
> 構造変更 (ファイル新設・削除・モジュール分割) があったら本書も更新する。

---

## 1. Rust バックエンド (`src-tauri/src/`)

### 1-1. エントリポイント / IPC ハブ

| ファイル | 機能 |
|---|---|
| [main.rs](../src-tauri/src/main.rs) | Tauri アプリのエントリ。tracing 初期化、`StartupCheck::begin()`、AppUserModelID 登録、ConfigManager 初期化、孤児カーソル復旧、pending snapshot リカバリ、`tauri::Builder` 構築 (single-instance プラグイン / 各種 plugin / setup でトレイ・ホットキー) |
| [lib.rs](../src-tauri/src/lib.rs) | 20 モジュールの `pub mod` 宣言 |
| [commands/mod.rs](../src-tauri/src/commands/mod.rs) | 全 Tauri コマンドのハンドラ登録 (`get_command_handlers()` が 61 IPC を `tauri::generate_handler!` に渡す) |
| [errors.rs](../src-tauri/src/errors.rs) | `AppError` (`thiserror`、`Serialize` 派生で IPC 経由 throw 対応) |

### 1-2. IPC コマンド実体 (10 サブモジュール / 61 個)

| ファイル | 主な IPC |
|---|---|
| [commands/cursor_build/](../src-tauri/src/commands/cursor_build/) | `mod` (公開 API + 共通ステート) / `build` (`export_cursorpack`) / `stream` (`export_cursorpack_streamed`、進捗イベント `build-progress`) / `cancel` (`cancel_build`) / `sign` (Ed25519 署名埋込) / `dto` (DTO 定義) |
| [commands/cursor_io.rs](../src-tauri/src/commands/cursor_io.rs) | `import_cursor_file` (.cur/.ico → PNG) / `inspect_ani_file` (RIFF 解析・プレビュー) / `take_pending_cursorpack` (起動時 argv からの引き継ぎ) |
| [commands/ani_export.rs](../src-tauri/src/commands/ani_export.rs) | `export_ani_with_hotspot` (.ani 書き出し + hotspot 埋込) |
| [commands/theme.rs](../src-tauri/src/commands/theme.rs) | `get_cursor_roles` / `get_current_cursors` / `get_themes` / `get_theme_previews` / `get_theme_role_previews` / `apply_theme` / `set_theme_favorite` / `clear_cursor_cache` / `inspect_cursorpack` / `import_cursorpack` / `delete_theme` / `duplicate_theme` / `repackage_theme` |
| [commands/system.rs](../src-tauri/src/commands/system.rs) | `reset_to_default` / `reset_to_initial` / `get_environment_report` / `get_config` / `update_config` / `get_autostart_status` / `get_app_info` / `list_config_backups` / `restore_config_backup` / `open_url` / `open_log_folder` / `get_accessibility_conflicts` / `check_update_is_major_jump` / `list_crash_reports` / `clear_crash_reports` / `submit_crash_reports` |
| [commands/keystore.rs](../src-tauri/src/commands/keystore.rs) | `keystore_info` / `keystore_generate` / `keystore_delete` / `keystore_export` / `keystore_import` |
| [commands/marketplace.rs](../src-tauri/src/commands/marketplace.rs) | `marketplace_fetch_index` / `marketplace_install` / `marketplace_fetch_preview` |
| [commands/marketplace_submit.rs](../src-tauri/src/commands/marketplace_submit.rs) | `start_device_flow` / `complete_device_flow` / `cancel_device_flow` / `submit_theme_auto` / `revoke_github_link` |
| [commands/profile.rs](../src-tauri/src/commands/profile.rs) | `export_profile` / `import_profile` |
| [commands/windows_scheme.rs](../src-tauri/src/commands/windows_scheme.rs) | `list_windows_schemes` / `apply_windows_scheme` / `get_windows_scheme_previews` / `get_windows_scheme_role_previews` / `export_windows_scheme_as_cursorpack` |
| [bulk_import/](../src-tauri/src/bulk_import/) | `bulk_resolve_assets` / `cancel_bulk_import` / `parse_cursorpack_for_creator` (実体は `bulk_import` モジュール側、`tauri::command` 属性付きの関数を `commands/mod.rs` から再エクスポート) |

### 1-3. ドメイン / 機能モジュール

| ファイル | 機能 |
|---|---|
| [config.rs](../src-tauri/src/config.rs) | `AppConfig` の RwLock + schema_version + `config.bak.v{N}.json` 退避 + `config.corrupt.{epoch}.json` |
| [registry/mod.rs](../src-tauri/src/registry/mod.rs) | `HKCU\Control Panel\Cursors` 読み書き、`SPI_SETCURSORS` / `SPI_SETCURSORSHADOW`、トランザクション + `_pending_apply.snapshot`、`save_initial_snapshot` / `check_pending_snapshot` / `reset_to_windows_default` |
| [registry/roles.rs](../src-tauri/src/registry/roles.rs) | 17 役割の絶対パス書き込みロジック、`compute_apply_values` 純粋関数 |
| [registry/scheme.rs](../src-tauri/src/registry/scheme.rs) | `Schemes` への REG_EXPAND_SZ 登録、`build_scheme_value` / `sanitize_scheme_name` |
| [registry/env.rs](../src-tauri/src/registry/env.rs) | レジストリ環境変数操作補助 |
| [cursor/cur_build.rs](../src-tauri/src/cursor/cur_build.rs) | PNG → .cur マルチ解像度 6 サイズパッキング、ホットスポット書込み |
| [cursor/image.rs](../src-tauri/src/cursor/image.rs) | Lanczos / Nearest リサイズ + ドット絵自動判定 + `RESIZE_CACHE` (64 エントリ FIFO) + `strip_png_metadata` |
| [cursor/ani.rs](../src-tauri/src/cursor/ani.rs) | ANI (RIFF/ACON) パーサー + `ParsedAni::total_duration_ms` (検査専用) |
| [cursor/ani_write.rs](../src-tauri/src/cursor/ani_write.rs) | ANI 書き出し (.cur frame の連結 + RIFF header + hotspot 埋込) |
| [cursor/ico_cur.rs](../src-tauri/src/cursor/ico_cur.rs) | .ico/.cur 解析 (ICONDIRENTRY + PNG/BMP DIB) + `pick_largest_as_png` |
| [cursor_watcher.rs](../src-tauri/src/cursor_watcher.rs) | `WM_SETTINGCHANGE` 購読 (不可視ウィンドウ) → `cursor-changed` イベント |
| [theme/mod.rs](../src-tauri/src/theme/mod.rs) | `ThemeManager`：`.cursorpack` 入出力、`apply_theme`、`cleanup_orphan_references`、Ed25519 署名埋込 |
| [theme/sanitize.rs](../src-tauri/src/theme/sanitize.rs) | `sanitize_archive_path` + Zip 爆弾対策 (50/200/10 MB 三段階) + `S_IFLNK` 拒否 |
| [theme/types.rs](../src-tauri/src/theme/types.rs) | `ThemeMeta` / `ThemeSummary` / `LocalizedString` 等 |
| [bulk_import/mod.rs](../src-tauri/src/bulk_import/mod.rs) | `CancelRegistry` + 公開 API + 進捗イベント |
| [bulk_import/assets.rs](../src-tauri/src/bulk_import/assets.rs) | 複数ファイル/フォルダ走査 → `ResolvedAsset` 変換、ファジーマッチ、リサンプル並列化 |
| [bulk_import/cursorpack.rs](../src-tauri/src/bulk_import/cursorpack.rs) | `.cursorpack` 読込 (ライブラリ非依存、creator 直挿入用) |
| [backup.rs](../src-tauri/src/backup.rs) | `.cursorprofile` Zip 入出力、`ProfileEnvelope`、merge/overwrite |
| [marketplace.rs](../src-tauri/src/marketplace.rs) | `MarketplaceClient`：reqwest(rustls) + SHA-256 + Ed25519 + `historical_keys` ローテーション + 50MB ガード。`fetch_preview`：URL スキーム/ホスト + ロール名バリデーション + 500KB 上限でプレビュー PNG を取得 |
| [keystore.rs](../src-tauri/src/keystore.rs) | DPAPI 暗号化保存、`generate`/`sign`/`verify`、XChaCha20-Poly1305 + Argon2id `.cfkey` 入出力。`save/load/delete_github_oauth_token` で GitHub OAuth トークンも DPAPI 管理 |
| [github/mod.rs](../src-tauri/src/github/mod.rs) | `github` モジュール公開 API と re-export |
| [github/types.rs](../src-tauri/src/github/types.rs) | Device Flow / PR 作成で使う Rust 型 (`DeviceFlowResponse`, `GithubPrResult` 等) |
| [github/device_flow.rs](../src-tauri/src/github/device_flow.rs) | GitHub OAuth Device Flow 実装。`start` → ポーリング → トークン取得・DPAPI 保存。scope は `public_repo` 限定 |
| [github/client.rs](../src-tauri/src/github/client.rs) | GitHub REST API クライアント。PR 作成 / ブランチ操作 / `.cursorpack` アップロード。`client_id` は `option_env!("GITHUB_OAUTH_CLIENT_ID")` で注入 |
| [health.rs](../src-tauri/src/health.rs) | `startup.json` の `pending_failures`、3 回連続失敗検知、バージョン変更で自動リセット |
| [crash.rs](../src-tauri/src/crash.rs) | `install_panic_hook`、`%LOCALAPPDATA%\...\crash\panic-{epoch}.json`、`prune_old_reports`、`general.crash_reporting` 同意、送信ペイロード生成 |
| [tray.rs](../src-tauri/src/tray.rs) | システムトレイ + `show_or_recreate_main_window` (WebView 破棄/再生成) |
| [hotkey.rs](../src-tauri/src/hotkey.rs) | `RegisterHotKey` で `Ctrl+Alt+Shift+R` → `panic-hotkey` イベント |
| [autostart.rs](../src-tauri/src/autostart.rs) | `HKCU\...\Run` 登録、MSIX 検出時は no-op で `startupTask` に委譲 |
| [appusermodel.rs](../src-tauri/src/appusermodel.rs) | `SetCurrentProcessExplicitAppUserModelID("dev.easycursorswap.app")` |
| [accessibility.rs](../src-tauri/src/accessibility.rs) | `CursorIndicator` / `ContrastScheme` / `CursorBaseSize` 競合検出 |
| [environment.rs](../src-tauri/src/environment.rs) | RDP / Citrix / Server 検出 (`SM_REMOTESESSION` + `InstallationType`) |
| [logging.rs](../src-tauri/src/logging.rs) | `tracing-appender` 日次ローテ + 14 日 + 100MB 上限 + `redact_path` / `short_hash` |

> 多重起動防止は `tauri_plugin_single_instance` プラグインに集約済 (旧 `single_instance.rs` モジュールは削除)。

### 1-4. ベンチ / テスト

| ファイル | 機能 |
|---|---|
| [benches/cursor_build.rs](../src-tauri/benches/cursor_build.rs) | Lanczos / Nearest + 6 サイズ .cur ビルド計測 (cold / warm パス) |
| [benches/startup.rs](../src-tauri/benches/startup.rs) | 起動時間 / 常駐メモリ目標値検証 |

---

## 2. フロントエンド (`app/`)

### 2-1. ページ (5 画面)

| ファイル | 役割 |
|---|---|
| [pages/index.vue](../app/pages/index.vue) | テーマライブラリ (grid/list/タグフィルタ/インポート/D&D) |
| [pages/index.helpers.ts](../app/pages/index.helpers.ts) | Library ページの IPC → Card マッピング (`IpcThemeSummary` 型 + `mapLocalSummaryToCard`) |
| [pages/creator.vue](../app/pages/creator.vue) | クリエイターモード (3 カラム + バルクインポート + .cursorpack 出力) |
| [pages/marketplace.vue](../app/pages/marketplace.vue) | 公式インデックス (フィルタ/検索/Featured/PR 提出) |
| [pages/marketplace.helpers.ts](../app/pages/marketplace.helpers.ts) | Marketplace ページの `filteredGrid` 計算 (純関数 `computeFilteredGrid`) |
| [pages/settings.vue](../app/pages/settings.vue) | 設定 8 セクション (一般/起動/ライブラリ/セキュリティ/鍵/ログ/更新/About) |

### 2-2. コンポーネント (責務別グループ)

| グループ | 主要ファイル |
|---|---|
| [shell/](../app/components/shell/) | `AppTitlebar` / `AppSidebar` / `EnvironmentBanner` |
| [library/](../app/components/library/) | `ThemeCard` / `ThemeRow` / `ThemeDetailModal` / `ThemeDetailDrawer` / `ApplyModal` / `ImportConflictDialog` / `ThemePickerModal` / `CursorMatrix` / `LibraryToolbar` / `LibraryFilterBar` / `LibraryEmptyState` / `LibraryDropOverlay` |
| [creator/](../app/components/creator/) | `CreatorStartScreen` / `CreatorToolbar` / `CreatorRoleList` / `CreatorMetadataPane` (Hotspot 節を内包) / `NewThemeStartModal` / `SaveDestinationModal` / `BulkImportButton` / `BulkImportPreviewModal` / `BulkImportRoleRow` / `RoleListItem` / `SizeStrip` / `AniThumb` |
| [marketplace/](../app/components/marketplace/) | `FeaturedCard` / `MarketplaceCard` / `SubmitThemeDialog` (Auto/Manual タブ切替) / `MarketplaceDetailModal` / `SubmitDeviceFlowModal` (Device Flow 認証 UI) |
| [settings/](../app/components/settings/) | `GeneralSection` / `StartupSection` / `LibrarySection` / `SecuritySection` / `KeysSection` / `LoggingSection` (ログ出力設定 + クラッシュレポート opt-in トグル / 件数表示 / 送信・クリアボタン) / `UpdatesSection` / `AboutSection` / `SettingsRow` (anchor prop で検索ジャンプ対応) / `SettingsToggle` / `PassphrasePrompt` / `ConfigRecoveryPanel` / `SettingsSearchDropdown` (ja/en 両言語の横断検索ドロップダウン) |
| [preview/](../app/components/preview/) | `CursorPreview` (theme detail で使うプレビュー) |
| [panic/](../app/components/panic/) | `PanicFlow` (ステージ選択 + ライブログ + 17 ロールグリッド) |
| [icons/](../app/components/icons/) | `UiIcon` + `UI_ICONS`、`CursorIcon` + `CURSOR_ICONS` — render 関数で v-html 回避 |
| [ui/](../app/components/ui/) | `UiSelect` (ネイティブ select の白背景を回避) |

### 2-3. Composables (25 個)

| ファイル | 役割 |
|---|---|
| [useTauri.ts](../app/composables/useTauri.ts) | `invokeTauri` IPC ラッパー (Web 開発時フォールバック) |
| [useThemes.ts](../app/composables/useThemes.ts) | テーマ一覧の共有リアクティブ singleton |
| [useAppSettings.ts](../app/composables/useAppSettings.ts) | `get_config`/`update_config` + dirty フラグ |
| [useI18n.ts](../app/composables/useI18n.ts) | `t(key, params)` + フォールバック + OS ロケール検出 |
| [useKeystore.ts](../app/composables/useKeystore.ts) | 鍵生成/削除/エクスポート/インポート + key_id 表示 |
| [useUiTheme.ts](../app/composables/useUiTheme.ts) | アプリ自体の light/dark 切替 |
| [useRoleMatcher.ts](../app/composables/useRoleMatcher.ts) | エイリアス辞書 + `scoreRole` + `resolveCollisions` |
| [useThemePreviews.ts](../app/composables/useThemePreviews.ts) | プレビュー画像取得 (ロール×サイズ) |
| [useBulkImport.ts](../app/composables/useBulkImport.ts) | バルクインポート IPC ラッパー + 進捗購読 |
| [useCreatorAssets.ts](../app/composables/useCreatorAssets.ts) | `assignedPng` / `Hotspot` 統合管理 |
| [useCreatorPickers.ts](../app/composables/useCreatorPickers.ts) | Creator のファイル/フォルダピッカー UX |
| [useCreatorImport.ts](../app/composables/useCreatorImport.ts) | Creator の単発取り込み (`import_cursor_file` / `inspect_ani_file`) |
| [useCreatorBulkImportFlow.ts](../app/composables/useCreatorBulkImportFlow.ts) | バルクインポート preview → confirm のステートマシン |
| [useCreatorExport.ts](../app/composables/useCreatorExport.ts) | `.cursorpack` ストリーム出力フロー (進捗 + cancel) |
| [useHotspotDefaults.ts](../app/composables/useHotspotDefaults.ts) | 役割別の hotspot デフォルト座標 |
| [useHotspotInteraction.ts](../app/composables/useHotspotInteraction.ts) | hotspot ドラッグ操作の純粋関数群 |
| [useAniPlayer.ts](../app/composables/useAniPlayer.ts) | ANI プレビュー再生 (frame タイマー + cancel) |
| [useCursorpackOpener.ts](../app/composables/useCursorpackOpener.ts) | `.cursorpack` ダブルクリック / argv 開封フロー |
| [useUpdater.ts](../app/composables/useUpdater.ts) | check / downloadAndInstall / relaunch |
| [useNotify.ts](../app/composables/useNotify.ts) | Toast 通知 (permission キャッシュ) |
| [sanitizeSvg.ts](../app/composables/sanitizeSvg.ts) | SVG サニタイズ (`<script>`/`href`/`on*`/`javascript:` 除去) |
| [useAppInfo.ts](../app/composables/useAppInfo.ts) | `get_app_info` IPC で取得したアプリ情報 (version, cursors_dir 等) の共有 |
| [useSettingsSearch.ts](../app/composables/useSettingsSearch.ts) | 設定検索カタログ + ja/en 横断 substring 検索 + アンカージャンプ |
| [useMarketplacePreviews.ts](../app/composables/useMarketplacePreviews.ts) | マーケットプレース プレビュー PNG のシングルトンキャッシュ + in-flight 重複排除 (`marketplace_fetch_preview` IPC ラッパー) |
| [useGithubAuth.ts](../app/composables/useGithubAuth.ts) | GitHub Device Flow 認証状態管理。`start_device_flow` / `complete_device_flow` / `cancel_device_flow` / `revoke_github_link` IPC ラッパー + 接続済み GitHub アカウント情報のリアクティブ保持 |
| [useMarketplaceSubmit.ts](../app/composables/useMarketplaceSubmit.ts) | 自動 Marketplace 提出フロー。`submit_theme_auto` IPC ラッパー + 提出進捗・エラー状態管理 |

### 2-4. その他

| ファイル | 役割 |
|---|---|
| [locales/ja.ts](../app/locales/ja.ts), [en.ts](../app/locales/en.ts) | i18n キー (CI で parity 検証) |
| [types/config.ts](../app/types/config.ts), [theme.ts](../app/types/theme.ts), [marketplace.ts](../app/types/marketplace.ts), [githubAuth.ts](../app/types/githubAuth.ts) | IPC ペイロード型 |
| [assets/css/tailwind.css](../app/assets/css/tailwind.css) | Tailwind v4 エントリ + `@theme` ブロック (design tokens) + 横断 shared utility (`.btn` / `.card` / `.chip` / `.input` / `.modal*` ほか) |
| [assets/css/global.css](../app/assets/css/global.css) | `:root` design tokens + CSS リセット + スクロールバー + `:focus-visible` + `prefers-reduced-motion` + 共有 `@keyframes` + `html.light` トークン上書き |
| [layouts/default.vue](../app/layouts/default.vue) | サイドバー連動レイアウト + `panic-hotkey` 購読 |

---

## 3. インフラ / 周辺

| ディレクトリ | 役割 |
|---|---|
| [easy-cursor-swap-crash-report-worker (private)](https://github.com/nishiuriraku/easy-cursor-swap-crash-report-worker) | Cloudflare Workers + KV (dedup / rate limit)。クラッシュレポート受信 endpoint。別 private repo に切り出し済 (2026-05-09) |
| [easy-cursor-swap-index](https://github.com/nishiuriraku/easy-cursor-swap-index) | Marketplace インデックス + `scripts/marketplace/validate.mjs` (Ajv 版: スキーマ / SHA-256 / Ed25519 / VirusTotal v3) + `malware-hashes.txt`。別 repo に切り出し済 (2026-05-08) |
| [scripts/check-i18n.mjs](../scripts/check-i18n.mjs) | ja/en parity CI |
| [scripts/verify-gate.sh](../scripts/verify-gate.sh) | コミット前の正準検証スクリプト |
| [.github/workflows/](../.github/workflows/) | `ci` / `performance` / `release` |

---

## 統計

| 指標 | 値 |
|---|---|
| Rust モジュール数 (lib.rs `pub mod`) | 20 + ベンチ 2 |
| Tauri IPC コマンド数 | 61 |
| Vue ページ数 | 4 (+2 helpers) |
| Vue コンポーネント (subdir 別) | shell 3 / library 12 / creator 12 / marketplace 5 / settings 12 / preview 1 / panic 1 / icons 2 / ui 1 |
| Composables 数 | 25 |
| Vitest テストファイル数 | 14 (composables) + 4 (pages) + 5 (components/creator) + 7 (components/library) + 8 (components/settings) + 2 (components/marketplace) + 1 (components/preview) = 41 |
| CI ワークフロー数 | 3 (ci / performance / release) |
