# EasyCursorSwap ファイル機能インベントリ (2026-05-08 時点)

> [!NOTE]
> このドキュメントは v1.0 リリース直前の **コードベース構造のスナップショット** です。
> 計画と実装の突合結果と、次の方針案を併記しています。
>
> - 計画と実装の追従は [`implementation_plan.md`](implementation_plan.md) が真値。
> - 凍結された原要件は [`first_plan.md`](first_plan.md)。
> - このファイルは大きな構造変更のタイミング（モジュール分割・統合等）でのみ更新。

---

## 1. Rust バックエンド (`src-tauri/src/`)

### 1-1. エントリポイント / IPC ハブ

| ファイル | 機能 |
|---|---|
| [main.rs](../src-tauri/src/main.rs) | Tauri アプリのエントリ。`AppUserModelID` 登録、`StartupCheck::begin()`、panic フック、ダークモード自動切替、ホットキー登録 |
| [lib.rs](../src-tauri/src/lib.rs) | 22 モジュールの `pub mod` 宣言 + Tauri builder 構築（`get_command_handlers` で IPC 登録、State に `ConfigManager`） |
| [commands/mod.rs](../src-tauri/src/commands/mod.rs) | 全 Tauri コマンドのハンドラ登録（44 IPC） |
| [errors.rs](../src-tauri/src/errors.rs) | `AppError`（`thiserror`、`Serialize` 派生で IPC 経由の throw 対応） |

### 1-2. IPC コマンド実体（責務別に分割済み — 9 ファイル）

| ファイル | 主な IPC（合計 44 個） |
|---|---|
| [commands/cursor_build.rs](../src-tauri/src/commands/cursor_build.rs) | `build_cursor_file` / `export_cursorpack` / `export_cursorpack_streamed` / `cancel_build`（進捗イベント `build-progress`） |
| [commands/cursor_io.rs](../src-tauri/src/commands/cursor_io.rs) | `import_cursor_file`（.cur/.ico → PNG）/ `inspect_ani_file`（RIFF 解析・プレビュー） |
| [commands/theme.rs](../src-tauri/src/commands/theme.rs) | `get_cursor_roles` / `get_current_cursors` / `get_themes` / `get_theme_previews` / `apply_theme` / `clear_cursor_cache` / `inspect_cursorpack` / `import_cursorpack` / `delete_theme` / `duplicate_theme` / `repackage_theme` |
| [commands/system.rs](../src-tauri/src/commands/system.rs) | `reset_to_default` / `reset_to_initial` / `get_dark_mode_status` / `get_environment_report` / `get_config` / `update_config` / `get_autostart_status` / `get_app_info` / `list_config_backups` / `restore_config_backup` / `open_url` / `get_accessibility_conflicts` / `check_update_is_major_jump` / `list_crash_reports` / `clear_crash_reports` |
| [commands/keystore.rs](../src-tauri/src/commands/keystore.rs) | `keystore_info` / `keystore_generate` / `keystore_delete` / `keystore_export` / `keystore_import` |
| [commands/marketplace.rs](../src-tauri/src/commands/marketplace.rs) | `marketplace_fetch_index` / `marketplace_install` |
| [commands/profile.rs](../src-tauri/src/commands/profile.rs) | `export_profile` / `import_profile` |
| [commands/windows_scheme.rs](../src-tauri/src/commands/windows_scheme.rs) | `get_windows_scheme_previews` / `export_windows_scheme_as_cursorpack` / `list_windows_schemes` / `apply_windows_scheme` |
| [bulk_import/](../src-tauri/src/bulk_import/) | `bulk_resolve_assets` / `cancel_bulk_import` / `parse_cursorpack_for_creator` |

### 1-3. ドメイン / 機能モジュール

