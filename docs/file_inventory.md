# EasyCursorSwap ファイル機能インベントリ

> 最終更新: 2026-05-20
>
> `src-tauri/src/` と `app/` の **生きたファイル索引**。何がどこにあるかを 1 ファイル単位で把握するための辞書。
> 俯瞰の見取り図は [`architecture.md`](./architecture.md) を参照。
> 構造変更 (ファイル新設・削除・モジュール分割) があったら本書も更新する。

---

## 1. Rust バックエンド (`src-tauri/src/`)

### 1-1. エントリポイント / IPC ハブ

| ファイル | 機能 |
|---|---|
| [main.rs](../src-tauri/src/main.rs) | Tauri アプリのエントリ。tracing 初期化、`StartupCheck::begin()`、AppUserModelID 登録、ConfigManager 初期化、孤児カーソル復旧、pending snapshot リカバリ、`tauri::Builder` 構築 (single-instance プラグイン / 各種 plugin / setup でトレイ・ホットキー)、`build()` + `run(callback)` 形式で `RunEvent::ExitRequested { code: None }` を `prevent_exit` してトレイ常駐させるガード (close ボタン → WebView destroy → プロセス残留 を保証) |
| [lib.rs](../src-tauri/src/lib.rs) | 23 モジュールの `pub mod` 宣言 |
| [commands/mod.rs](../src-tauri/src/commands/mod.rs) | 全 Tauri コマンドのハンドラ登録 (`get_command_handlers()` が 53 IPC を `tauri::generate_handler!` に渡す) |
| [errors.rs](../src-tauri/src/errors.rs) | `AppError` (`thiserror`、`Serialize` 派生で IPC 経由 throw 対応) |

### 1-2. IPC コマンド実体 (9 サブモジュール / 53 個)

| ファイル | 主な IPC |
|---|---|
| [commands/cursor_build/](../src-tauri/src/commands/cursor_build/) | `mod` (公開 API + 共通ステート + `cancel_build` IPC) / `build` (`export_cursorpack`) / `stream` (`export_cursorpack_streamed`、進捗イベント `build-progress`) / `sign` (Ed25519 署名埋込) / `dto` (DTO 定義)。共有キャンセルレジストリは別 `cancel_registry.rs` に切出済 |
| [commands/cursor_io.rs](../src-tauri/src/commands/cursor_io.rs) | `take_pending_cursorpack` (起動時 argv からの `.cursorpack` 引き継ぎ。 `extract_cursorpack_arg` / `stash_pending_cursorpack` / `handle_pending_cursorpack` ヘルパー含む) |
| [commands/theme.rs](../src-tauri/src/commands/theme.rs) | `get_themes` / `get_theme_previews` / `get_theme_role_previews` / `apply_theme` / `set_theme_favorite` / `inspect_cursorpack` / `import_cursorpack` / `delete_theme` / `duplicate_theme` / `repackage_theme` |
| [commands/system.rs](../src-tauri/src/commands/system.rs) | `reset_to_default` / `reset_to_initial` / `get_environment_report` / `get_config` / `update_config` / `get_app_info` / `list_config_backups` / `restore_config_backup` / `open_url` / `open_log_folder` / `get_accessibility_conflicts` / `set_cursor_base_size` / `check_update_is_major_jump` / `list_crash_reports` / `clear_crash_reports` / `submit_crash_reports` |
| [commands/keystore.rs](../src-tauri/src/commands/keystore.rs) | `keystore_info` / `keystore_generate` / `keystore_delete` / `keystore_export` / `keystore_import` |
| [commands/marketplace.rs](../src-tauri/src/commands/marketplace.rs) | `marketplace_fetch_index` / `marketplace_install` / `marketplace_fetch_preview` |
| [commands/marketplace_submit.rs](../src-tauri/src/commands/marketplace_submit.rs) | `start_device_flow` / `complete_device_flow` / `cancel_device_flow` / `submit_theme_auto` / `revoke_github_link` |
| [commands/profile.rs](../src-tauri/src/commands/profile.rs) | `export_profile` / `import_profile` |
| [commands/windows_scheme.rs](../src-tauri/src/commands/windows_scheme.rs) | `list_windows_schemes` / `apply_windows_scheme` / `get_windows_scheme_previews` / `get_windows_scheme_role_previews` / `export_windows_scheme_as_cursorpack` |
| [bulk_import/](../src-tauri/src/bulk_import/) | `bulk_resolve_assets` / `cancel_bulk_import` / `parse_cursorpack_for_creator` (実体は `bulk_import` モジュール側、`tauri::command` 属性付きの関数を `commands/mod.rs` から再エクスポート) |

