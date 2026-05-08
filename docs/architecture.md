# EasyCursorSwap アーキテクチャマップ

> 最終更新: 2026-05-08
>
> ファイルの責務を一望するための **生きたインデックス**。実装が増減したら本書を更新する。
> 仕様の正本は [`first_plan.md`](./first_plan.md)、進行中タスクは [`implementation_plan.md`](./implementation_plan.md)。
> 本書はリファクタや初見オンボード時に「どのモジュールがどう繋がっているか」を素早く掴むためのもの。

## 全体像

```
┌─────────────────────────────────────────────────────────────────┐
│ Vue / Nuxt 4 SPA  (app/)                                        │
│   pages → components → composables → useTauri.invoke()           │
└───────────────────────┬─────────────────────────────────────────┘
                        │ IPC (Tauri 2 / serde)
┌───────────────────────▼─────────────────────────────────────────┐
│ Rust バックエンド  (src-tauri/src/)                              │
│   commands.rs (49 IPC 受け口)                                    │
│   ├─ config / theme / cursor / registry  ← Source of Truth      │
│   ├─ marketplace / keystore / bulk_import                       │
│   └─ tray / darkmode / hotkey / health / crash                  │
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

## Rust 側モジュール (`src-tauri/src/`)

合計 22 モジュール / 約 9,300 行。`lib.rs` で全モジュールを `pub mod` 公開し、`main.rs` から `tauri::Builder` に組み込む。

### 責務マップ

| カテゴリ | モジュール | 主な役割 | 行数 |
|---|---|---|---|
| **IPC 表玄関** | `commands.rs` | 49 個の `#[tauri::command]` を `tauri::generate_handler!` に登録 | 1229 |
| **設定 / 状態** | `config.rs` | `AppConfig` / `ConfigManager` (RwLock + 自動 schema migration + バックアップ) | 399 |
| | `errors.rs` | `AppError` / `AppResult` 共通型 | 75 |
| **カーソル生成パイプライン** | `cursor.rs` | PNG/SVG → `.cur` バイナリ生成 (Lanczos/Nearest, 6 サイズ, hotspot, ANI/CUR/ICO 解析) | 1289 |
| | `cursor_watcher.rs` | コントロールパネル等で外部に書き換えられたら `cursor-changed` を発火 | 114 |
| **レジストリ操作** | `registry.rs` | `HKCU\Control Panel\Cursors` 適用/復元、Scheme 列挙、トランザクションログ、初期スナップショット | 1020 |
| **テーマパッケージ** | `theme.rs` | `.cursorpack` の作成 / 解凍 / バリデーション、theme.json 管理、孤児カーソル復旧 | 1255 |
| | `bulk_import.rs` | ファイル / フォルダ / cursorpack の一括取り込み (リサンプル並列化, キャンセル可能) | 703 |
| | `backup.rs` | `.cursorprofile` (config + 全テーマの ZIP) の export/import | 295 |
| **マーケットプレース** | `marketplace.rs` | HTTP インデックス取得、SHA-256 + Ed25519 検証、ダウンロードサイズ上限 | 279 |
| | `keystore.rs` | クリエイター用 Ed25519 鍵ペア (DPAPI 暗号化), `.cfkey` import/export, key_id 計算 | 415 |
| **信頼性 / 復旧** | `health.rs` | 起動連続失敗カウンタ + ロールバック対象バージョン算出 | 209 |
| | `crash.rs` | panic フック + `crash-reports/` ディレクトリの retention | 269 |
| **OS 統合** | `tray.rs` | システムトレイ常駐 / メインウィンドウ再生成 | 162 |
| | `darkmode.rs` | OS テーマ変更監視 → light/dark テーマ自動切替 | 148 |
| | `hotkey.rs` | グローバルホットキー (Ctrl+Alt+Shift+R 等) | 258 |
| | `autostart.rs` | `HKCU\...\Run` 自動起動レジストリ管理 | 274 |
| | `single_instance.rs` | Named Mutex による多重起動防止 + 既存インスタンスへ表示要求 | 171 |
| | `appusermodel.rs` | AppUserModelID 登録 (Win トースト発信元) | 40 |
| | `accessibility.rs` | マウスソナー / ハイコントラスト / カーソル拡大の検出 | 140 |
| | `environment.rs` | RDP / Citrix / Sandbox など動作対象外環境の検出 | 115 |
| **観測** | `logging.rs` | tracing 初期化 + `redact_path` / `short_hash` PII helper | 158 |

