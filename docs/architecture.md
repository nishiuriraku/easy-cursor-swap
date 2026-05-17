# EasyCursorSwap アーキテクチャマップ

> 最終更新: 2026-05-13
>
> ファイルの責務を一望するための **生きたインデックス**。実装が増減したら本書を更新する。
> リファクタや初見オンボード時に「どのモジュールがどう繋がっているか」を素早く掴むためのもの。
> ファイル単位の詳細索引は [`file_inventory.md`](./file_inventory.md) を参照。

## 全体像

```
┌─────────────────────────────────────────────────────────────────┐
│ Vue / Nuxt 4 SPA  (app/)                                        │
│   pages → components → composables → useTauri.invoke()           │
└───────────────────────┬─────────────────────────────────────────┘
                        │ IPC (Tauri 2 / serde)
┌───────────────────────▼─────────────────────────────────────────┐
│ Rust バックエンド  (src-tauri/src/)                              │
│   commands/ (53 IPC 受け口を 10 サブモジュールに分割)            │
│   ├─ config / theme / cursor / registry  ← Source of Truth      │
│   ├─ marketplace / keystore / bulk_import                       │
│   └─ tray / hotkey / health / crash                             │
└─────────────────────────────────────────────────────────────────┘
                        │
                ┌───────┴───────┐
                ▼               ▼
        HKCU\Control Panel\   ~/.custom_cursors/
        Cursors                 ├─ <UUID>/theme.json + *.cur
        Cursors\Schemes         ├─ _initial_snapshot.json
                                ├─ _pending_apply.snapshot
                                └─ _keys/  (DPAPI 暗号化 Ed25519)
```

**重要な不変条件**

- HKCU のみを書き換える（HKLM や UAC は触らない）
- 適用はトランザクショナル: `_pending_apply.snapshot` → 書込 → 削除。残ってたら起動時に自動巻き戻し
- `~/.custom_cursors/` はアンインストール後も残す
- ログには PII を出さない: パスは `redact_path`、ハッシュは `short_hash`(12)
- アーカイブ展開は `theme::sanitize_archive_path` 必須 (50/200/10/1024 MB ガード)
- Vue では `v-html` 禁止。SVG は `composables/sanitizeSvg`、render-function は `components/icons/`