### 1-3. ドメイン / 機能モジュール

| ファイル | 機能 |
|---|---|
| [config.rs](../src-tauri/src/config.rs) | `AppConfig` の RwLock + schema_version (v1 固定) + `config.corrupt.{epoch}.json` 退避 |
| [registry/mod.rs](../src-tauri/src/registry/mod.rs) | `HKCU\Control Panel\Cursors` 読み書き、theme apply の `SPI_SETCURSORS` 通知 (`notify_cursor_change`) + `SPI_SETCURSORSHADOW` (`set_cursor_shadow`)、`LoadImageW` + `SetSystemCursor`(OCR_* × 14) によるカーソル即時リサイズ (`apply_system_cursors_at_size`、cursor size 変更経路は broadcast を意図的に行わない)、トランザクション + `_pending_apply.snapshot`、`save_initial_snapshot` / `check_pending_snapshot` / `reset_to_windows_default` |
| [registry/roles.rs](../src-tauri/src/registry/roles.rs) | 17 役割の絶対パス書き込みロジック、`compute_apply_values` 純粋関数 |
| [registry/scheme.rs](../src-tauri/src/registry/scheme.rs) | `Schemes` への REG_EXPAND_SZ 登録、`build_scheme_value` / `sanitize_scheme_name` |
| [registry/env.rs](../src-tauri/src/registry/env.rs) | レジストリ環境変数操作補助 |
| [cursor/cur_build.rs](../src-tauri/src/cursor/cur_build.rs) | PNG → .cur マルチ解像度 6 サイズパッキング、ホットスポット書込み |
| [cursor/image.rs](../src-tauri/src/cursor/image.rs) | Lanczos / Nearest リサイズ + ドット絵自動判定 + `RESIZE_CACHE` (64 エントリ FIFO) + `strip_png_metadata` |
| [cursor/ani.rs](../src-tauri/src/cursor/ani.rs) | ANI (RIFF/ACON) パーサー + `ParsedAni::total_duration_ms` (検査専用) |
| [cursor/ani_write.rs](../src-tauri/src/cursor/ani_write.rs) | ANI 書き出し (.cur frame の連結 + RIFF header + hotspot 埋込) |
| [cursor/ico_cur.rs](../src-tauri/src/cursor/ico_cur.rs) | .ico/.cur 解析 (ICONDIRENTRY + PNG/BMP DIB) + `pick_largest_as_png` |
| [cursor_watcher.rs](../src-tauri/src/cursor_watcher.rs) | `WM_SETTINGCHANGE` 購読 (不可視ウィンドウ) → `cursor-changed` イベント |
| [theme/mod.rs](../src-tauri/src/theme/mod.rs) | エントリポイント。`ThemeManager` (ZST) 定義 + 公開 re-export。実体メソッドは下 4 サブモジュール (listing / preview / apply / package) の `impl ThemeManager` に分散 |
| [theme/listing.rs](../src-tauri/src/theme/listing.rs) | `list_themes` / `load_metadata` / `theme_exists` / `cleanup_orphan_references` / `dir_size_bytes`、および `set_metadata_source` 共有 helper |
| [theme/preview.rs](../src-tauri/src/theme/preview.rs) | `load_role_previews` / `load_role_previews_with_hotspots` / `render_paths_as_previews(_with_hotspots)` / `render_cursor_file_as_png` / `build_preview_pngs`、`RolePreview` DTO + `PREVIEW_ROLES` / `PREVIEW_SIZE` const |
| [theme/apply.rs](../src-tauri/src/theme/apply.rs) | `apply_theme` (レジストリ書込 + SPI_SETCURSORS + Schemes 登録) / `theme_active_in_registry` |
| [theme/package.rs](../src-tauri/src/theme/package.rs) | `.cursorpack` zip 入出力: `import_cursorpack_*` / `inspect_cursorpack_*` / `write_cursorpack_to_buffer` / `export_cursorpack` / `delete_theme` / `duplicate_theme` / `repackage_theme` / `export_scheme_as_cursorpack` |
| [theme/sanitize.rs](../src-tauri/src/theme/sanitize.rs) | `sanitize_archive_path` + Zip 爆弾対策 (50/200/10 MB 三段階) + `S_IFLNK` 拒否 |
| [theme/types.rs](../src-tauri/src/theme/types.rs) | `ThemeMeta` / `ThemeSummary` / `LocalizedString` 等 |
| [bulk_import/mod.rs](../src-tauri/src/bulk_import/mod.rs) | 公開 API + 進捗イベント + DTO 集約 (`CancelRegistry` は `cancel_registry.rs` に切り出して cursor_build と共有) |
| [cancel_registry.rs](../src-tauri/src/cancel_registry.rs) | 長時間ジョブのキャンセルレジストリ。Tauri App state として `manage()` する per-instance struct (HashMap<String, bool>)。`register` / `cancel` / `is_active` / `is_cancelled` / `drop_job` API |
| [bulk_import/assets.rs](../src-tauri/src/bulk_import/assets.rs) | 複数ファイル/フォルダ走査 → `ResolvedAsset` 変換、ファジーマッチ、リサンプル並列化 |
| [bulk_import/cursorpack.rs](../src-tauri/src/bulk_import/cursorpack.rs) | `.cursorpack` 読込 (ライブラリ非依存、creator 直挿入用) |
| [backup.rs](../src-tauri/src/backup.rs) | `.cursorprofile` Zip 入出力、`ProfileEnvelope`、merge/overwrite |
| [marketplace.rs](../src-tauri/src/marketplace.rs) | `MarketplaceClient`：reqwest(rustls) + SHA-256 + Ed25519 + `historical_keys` ローテーション + 50MB ガード。`fetch_preview`：URL スキーム/ホスト + ロール名バリデーション + 500KB 上限でプレビュー PNG を取得 |
| [keystore.rs](../src-tauri/src/keystore.rs) | DPAPI 暗号化保存、`generate`/`sign`/`verify`、XChaCha20-Poly1305 + Argon2id `.cfkey` 入出力。`save/load/delete_github_oauth_token` で GitHub OAuth トークンも DPAPI 管理 |
| [github/mod.rs](../src-tauri/src/github/mod.rs) | `github` モジュール公開 API と re-export |
| [github/types.rs](../src-tauri/src/github/types.rs) | Device Flow / PR 作成で使う Rust 型 (`DeviceFlowResponse`, `GithubPrResult` 等) |
| [github/device_flow.rs](../src-tauri/src/github/device_flow.rs) | GitHub OAuth Device Flow 実装。`start` → ポーリング → トークン取得・DPAPI 保存。scope は `public_repo` 限定 |
| [github/client.rs](../src-tauri/src/github/client.rs) | GitHub REST API クライアント。PR 作成 / ブランチ操作 / `.cursorpack` アップロード。`client_id` は `option_env!("EASY_CURSOR_SWAP_GITHUB_OAUTH_CLIENT_ID")` で注入 |
| [health.rs](../src-tauri/src/health.rs) | `startup.json` の `pending_failures`、3 回連続失敗検知、バージョン変更で自動リセット |
| [rollback.rs](../src-tauri/src/rollback.rs) | 自動ロールバック (`download_to_temp` / `verify_minisign` / `launch_silent_installer`)。pre-main フェーズで `main.rs::show_rollback_dialog` の Yes 経路から呼ばれる。`tauri-plugin-updater` と同じ `minisign-verify` crate で署名互換 |
| [crash.rs](../src-tauri/src/crash.rs) | `install_panic_hook`、`%LOCALAPPDATA%\...\crash\panic-{epoch}.json`、`prune_old_reports`、`general.crash_reporting` 同意、送信ペイロード生成 |
| [tray.rs](../src-tauri/src/tray.rs) | システムトレイ + `show_or_recreate_main_window` (WebView 破棄/再生成) |
| [hotkey.rs](../src-tauri/src/hotkey.rs) | `RegisterHotKey` で `Ctrl+Alt+Shift+R` → `panic-hotkey` イベント |
| [autostart.rs](../src-tauri/src/autostart.rs) | `HKCU\...\Run` 登録、MSIX 検出時は no-op で `startupTask` に委譲 |
| [appusermodel.rs](../src-tauri/src/appusermodel.rs) | `SetCurrentProcessExplicitAppUserModelID("dev.easycursorswap.app")` |
| [accessibility.rs](../src-tauri/src/accessibility.rs) | `CursorIndicator` / `ContrastScheme` / `CursorBaseSize` 競合検出 + `Accessibility\CursorSize` (slider 1-15) / `Accessibility\CursorType` (0/1/2/3/6/...) を IPC レスポンス用に取得 (eoa pipeline 状態を frontend 側で `cursor_size_slider != 1` で判定する) |
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