### IPC 一覧 (49 commands)

`commands.rs::get_command_handlers!` に登録される 49 個。フロントは `app/composables/useTauri.ts::invokeTauri<T>(name, args)` で呼ぶ。

| カテゴリ | コマンド名 |
|---|---|
| テーマ | `get_themes`, `apply_theme`, `delete_theme`, `duplicate_theme`, `repackage_theme`, `get_theme_previews`, `get_cursor_roles`, `get_current_cursors` |
| .cursorpack | `inspect_cursorpack`, `import_cursorpack`, `export_cursorpack`, `export_cursorpack_streamed`, `cancel_build`, `build_cursor_file` |
| 鍵管理 | `keystore_info`, `keystore_generate`, `keystore_delete`, `keystore_export`, `keystore_import` |
| カーソルファイル | `import_cursor_file`, `inspect_ani_file` |
| マーケットプレース | `marketplace_fetch_index`, `marketplace_install` |
| プロファイル | `export_profile`, `import_profile` |
| Windows スキーム | `list_windows_schemes`, `apply_windows_scheme`, `get_windows_scheme_previews`, `export_windows_scheme_as_cursorpack` |
| システム / 設定 | `get_config`, `update_config`, `get_dark_mode_status`, `get_environment_report`, `get_app_info`, `get_autostart_status`, `get_accessibility_conflicts`, `list_config_backups`, `restore_config_backup`, `check_update_is_major_jump`, `open_url`, `clear_cursor_cache`, `reset_to_default`, `reset_to_initial`, `list_crash_reports`, `clear_crash_reports` |
| 一括取込 | `bulk_resolve_assets`, `cancel_bulk_import`, `parse_cursorpack_for_creator` |

### 起動シーケンス (`main.rs`)

1. panic フック設置 → 古いクラッシュレポート掃除
2. tracing logger 初期化 (フォールバック有り)
3. **起動ヘルスチェック** (`StartupCheck::begin`) — 連続 3 回失敗ならロールバック誘導ダイアログ
4. AppUserModelID 登録
5. **多重起動防止** (`SingleInstanceLock::acquire`)
6. `ConfigManager::init` (ここで失敗したら専用ダイアログでバックアップ場所を案内し終了)
7. autostart レジストリを config に追従
8. 初回スナップショット保存 (`_initial_snapshot.json`)
9. 孤児カーソル参照のクリーンアップ (`ThemeManager::cleanup_orphan_references`)
10. **クラッシュリカバリ**: `_pending_apply.snapshot` が残っていれば前回の適用処理が中断 → Windows 既定へ復元
11. `tauri::Builder` を組み立て、トレイ・ダークモード監視・パニックホットキー・カーソルウォッチャを `setup` で起動
12. 全部成功 → `mark_healthy` (失敗カウンタをリセット)

## フロントエンド (`app/`)

合計 約 12,200 行。Nuxt 4 SPA。`pathPrefix: false` のため component はファイル名 (basename) で参照する。

### ディレクトリ構成

```
app/
├─ pages/          ← トップレベル画面 (Library / Creator / Marketplace / Settings / Appearance)
├─ layouts/        ← default.vue (シェル: AppTitlebar + AppSidebar + slot)
├─ components/
│  ├─ shell/       ← AppTitlebar, AppSidebar, EnvironmentBanner
│  ├─ library/     ← ThemeCard, ThemeRow, ThemeDetailModal, ApplyModal, ImportConflictDialog, ThemePickerModal, CursorMatrix, ThemeDetailDrawer
│  ├─ creator/     ← BulkImportButton, BulkImportPreviewModal, BulkImportRoleRow, NewThemeStartModal, CreatorStartScreen, SizeStrip
│  ├─ marketplace/ ← MarketplaceCard, FeaturedCard, SubmitThemeDialog
│  ├─ settings/    ← SettingsRow, ConfigRecoveryPanel, ModeIndicator, PassphrasePrompt, PairingSlot
│  ├─ panic/       ← PanicFlow (Stage 1 / Stage 2 リカバリ)
│  ├─ ui/          ← UiSelect (ネイティブ select の白背景を回避)
│  └─ icons/       ← UiIcons / CursorIcons (render-function ベースで v-html を使わない)
├─ composables/    ← useThemes, useAppSettings, useI18n, useTauri (IPC), useKeystore, useUiTheme,
│                    useRoleMatcher, useThemePreviews, useBulkImport, useUpdater, useNotify,
│                    sanitizeSvg, useCreatorAssets
├─ types/          ← config.ts, theme.ts, marketplace.ts (Rust struct と 1:1)
├─ locales/        ← ja.ts, en.ts (CI で parity チェック)
├─ assets/css/     ← global.css (デザイントークン / Win11 chrome / Glassmorphism)
└─ plugins/        ← clickOutside.client.ts
```