| ファイル | 機能 |
|---|---|
| [config.rs](../src-tauri/src/config.rs) | `AppConfig` の RwLock + schema_version + `config.bak.v{N}.json` 退避 + `config.corrupt.{epoch}.json` |
| [registry/mod.rs](../src-tauri/src/registry/mod.rs) | `HKCU\Control Panel\Cursors` 読み書き、`SPI_SETCURSORS` / `SPI_SETCURSORSHADOW`、トランザクション + `_pending_apply.snapshot` |
| [registry/roles.rs](../src-tauri/src/registry/roles.rs) | 17 役割の絶対パス書き込みロジック、`compute_apply_values` 純粋関数 |
| [registry/scheme.rs](../src-tauri/src/registry/scheme.rs) | `Schemes` への REG_EXPAND_SZ 登録、`build_scheme_value` / `sanitize_scheme_name` |
| [registry/env.rs](../src-tauri/src/registry/env.rs) | レジストリ環境変数操作補助 |
| [cursor/cur_build.rs](../src-tauri/src/cursor/cur_build.rs) | PNG → .cur マルチ解像度 6 サイズパッキング、ホットスポット書込み |
| [cursor/image.rs](../src-tauri/src/cursor/image.rs) | Lanczos / Nearest リサイズ + ドット絵自動判定 + `RESIZE_CACHE`（64 エントリ FIFO）+ `strip_png_metadata` |
| [cursor/ani.rs](../src-tauri/src/cursor/ani.rs) | ANI（RIFF/ACON）パーサー + `ParsedAni::total_duration_ms`（検査専用） |
| [cursor/ico_cur.rs](../src-tauri/src/cursor/ico_cur.rs) | .ico/.cur 解析（ICONDIRENTRY + PNG/BMP DIB）+ `pick_largest_as_png` |
| [cursor_watcher.rs](../src-tauri/src/cursor_watcher.rs) | `WM_SETTINGCHANGE` 購読（不可視ウィンドウ）→ `cursor-changed` イベント |
| [theme/mod.rs](../src-tauri/src/theme/mod.rs) | `ThemeManager`：`.cursorpack` 入出力、`apply_theme`、`cleanup_orphan_references`、Ed25519 署名埋込 |
| [theme/sanitize.rs](../src-tauri/src/theme/sanitize.rs) | `sanitize_archive_path` + Zip 爆弾対策（50/200/10 MB 三段階）+ `S_IFLNK` 拒否 |
| [theme/types.rs](../src-tauri/src/theme/types.rs) | `ThemeMeta` / `ThemeSummary` / `LocalizedString` 等 |
| [bulk_import/assets.rs](../src-tauri/src/bulk_import/assets.rs) | 複数ファイル/フォルダ走査 → `ResolvedAsset` 変換、ファジーマッチ |
| [bulk_import/cursorpack.rs](../src-tauri/src/bulk_import/cursorpack.rs) | `.cursorpack` 読込（ライブラリ非依存、creator 直挿入用） |
| [backup.rs](../src-tauri/src/backup.rs) | `.cursorprofile` Zip 入出力、`ProfileEnvelope`、merge/overwrite |
| [marketplace.rs](../src-tauri/src/marketplace.rs) | `MarketplaceClient`：reqwest(rustls) + SHA-256 + Ed25519 + `historical_keys` ローテーション + 50MB ガード |
| [keystore.rs](../src-tauri/src/keystore.rs) | DPAPI 暗号化保存、`generate`/`sign`/`verify`、XChaCha20-Poly1305 + Argon2id `.cfkey` 入出力 |
| [health.rs](../src-tauri/src/health.rs) | `startup.json` の `pending_failures`、3 回連続失敗検知、バージョン変更で自動リセット |
| [crash.rs](../src-tauri/src/crash.rs) | `install_panic_hook`、`%LOCALAPPDATA%\...\crash\panic-{epoch}.json`、`prune_old_reports`、`general.crash_reporting` 同意 |
| [tray.rs](../src-tauri/src/tray.rs) | システムトレイ + `show_or_recreate_main_window`（WebView 破棄/再生成） |
| [darkmode.rs](../src-tauri/src/darkmode.rs) | `AppsUseLightTheme` 監視 + `WM_SETTINGCHANGE`（不可視ウィンドウ） |
| [hotkey.rs](../src-tauri/src/hotkey.rs) | `RegisterHotKey` で `Ctrl+Alt+Shift+R` → `panic-hotkey` イベント |
| [autostart.rs](../src-tauri/src/autostart.rs) | `HKCU\...\Run` 登録、MSIX 検出時は no-op で `startupTask` に委譲 |
| [single_instance.rs](../src-tauri/src/single_instance.rs) | Named Mutex + CreateEvent シグナル方式、第二インスタンスが `ShowWindow` を発火 |
| [appusermodel.rs](../src-tauri/src/appusermodel.rs) | `SetCurrentProcessExplicitAppUserModelID("dev.easycursorswap.app")` |
| [accessibility.rs](../src-tauri/src/accessibility.rs) | `CursorIndicator` / `ContrastScheme` / `CursorBaseSize` 競合検出 |
| [environment.rs](../src-tauri/src/environment.rs) | RDP / Citrix / Server 検出 (`SM_REMOTESESSION` + `InstallationType`) |
| [logging.rs](../src-tauri/src/logging.rs) | `tracing-appender` 日次ローテ + 14 日 + 100MB 上限 + `redact_path` / `short_hash` |