詳細な security 不変条件は [Security](#security) を参照。

## Rust 側モジュール (`src-tauri/src/`)

`lib.rs` は 21 個のモジュールを `pub mod` で公開し、`main.rs` から `tauri::Builder` に組み込む。
直近のリファクタで `commands` / `cursor` / `theme` / `bulk_import` / `registry` を **ディレクトリ + サブモジュール構成** に分割済み。多重起動防止は自前 `single_instance.rs` を廃止し `tauri_plugin_single_instance` プラグインに移行。

### 責務マップ

| カテゴリ | モジュール | 主な役割 |
|---|---|---|
| **IPC 表玄関** | `commands/` | 53 個の `#[tauri::command]` を 10 サブモジュールに分割。`mod.rs::get_command_handlers()` が `tauri::generate_handler!` にまとめて渡す。サブモジュール: `theme` / `cursor_build/` (build / cancel / dto / sign / stream の 5 ファイル分割) / `cursor_io` / `keystore` / `marketplace` / `marketplace_submit` / `profile` / `system` / `updater` (チャンネル切替 endpoint override) / `windows_scheme` |
| **GitHub API クライアント** | `github/` | OAuth Device Flow + REST API (`mod.rs` / `types.rs` / `device_flow.rs` / `client.rs`)。Marketplace 自動提出フローから利用。`client_id` は build 時に `option_env!("EASY_CURSOR_SWAP_GITHUB_OAUTH_CLIENT_ID")` で注入。 |
| **設定 / 状態** | `config.rs` | `AppConfig` / `ConfigManager` (RwLock + schema_version (v1 固定) + パースエラー時 `config.corrupt.*.json` 退避) |
| | `errors.rs` | `AppError` / `AppResult` 共通型 |
| **カーソル生成パイプライン** | `cursor/` | 5 サブモジュール: `image` (リサイズ / hotspot / メタデータ剥離) / `cur_build` (PNG → 6 解像度 .cur) / `ico_cur` (ICO/CUR 解析) / `ani` (RIFF/ACON 解析) / `ani_write` (ANI 書き出し)。`mod.rs` で全シンボルを `pub use` 再エクスポート |
| | `cursor_watcher.rs` | コントロールパネル等で外部に書き換えられたら `cursor-changed` を発火 |
| **レジストリ操作** | `registry/` | 4 ファイル: `mod` (適用 / 復元 / pending snapshot / SPI 通知) / `scheme` (`HKCU\...\Schemes` の列挙と書込) / `roles` (17 ロール↔レジストリ値名マッピング) / `env` (RDP / Citrix / 環境検出ヘルパ) |
| **テーマパッケージ** | `theme/` | 3 ファイル: `types` (DTO + 内部 helper) / `sanitize` (path traversal 対策) / `mod` (`ThemeManager` impl 本体)。`.cursorpack` の作成・解凍・バリデーション、theme.json 管理、孤児カーソル復旧 |
| | `bulk_import/` | 3 ファイル: `mod` (CancelRegistry / 公開 API) / `assets` (ファイル・フォルダ並列解決) / `cursorpack` (パック解析の Creator 向けエントリ) |
| | `backup.rs` | `.cursorprofile` (config + 全テーマの ZIP) の export/import |
| **マーケットプレース** | `marketplace.rs` | HTTP インデックス取得 (rustls-tls)、SHA-256 + Ed25519 検証、ダウンロードサイズ上限。プレビュー PNG 取得 (`fetch_preview`: URL バリデーション + ロール名バリデーション + 500KB 上限) |
| | `keystore.rs` | クリエイター用 Ed25519 鍵ペア (DPAPI 暗号化), `.cfkey` import/export (XChaCha20-Poly1305 + Argon2id), key_id 計算 (SHA-256[:16]) |
| **信頼性 / 復旧** | `health.rs` | 起動連続失敗カウンタ + ロールバック対象バージョン算出 |
| | `crash.rs` | panic フック + `crash-reports/` ディレクトリの retention + 投稿ペイロード生成 |
| **OS 統合** | `tray.rs` | システムトレイ常駐 / メインウィンドウ再生成 |
| | `hotkey.rs` | グローバルホットキー (Ctrl+Alt+Shift+R 等) |
| | `autostart.rs` | `HKCU\...\Run` 自動起動レジストリ管理 (config が Source of Truth) |
| | `appusermodel.rs` | AppUserModelID 登録 (Win トースト発信元) |
| | `accessibility.rs` | マウスソナー / ハイコントラスト / カーソル拡大の検出 |
| | `environment.rs` | RDP / Citrix / Sandbox など動作対象外環境の検出 |
| **観測** | `logging.rs` | tracing 初期化 + `redact_path` / `short_hash` PII helper |

> 多重起動防止は `tauri_plugin_single_instance::init` プラグインに集約 (argv ハンドオーバ含む)。旧 `single_instance.rs` モジュールは削除済。

### IPC 一覧 (53 commands)

`commands::get_command_handlers()` が `tauri::generate_handler!` に登録する 53 個。フロントは `app/composables/useTauri.ts::invokeTauri<T>(name, args)` で呼ぶ。

| カテゴリ | コマンド名 |
|---|---|
| テーマ (10) | `get_themes`, `apply_theme`, `delete_theme`, `duplicate_theme`, `repackage_theme`, `get_theme_previews`, `get_theme_role_previews`, `set_theme_favorite`, `inspect_cursorpack`, `import_cursorpack` |
| .cursorpack ビルド (3) | `export_cursorpack`, `export_cursorpack_streamed`, `cancel_build` |
| .cursorpack ファイル関連付け (1) | `take_pending_cursorpack` |
| 鍵管理 (5) | `keystore_info`, `keystore_generate`, `keystore_delete`, `keystore_export`, `keystore_import` |
| マーケットプレース (3) | `marketplace_fetch_index`, `marketplace_install`, `marketplace_fetch_preview` |
| マーケットプレース自動提出 (5) | `start_device_flow`, `complete_device_flow`, `cancel_device_flow`, `submit_theme_auto`, `revoke_github_link` |
| プロファイル (2) | `export_profile`, `import_profile` |
| Windows スキーム (5) | `list_windows_schemes`, `apply_windows_scheme`, `get_windows_scheme_previews`, `get_windows_scheme_role_previews`, `export_windows_scheme_as_cursorpack` |
| システム / 設定 / 診断 (15) | `reset_to_default`, `reset_to_initial`, `get_environment_report`, `get_config`, `update_config`, `get_app_info`, `list_config_backups`, `restore_config_backup`, `check_update_is_major_jump`, `open_url`, `open_log_folder`, `get_accessibility_conflicts`, `list_crash_reports`, `clear_crash_reports`, `submit_crash_reports` |
| Updater (1) | `check_for_update_on_channel` (channel='beta' 時に runtime で別 endpoint を引く) |
| 一括取込 (3) | `bulk_resolve_assets`, `cancel_bulk_import`, `parse_cursorpack_for_creator` |

### 起動シーケンス (`main.rs`)

1. tracing logger 初期化 (フォールバック有り)
2. **起動ヘルスチェック** (`StartupCheck::begin`) — 連続 3 回失敗ならロールバック誘導ダイアログ (`show_rollback_dialog`)
3. AppUserModelID 登録 (`appusermodel::register_aumid`)
4. `ConfigManager::init` (失敗時は `show_migration_failure_dialog` で `config.corrupt.*.json` の場所を案内し終了)
5. autostart レジストリを config に追従 (`autostart::set_enabled`)
6. 初回スナップショット保存 (`RegistryManager::save_initial_snapshot`)
7. 孤児カーソル参照のクリーンアップ (`ThemeManager::cleanup_orphan_references`)
8. **クラッシュリカバリ**: `RegistryManager::check_pending_snapshot` — 残っていれば前回の適用処理が中断 → Windows 既定へ復元
9. `tauri::Builder` を組み立て、`tauri_plugin_single_instance::init` で多重起動防止 + argv ハンドオーバ
10. `setup` 内でトレイ起動 (`tray::setup_tray`)、パニックホットキー登録、カーソルウォッチャ起動
11. 全部成功 → `mark_healthy` (失敗カウンタをリセット)

## Security

README の security テーブルは概要、本セクションは **不変条件 (守るべき規律) と担当モジュールのマッピング**。アルゴリズム詳細・サイズ上限は実装コードを参照。

| 不変条件 | 担当 |
|---|---|
| HKCU のみ書く / HKLM・UAC を触らない | `registry/mod.rs` |
| 適用は **書く前に snapshot を残し、成功後に消す** (起動時に残骸を検出したら自動巻き戻し) | `registry/mod.rs::check_pending_snapshot` / `reset_to_windows_default` |
| パニック復旧用に初回起動時のスナップショットを保存 | `registry/mod.rs::save_initial_snapshot` |
| 著者鍵は DPAPI で暗号化して `~/.custom_cursors/_keys/` に保存 (`CryptProtectData`) | `keystore.rs::dpapi_encrypt` |
| 鍵エクスポート時は XChaCha20-Poly1305 + Argon2id でパスフレーズ暗号化 (`.cfkey`) | `keystore.rs` |
| Marketplace 投稿パックは SHA-256 + Ed25519 を**ダウンロード後に必ず検証** | `marketplace.rs::verify_pack` |
| Marketplace 自動提出の GitHub OAuth トークンは DPAPI で暗号化、scope は `public_repo` 限定 | `keystore.rs::save_github_oauth_token` / `github/device_flow.rs` |
| ダウンロード前に Content-Length を見て三段階サイズ上限 (50 MB 圧縮 / 200 MB 展開 / 10 MB / image) | `marketplace.rs` / `theme/sanitize.rs` |
| Marketplace テーマは**読み取り専用**: `repackage_theme` IPC がソース確認し、marketplace 由来のテーマは編集・エクスポート要求を拒否する | `commands/theme.rs::repackage_theme` |
| プレビュー PNG 取得は URL スキーム + ホスト + ロール名を検証し、500KB を超えるレスポンスは拒否 | `marketplace.rs::fetch_preview` |
| アーカイブ展開は `sanitize_archive_path` を必ず通す (path traversal / symlink / 絶対パス拒否) | `theme/sanitize.rs::sanitize_archive_path` |
| PNG 取り込み時に eXIf / iTXt / zTXt メタデータを剥離 | `cursor/image.rs` |
| SVG は WebView で表示する前に script / style / on* 属性を除去 | `app/composables/sanitizeSvg.ts` |
| Vue では `v-html` を使わない (CI で grep ベースで検出) | `app/components/icons/{UiIcon,CursorIcon}.vue` の render-function 方式 |
| ログには PII を残さない (パスは `redact_path`、ハッシュは `short_hash[:12]`) | `logging.rs` |
| HTTPS は rustls-tls (OS の TLS スタックに依存しない) | `Cargo.toml` の reqwest features |

## フロントエンド (`app/`)

Nuxt 4 SPA。`pathPrefix: false` のため component はファイル名 (basename) で参照する。

### ディレクトリ構成

```
app/
├─ pages/          ← トップレベル画面 (Library / Creator / Marketplace / Settings)
├─ layouts/        ← default.vue (シェル: AppTitlebar + AppSidebar + slot)
├─ components/
│  ├─ shell/       ← AppTitlebar, AppSidebar, EnvironmentBanner
│  ├─ library/     ← ThemeCard, ThemeRow, ThemeDetailModal, ThemeDetailDrawer, ApplyModal,
│  │                ImportConflictDialog, ThemePickerModal, CursorMatrix, LibraryToolbar,
│  │                LibraryFilterBar, LibraryEmptyState, LibraryDropOverlay
│  ├─ creator/     ← CreatorStartScreen, CreatorToolbar, CreatorRoleList, CreatorMetadataPane,
│  │                NewThemeStartModal, SaveDestinationModal, BulkImportButton,
│  │                BulkImportPreviewModal, BulkImportRoleRow, RoleListItem, SizeStrip, AniThumb
│  ├─ marketplace/ ← FeaturedCard, SubmitThemeDialog, MarketplaceDetailModal, SubmitDeviceFlowModal
│  ├─ settings/    ← SettingsRow, SettingsToggle, ConfigRecoveryPanel, PassphrasePrompt,
│  │                SettingsSearchDropdown, GeneralSection, StartupSection,
│  │                LibrarySection, SecuritySection, KeysSection, LoggingSection,
│  │                UpdatesSection, AboutSection
│  ├─ preview/     ← CursorPreview (theme detail で使うプレビュー)
│  ├─ panic/       ← PanicFlow (Stage 1 / Stage 2 リカバリ)
│  ├─ ui/          ← UiSelect (ネイティブ select の白背景を回避)
│  └─ icons/       ← UiIcon / CursorIcon (render-function ベースで v-html を使わない)
├─ composables/    ← useThemes, useAppSettings, useI18n, useTauri (IPC), useKeystore, useUiTheme,
│                    useRoleMatcher, useThemePreviews, useBulkImport, useUpdater, useNotify,
│                    sanitizeSvg, useCreatorAssets, useCreatorPickers, useCreatorImport,
│                    useCreatorBulkImportFlow, useCreatorExport, useHotspotDefaults,
│                    useHotspotInteraction, useAniPlayer, useCursorpackOpener,
│                    useAppInfo, useSettingsSearch, useMarketplacePreviews,
│                    useGithubAuth (GitHub Device Flow 認証・トークン管理),
│                    useMarketplaceSubmit (自動 PR 提出フロー),
│                    useUpdaterBootstrap (起動時 1 回の auto_update チェック) (合計 27)
├─ types/          ← config.ts, theme.ts, marketplace.ts, githubAuth.ts (Rust struct と 1:1)
├─ locales/        ← ja.ts, en.ts (CI で parity チェック)
├─ assets/css/     ← tailwind.css (Tailwind v4 entry + @theme + 横断 shared utility) +
│                    global.css (デザイントークン / CSS リセット / @keyframes)
└─ plugins/        ← clickOutside.client.ts
```

### Page → Composable → IPC の主な経路

| Page | 主な composable / Component | 主な IPC |
|---|---|---|
| `index.vue` (Library) | useThemes, useCursorpackOpener, useThemePreviews, ThemeCard / ThemeRow, ApplyModal, ImportConflictDialog, ThemePickerModal | `get_themes`, `apply_theme`, `import_cursorpack`, `inspect_cursorpack`, `delete_theme`, `duplicate_theme`, `repackage_theme`, `list_windows_schemes`, `apply_windows_scheme`, `get_windows_scheme_role_previews`, `export_windows_scheme_as_cursorpack`, `set_theme_favorite`, `take_pending_cursorpack` |
| `creator.vue` | useCreatorAssets, useCreatorPickers, useCreatorImport, useCreatorBulkImportFlow, useCreatorExport, useRoleMatcher, useBulkImport, useKeystore, useHotspotDefaults, useHotspotInteraction, useAniPlayer, sanitizeSvg, NewThemeStartModal, BulkImportPreviewModal | `parse_cursorpack_for_creator`, `bulk_resolve_assets`, `cancel_bulk_import`, `export_cursorpack_streamed`, `cancel_build`, `repackage_theme`, `apply_theme`, `keystore_info` |
| `marketplace.vue` | useThemes, useMarketplacePreviews, useGithubAuth, useMarketplaceSubmit, FeaturedCard, MarketplaceDetailModal, SubmitThemeDialog, SubmitDeviceFlowModal | `marketplace_fetch_index`, `marketplace_install`, `marketplace_fetch_preview`, `keystore_info`, `start_device_flow`, `complete_device_flow`, `cancel_device_flow`, `submit_theme_auto`, `revoke_github_link`, `open_url` |
| `settings.vue` | useAppSettings, useKeystore, useUpdater, useSettingsSearch, ConfigRecoveryPanel, PassphrasePrompt, SettingsSearchDropdown, GeneralSection 〜 AboutSection の 8 セクション | `get_config`, `update_config`, `keystore_*`, `list_config_backups`, `restore_config_backup`, `export_profile`, `import_profile`, `list_crash_reports`, `clear_crash_reports`, `submit_crash_reports`, `check_update_is_major_jump` |
| `PanicFlow.vue` (modal) | useNotify | `reset_to_default`, `reset_to_initial` |

### 巨大ファイルの状態 (リファクタ追跡用)

**Rust** (分割完了)

1. ✅ `commands.rs` 1229 行 → `commands/` 10 サブモジュール (うち `cursor_build/` はさらに build / cancel / dto / sign / stream の 5 ファイル分割。`commands/updater` はチャンネル切替 endpoint override を担当)
2. ✅ `cursor.rs` 1289 行 → `cursor/` 5 サブモジュール
3. ✅ `theme.rs` 1255 行 → `theme/` 3 ファイル
4. ✅ `registry.rs` 1020 行 → `registry/` 4 ファイル (mod / scheme / roles / env)
5. ✅ `bulk_import.rs` 703 行 → `bulk_import/` 3 ファイル (mod / assets / cursorpack)

**Vue** 残作業

1. `pages/creator.vue` — Stage 切替 / 17 ロール UI / hotspot drag / build 進捗。**分割候補**: ペインごとの component + composable 抽出を継続
2. `pages/settings.vue` — 8 セクションは既に `components/settings/*Section.vue` に切り出し済
3. `pages/index.vue` — Library 一覧 + フィルタ + sort + Windows scheme + drop import

## 検証ゲート

CI と手元の最終チェック共に `scripts/verify-gate.sh` を使う。順番:

```
cargo fmt --check
cargo clippy --all-targets -D warnings
cargo test --lib
prettier --check
vue-tsc --noEmit
node scripts/check-i18n.mjs
vitest run
```

最後に `npm run tauri:build` を通すと正式リリース可能。

## テスト戦略

### Rust (cargo test --lib)

pure function を中心に層が薄い。主要モジュール:

- `accessibility`, `autostart`, `bulk_import`, `commands`, `crash`, `cursor`, `health`, `hotkey`, `registry`, `theme`
- 重点: パーサー (CUR/ICO/ANI), アーカイブ sanitize (`theme::sanitize_archive_path`), bulk import フロー, role matcher

### Frontend (vitest)

`app/composables/__tests__/` に 15 ファイル:

- `sanitizeSvg.test.ts`, `settingsSearch.test.ts`, `useAniPlayer.test.ts`, `useCreatorAssets.test.ts`, `useCreatorBulkImportFlow.test.ts`, `useCursorpackOpener.test.ts`, `useGithubAuth.test.ts`, `useHotspotDefaults.test.ts`, `useHotspotInteraction.test.ts`, `useI18n.test.ts`, `useMarketplacePreviews.test.ts`, `useMarketplaceSubmit.test.ts`, `useRoleMatcher.test.ts`, `useThemes.test.ts`, `useUpdaterBootstrap.test.ts`
- コンポーネントは `app/components/creator/__tests__/BulkImportPreviewModal.test.ts`

### CI (`.github/workflows/`)

- `ci.yml` — 検証ゲート相当
- `performance.yml` — `benches/cursor_build.rs`, `benches/startup.rs` (Criterion)
- `release.yml` — 署名済みインストーラビルド

Marketplace 投稿パックの検証ワークフロー (`marketplace-validate.yml` / `validate.mjs`) は別リポジトリ [`nishiuriraku/easy-cursor-swap-index`](https://github.com/nishiuriraku/easy-cursor-swap-index) 側に存在する。