### Page → Composable → IPC の主な経路

| Page | 主な composable / Component | 主な IPC |
|---|---|---|
| `index.vue` (Library) | useThemes, ThemeCard / ThemeRow, ApplyModal, ImportConflictDialog, ThemePickerModal | `get_themes`, `apply_theme`, `import_cursorpack`, `inspect_cursorpack`, `delete_theme`, `duplicate_theme`, `repackage_theme`, `list_windows_schemes`, `get_windows_scheme_previews` |
| `creator.vue` | useCreatorAssets, useRoleMatcher, useBulkImport, useKeystore, sanitizeSvg, NewThemeStartModal, BulkImportPreviewModal | `import_cursor_file`, `inspect_ani_file`, `parse_cursorpack_for_creator`, `bulk_resolve_assets`, `build_cursor_file`, `export_cursorpack_streamed`, `cancel_build`, `keystore_info` |
| `marketplace.vue` | useThemes, MarketplaceCard, FeaturedCard, SubmitThemeDialog | `marketplace_fetch_index`, `marketplace_install`, `keystore_info` |
| `settings.vue` | useAppSettings, useKeystore, useUpdater, ConfigRecoveryPanel, PassphrasePrompt | `get_config`, `update_config`, `keystore_*`, `list_config_backups`, `restore_config_backup`, `export_profile`, `import_profile`, `list_crash_reports`, `clear_crash_reports`, `check_update_is_major_jump` |
| `appearance.vue` | useAppSettings, ModeIndicator, PairingSlot, ThemePickerModal | `get_dark_mode_status`, `update_config` |
| `PanicFlow.vue` (modal) | useNotify | `reset_to_default`, `reset_to_initial` |

### 巨大ファイルの状態 (リファクタ追跡用)

**Rust top 5**

1. `cursor.rs` 1289 行 — 画像処理 + ICO/CUR/ANI parse。**分割候補**: image_resize / cur_build / ico_cur_parse / ani_parse
2. `theme.rs` ~1255 行 — 大きい `impl ThemeManager`。**分割候補**: 検証, 保存/読込, 一覧, 削除/複製
3. `commands.rs` 1229 行 — 49 IPC を 1 ファイルに集約。**分割候補**: theme / keystore / cursorpack / system / windows_scheme
4. `registry.rs` 1020 行 — Scheme / Cursors / Snapshot / SPI を兼任。**分割候補**: scheme / cursors_apply / snapshot
5. `bulk_import.rs` 703 行 — 並列パイプ + cancel。比較的責務が単一。

**Vue top 3**

1. `pages/creator.vue` ~1500 行 — Stage 切替 / 17 ロール UI / hotspot drag / build 進捗。**分割候補**: ペインごとの component + composable 抽出
2. `pages/settings.vue` ~1060 行 — 8 セクション (general / startup / library / security / keys / logging / updates / about + recovery)
3. `pages/index.vue` ~1060 行 — Library 一覧 + フィルタ + sort + Windows scheme + drop import

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

72 テスト。pure function を中心に層が薄い。主要モジュール:

- `accessibility`, `autostart`, `bulk_import`, `commands`, `crash`, `cursor`, `health`, `hotkey`, `registry`, `theme`
- 重点: パーサー (CUR/ICO/ANI), アーカイブ sanitize (`theme::sanitize_archive_path`), bulk import フロー, role matcher

### Frontend (vitest)

- `app/composables/__tests__/useCreatorAssets.test.ts`
- `app/composables/__tests__/useRoleMatcher.test.ts` (28 ケースで日本語ファイル名 / フォルダ階層 / collision を網羅)

### CI (`.github/workflows/`)

- `ci.yml` — 検証ゲート相当
- `performance.yml` — `benches/cursor_build.rs`, `benches/startup.rs` (Criterion)
- `marketplace-validate.yml` — `scripts/marketplace/validate.mjs` で PR 提出されたパックを検証
- `release.yml` — 署名済みインストーラビルド