### 1-4. ベンチ / テスト

| ファイル | 機能 |
|---|---|
| [benches/cursor_build.rs](../src-tauri/benches/cursor_build.rs) | Lanczos / Nearest + 6 サイズ .cur ビルド計測（cold / warm パス） |
| [benches/startup.rs](../src-tauri/benches/startup.rs) | 起動時間 / 常駐メモリ目標値検証 |

---

## 2. フロントエンド (`app/`)

### 2-1. ページ（5 画面）

| ファイル | 役割 |
|---|---|
| [pages/index.vue](../app/pages/index.vue) | テーマライブラリ（grid/list/タグフィルタ/インポート/D&D） |
| [pages/creator.vue](../app/pages/creator.vue) | クリエイターモード（3 カラム + バルクインポート + .cursorpack 出力） |
| [pages/marketplace.vue](../app/pages/marketplace.vue) | 公式インデックス（フィルタ/検索/Featured/PR 提出） |
| [pages/settings.vue](../app/pages/settings.vue) | 設定 8 セクション（一般/起動/ライブラリ/セキュリティ/鍵/ログ/更新/About） |
| [pages/appearance.vue](../app/pages/appearance.vue) | ダークモード連動ペアリング |

### 2-2. コンポーネント（責務別グループ）

| グループ | 主要ファイル |
|---|---|
| [shell/](../app/components/shell/) | `AppTitlebar` / `AppSidebar` / `AppStatusbar` / `EnvironmentBanner` |
| [library/](../app/components/library/) | `ThemeCard` / `ApplyModal` / `CursorMatrix` / `ImportConflictDialog` / `ThemePickerModal` / `LibraryFilterBar` / `ThemeDetailDrawer` ほか 12 個 |
| [creator/](../app/components/creator/) | `CreatorToolbar` / `CreatorRoleList` / `RoleListItem` / `SizeStrip` / `BulkImport*` 3 種 / `CreatorMetadataPane` / `CreatorPropertiesPane` ほか 11 個 |
| [marketplace/](../app/components/marketplace/) | `FeaturedCard` / `MarketplaceCard` / `SubmitThemeDialog` |
| [settings/](../app/components/settings/) | `GeneralSection` / `StartupSection` / `LibrarySection` / `SecuritySection` / `KeysSection` / `LoggingSection` / `UpdatesSection` / `AboutSection` / `PassphrasePrompt` / `ConfigRecoveryPanel` / `PairingSlot` / `ModeIndicator` ほか 13 個 |
| [panic/](../app/components/panic/) | `PanicFlow`（ステージ選択 + ライブログ + 17 ロールグリッド） |
| [icons/](../app/components/icons/) | `UiIcon` + `UI_ICONS`(25)、`CursorIcon` + `CURSOR_ICONS`(17) — render 関数で v-html 回避 |
| [ui/](../app/components/ui/) | `UiSelect` |