### 2-1. ページ (4 画面 + 2 helpers)

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
| [library/](../app/components/library/) (14) | `ThemeCard` / `ThemeRow` / `ThemeDetailModal` (フッターアクション群を UiModal `#leftNote` / `#actions` slot に直接配置) / `ThemeDetailDrawer` / `ThemeDetailDrawerHero` / `ThemeDetailDrawerStrip` / `ApplyModal` / `ImportConflictDialog` / `ThemePickerModal` / `CursorMatrix` / `LibraryToolbar` / `LibraryFilterBar` / `LibraryEmptyState` / `LibraryDropOverlay` |
| [creator/](../app/components/creator/) (14) | `CreatorStartScreen` / `CreatorToolbar` / `CreatorRoleList` / `CreatorMetadataPane` (Hotspot 節を内包) / `CreatorEditorCanvas` / `NewThemeStartModal` / `SaveDestinationModal` / `DiscardEditDialog` (Clear / 画面遷移時の編集破棄確認) / `BulkImportButton` / `BulkImportPreviewModal` / `BulkImportRoleRow` / `RoleListItem` / `SizeStrip` / `AniThumb` |
| [marketplace/](../app/components/marketplace/) (6) | `FeaturedCard` / `SubmitThemeDialog` (Auto/Manual タブ切替) / `SubmitThemeAutoForm` / `SubmitThemeManualForm` / `MarketplaceDetailModal` / `SubmitDeviceFlowModal` (Device Flow 認証 UI) |
| [settings/](../app/components/settings/) (14) | `GeneralSection` / `StartupSection` / `LibrarySection` / `SecuritySection` / `KeysSection` / `LoggingSection` (ログ出力設定 + クラッシュレポート opt-in トグル / 件数表示 / 送信・クリアボタン) / `UpdatesSection` / `AboutSection` / `SettingsRow` (anchor prop で検索ジャンプ対応) / `SettingsToggle` / `PassphrasePrompt` / `ConfigRecoveryPanel` / `SettingsSearchDropdown` (ja/en 両言語の横断検索ドロップダウン) / `OssLicenseModal` |
| [preview/](../app/components/preview/) | `CursorPreview` (theme detail で使うプレビュー) |
| [panic/](../app/components/panic/) | `PanicFlow` (ステージ選択 + ライブログ + 17 ロールグリッド) |
| [icons/](../app/components/icons/) | `UiIcon` + `UI_ICONS`、`CursorIcon` + `CURSOR_ICONS` — render 関数で v-html 回避 |
| [ui/](../app/components/ui/) (5) | `UiSelect` (ネイティブ select の白背景を回避) / `UiButton` (.btn shared utility の Vue ラッパ + loading/icon ハンドリング) / `UiAlert` (info/success/warn/danger インラインバナー) / `UiModal` (Teleport + focus trap + useModalLifecycle を内包する shared modal shell) / `UiConfirmDialog` (UiModal + UiButton を compose した cancel/confirm 専用ダイアログ) |