### 2-3. Composables（13 個）

| ファイル | 役割 |
|---|---|
| [useTauri.ts](../app/composables/useTauri.ts) | `invokeTauri` IPC ラッパー（Web 開発時フォールバック） |
| [useThemes.ts](../app/composables/useThemes.ts) | テーマ一覧の共有リアクティブ singleton |
| [useAppSettings.ts](../app/composables/useAppSettings.ts) | `get_config`/`update_config` + dirty フラグ |
| [useI18n.ts](../app/composables/useI18n.ts) | `t(key, params)` + フォールバック + OS ロケール検出 |
| [useKeystore.ts](../app/composables/useKeystore.ts) | 鍵生成/削除/エクスポート/インポート + key_id 表示 |
| [useUiTheme.ts](../app/composables/useUiTheme.ts) | アプリ自体の light/dark 切替 |
| [useRoleMatcher.ts](../app/composables/useRoleMatcher.ts) | エイリアス辞書 + `scoreRole` + `resolveCollisions` |
| [useThemePreviews.ts](../app/composables/useThemePreviews.ts) | プレビュー画像取得（ロール×サイズ） |
| [useBulkImport.ts](../app/composables/useBulkImport.ts) | バルクインポート IPC ラッパー + 進捗購読 |
| [useCreatorAssets.ts](../app/composables/useCreatorAssets.ts) | `assignedPng` / `Hotspot` 統合管理 |
| [useUpdater.ts](../app/composables/useUpdater.ts) | check / downloadAndInstall / relaunch |
| [useNotify.ts](../app/composables/useNotify.ts) | Toast 通知（permission キャッシュ） |
| [sanitizeSvg.ts](../app/composables/sanitizeSvg.ts) | SVG サニタイズ（`<script>`/`href`/`on*`/`javascript:` 除去） |

### 2-4. その他

| ファイル | 役割 |
|---|---|
| [locales/ja.ts](../app/locales/ja.ts), [en.ts](../app/locales/en.ts) | 288 キー（CI で parity 検証） |
| [types/config.ts](../app/types/config.ts), [theme.ts](../app/types/theme.ts), [marketplace.ts](../app/types/marketplace.ts) | IPC ペイロード型 |
| [assets/css/global.css](../app/assets/css/global.css) | デザイントークン + Win11 chrome + Glassmorphism（Tailwind 不使用） |
| [layouts/default.vue](../app/layouts/default.vue) | サイドバー連動レイアウト + `panic-hotkey` 購読 |

---

## 3. インフラ / 周辺

| ディレクトリ | 役割 |
|---|---|
| [services/crash-report-worker/](../services/crash-report-worker/) | Cloudflare Workers + KV（dedup / rate limit）— **未デプロイ** |
| [scripts/check-i18n.mjs](../scripts/check-i18n.mjs) | ja/en parity CI |
| [scripts/marketplace/validate.mjs](../scripts/marketplace/validate.mjs) | スキーマ + SHA-256 + Ed25519 + VirusTotal v3 |
| [scripts/marketplace/malware-hashes.txt](../scripts/marketplace/malware-hashes.txt) | マルウェアハッシュ DB |
| [.github/workflows/](../.github/workflows/) | `ci` / `performance` / `release` / `marketplace-validate` の 4 本 |

---

## 4. 計画との突合 — 現在地

### 観察

- `first_plan.md` の v1.0 MVP スコープ（ライブプレビュー無し / `.ani` 新規生成不可 / Undo 無し / 自動切替はダークモードのみ）は **意図的な制約として完全に遵守**。
- 計画から **進んだ点**：Phase 9（マーケットプレイス）が当初予定より先行実装され、HTTP/SHA-256/Ed25519/historical_keys/ZIP 展開まで実装済み。
- 計画から **追加された点**：バルクインポート機能は `first_plan.md` に未記載で、2026-05-07 の追加スコープ。

### 完了 (✅)

Phase 1-4（基盤・Rust コア・画像・トレイ）/ Phase 5（UI 7 画面 + ARIA）/ Phase 6（パッケージ・セキュリティ・Ed25519・cursorprofile）/ Phase 7-2（通知）/ Phase 7-3（i18n）/ Phase 8-1（ベンチ）/ Phase 8-4（Updater）/ Phase 8-5（CI）/ Phase 9-2/9-3/9-4/9-5

### 残タスク（コード変更のみで完結するもの）

| # | タスク | 該当 Phase | 着手難度 |
|---|---|---|---|
| 1 | **WCAG AA コントラスト比 4.5:1 実測**（axe / Lighthouse の CI 組込 + トークン調整） | 5-11 | 中 |
| 2 | **クラッシュレポート閲覧/削除 UI**（IPC 配線済 → 設定 → ログに表示） | 7-1 続 | 小 |
| 3 | **クラッシュレポート送信クライアント**（`submit_pending_reports` IPC 新設 + endpoint/app_token 入力欄） | 7-1 続 | 中 |
| 4 | **バルクインポート手動 e2e 検証** | 2026-05-07 追加 | 検証のみ |

### 残タスク（外部依存）

EV/OV コードサイニング実申請 / SmartScreen レピュテーション / MS Store 申請 / Cloudflare Worker 実デプロイ

---

## 5. 次の方針案

### 案 A: クラッシュレポート閉ループ完成（推奨）

**1 機能 = 1 コミット ルールに最も馴染む。** タスク #2 → #3 を順に：
1. 設定 → ログセクションに **クラッシュレポート一覧 + 「履歴を消去」** UI を追加（IPC は配線済み、UI のみ）
2. `submit_pending_reports` IPC 新設 + `endpoint` / `app_token` 入力欄追加 + 送信成功ファイル削除

**メリット**：v1.0 の **DOD 未達項目を 1 件潰せる**。Worker デプロイは別フェーズだが、クライアント側は完成する。

### 案 B: WCAG AA 実測 CI

**v1.0 の "CI でグリーン" を満たすため**に、Lighthouse-CI または axe-core を `.github/workflows/ci.yml` に組込む。違反箇所を `global.css` のトークンで調整。

**メリット**：1 コミットで Phase 5-11 完了。**デメリット**：実装コスト中、デザインリファクタが波及する可能性。

### 案 C: バルクインポート手動 e2e 検証 + 微修正

[docs/superpowers/issue/2026-05-07-bulk-import-manual-e2e.md](superpowers/issue/2026-05-07-bulk-import-manual-e2e.md) に従って手動検証 → 出てきた issue を修正。

**メリット**：直近実装した機能の品質が確定。**デメリット**：手動操作が必要で対話セッション向き。

### 推奨順

**案 A（クラッシュレポート閉ループ）→ 案 B（WCAG）→ 案 C（バルク e2e）の順**：

- 案 A は IPC 既配線なので **設定 UI 1 コンポーネント追加だけ** で 1 コミット切れる（小さい）
- 続けて送信クライアントを **2 コミット目** に切る — `endpoint` / `app_token` の config 永続化と `submit_pending_reports` IPC
- これで v1.0 DOD が **コードレベルでは完全終了** し、残るは外部依存のみという明確な状態になる

---

## 統計（2026-05-08 時点）

| 指標 | 値 |
|---|---|
| Rust モジュール数 | 22 + ベンチ 2 |
| Tauri IPC コマンド数 | 44 |
| Vue ページ数 | 5 |
| Vue コンポーネント数 | 約 60（responsive group 別） |
| Composables 数 | 13 |
| i18n キー数 | 288（ja-en parity） |
| CI ワークフロー数 | 4 |