### 2-3. Composables (36 個)

| ファイル | 役割 |
|---|---|
| [useTauri.ts](../app/composables/useTauri.ts) | `invokeTauri` IPC ラッパー (Web 開発時フォールバック) |
| [useThemes.ts](../app/composables/useThemes.ts) | テーマ一覧の共有リアクティブ singleton + apply/delete/duplicate/repackage/set_favorite/inspect/import IPC ラッパ |
| [useAppSettings.ts](../app/composables/useAppSettings.ts) | `get_config`/`update_config` + dirty フラグ |
| [useI18n.ts](../app/composables/useI18n.ts) | `t(key, params)` + フォールバック + OS ロケール検出 |
| [useKeystore.ts](../app/composables/useKeystore.ts) | 鍵生成/削除/エクスポート/インポート + key_id 表示 |
| [useUiTheme.ts](../app/composables/useUiTheme.ts) | アプリ自体の light/dark 切替 |
| [useRoleMatcher.ts](../app/composables/useRoleMatcher.ts) | エイリアス辞書 + `scoreRole` + `resolveCollisions` |
| [useThemePreviews.ts](../app/composables/useThemePreviews.ts) | プレビュー画像取得 (ロール×サイズ) |
| [useBulkImport.ts](../app/composables/useBulkImport.ts) | バルクインポート IPC ラッパー + 進捗購読 + `.cursorpack` 解析 |
| [useBulkImportPreviewState.ts](../app/composables/useBulkImportPreviewState.ts) | `BulkImportPreviewModal` の matches/unmatched 三方移動 state machine + Blob URL ライフサイクル + ApplyPayload 組立。`PendingMatch` / `UnmatchedFile` / `ApplyPayload` 型もここで export |
| [useCreatorAssets.ts](../app/composables/useCreatorAssets.ts) | `assignedPng` / `Hotspot` 統合管理 |
| [useCreatorPickers.ts](../app/composables/useCreatorPickers.ts) | Creator のファイル/フォルダピッカー UX |
| [useCreatorImport.ts](../app/composables/useCreatorImport.ts) | Creator の単発取り込み (`import_cursor_file` / `inspect_ani_file`) |
| [useCreatorBulkImportFlow.ts](../app/composables/useCreatorBulkImportFlow.ts) | バルクインポート preview → confirm のステートマシン (IPC 呼出は useBulkImport へ delegation) |
| [useCreatorExport.ts](../app/composables/useCreatorExport.ts) | `.cursorpack` ストリーム出力フロー (進捗 + cancel) |
| [useCreatorMetaState.ts](../app/composables/useCreatorMetaState.ts) | Creator メタデータ入力欄 (name / nameEn / author / version / description / shadowEnabled) の 6 ref と reset() を集約 |
| [useHotspotDefaults.ts](../app/composables/useHotspotDefaults.ts) | 役割別の hotspot デフォルト座標 |
| [useHotspotInteraction.ts](../app/composables/useHotspotInteraction.ts) | hotspot ドラッグ操作の純粋関数群 |
| [useAniPlayer.ts](../app/composables/useAniPlayer.ts) | ANI プレビュー再生 (frame タイマー + cancel) |
| [useCursorpackOpener.ts](../app/composables/useCursorpackOpener.ts) | `.cursorpack` ダブルクリック / argv 開封フロー (`take_pending_cursorpack` IPC + `cursorpack-import-requested` event 重複排除) |
| [useUpdater.ts](../app/composables/useUpdater.ts) | tauri-apps/plugin-updater プラグイン API の check / downloadAndInstall / relaunch + `classifyUpdaterError` |
| [useUpdaterBootstrap.ts](../app/composables/useUpdaterBootstrap.ts) | 起動時 1 回だけ `auto_update + 24h クールダウン` で check し、`check_update_is_major_jump` IPC でメジャージャンプ判定後にヒット時 Toast 通知 (`app.vue` から呼び出し) |
| [useNotify.ts](../app/composables/useNotify.ts) | Toast 通知 (permission キャッシュ) |
| [sanitizeSvg.ts](../app/composables/sanitizeSvg.ts) | SVG サニタイズ (`<script>`/`href`/`on*`/`javascript:` 除去) |
| [useAppInfo.ts](../app/composables/useAppInfo.ts) | `get_app_info` IPC で取得したアプリ情報 (version, cursors_dir 等) の共有 |
| [useSettingsSearch.ts](../app/composables/useSettingsSearch.ts) | 設定検索カタログ + ja/en 横断 substring 検索 + アンカージャンプ |
| [useMarketplacePreviews.ts](../app/composables/useMarketplacePreviews.ts) | マーケットプレース プレビュー PNG のシングルトンキャッシュ + in-flight 重複排除 (`marketplace_fetch_preview` IPC ラッパー) |
| [useGithubAuth.ts](../app/composables/useGithubAuth.ts) | GitHub Device Flow 認証状態管理。`start_device_flow` / `complete_device_flow` / `cancel_device_flow` IPC ラッパー + 接続済み GitHub アカウント情報のリアクティブ保持 (revoke_github_link は pages/settings.vue 側) |
| [useMarketplaceSubmit.ts](../app/composables/useMarketplaceSubmit.ts) | 自動 Marketplace 提出フロー。`submit_theme_auto` IPC ラッパー + 提出進捗・エラー状態管理 |
| [pickLocalizedName.ts](../app/composables/pickLocalizedName.ts) | `MarketplaceEntry.name` (`LocalizedString`: string \| locale マップ) を現 locale で 1 つの表示文字列に解決する純関数。Rust `crate::theme::LocalizedString::get` と 1:1 同期し、`FeaturedCard` / `MarketplaceDetailModal` / 検索 / トーストで共有 |
| [useExternalUrl.ts](../app/composables/useExternalUrl.ts) | `open_url` IPC + `window.open` フォールバックの 1 行 API。6 callsite で重複していた try/catch を集約 |
| [useListbox.ts](../app/composables/useListbox.ts) | `UiSelect` の listbox 状態機械 + キーボードナビ + viewport-aware Teleport 位置計算 |
| [useModalLifecycle.ts](../app/composables/useModalLifecycle.ts) | Teleport modal の body scroll lock (重ね合わせ対応 counter) + Esc 購読 + cleanup |
| [useFocusTrap.ts](../app/composables/useFocusTrap.ts) | モーダル / ダイアログ用 focus trap (Tab/Shift+Tab wrap + 初期 focus + active=false で直前の要素へ復帰)。`UiModal` から利用 |
| [usePngBlobCache.ts](../app/composables/usePngBlobCache.ts) | Map + in-flight Promise + dispose の汎用キャッシュ機構 (useThemePreviews / useMarketplacePreviews / Creator の blob URL キャッシュで共有) |
| [useThemeCardState.ts](../app/composables/useThemeCardState.ts) | `ThemeCard` / `ThemeRow` の 5 ブロック並行重複 (preview fetch / kind 判定 / displayDate / 詳細遷移 / お気に入り) を共通化 |

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
| Rust モジュール数 (lib.rs `pub mod`) | 23 + ベンチ 2 |
| Tauri IPC コマンド数 | 53 |
| Vue ページ数 | 4 (+2 helpers) |
| Vue コンポーネント (subdir 別) | shell 3 / library 14 / creator 14 / marketplace 6 / settings 14 / preview 1 / panic 1 / icons 2 / ui 5 (合計 60) |
| Composables 数 | 36 |
| Vitest テストファイル数 | 20 (composables) + 4 (pages) + 6 (components/creator) + 7 (components/library) + 8 (components/settings) + 2 (components/marketplace) + 1 (components/preview) = 48 |
| CI ワークフロー数 | 3 (ci / performance / release) |
