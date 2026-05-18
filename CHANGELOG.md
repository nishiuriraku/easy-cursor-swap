# Changelog

All notable changes to EasyCursorSwap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- `get_app_info` IPC が常に `os_version: "Windows 0.0"` を返していたバグを修正 (`OSVERSIONINFOW::default()` のフィールドゼロのままだった)。`ntdll!RtlGetVersion` 経由でクランプされない真の OS バージョンを返す。
- `export_cursorpack_streamed` の sign / package 段階でビルドを中断したとき `build-progress` イベントに `stage: "cancelled"` が発火されず、Creator の進捗バーが固まる問題を修正。

### Changed

- パニックリセットフロー (`Ctrl+Alt+Shift+R`) が 17 ロール × 40ms の擬似アニメと「snapshot saved」「writing X → Y」「recovery completed in Xms」のハードコード英文ログを擬似的に表示していた挙動を廃止。実 IPC (`reset_to_default` / `reset_to_initial`) を即時呼び出し、完了/失敗のみを正直に表示するシンプルな UI に改修。
- サイドバーのバージョン表記がハードコード `v1.0` だったのを `useAppInfo` 経由で `Cargo.toml` の実バージョンを表示するよう変更。Creator 画面のヒーロー表示 (`CREATOR · v…`) も同様。

### Removed

- サイドバーの「トレイ常駐 · 11.4 MB」ステータス表示を削除 (`trayMemoryMb` props は親から渡されておらず、常にデフォルト値 11.4 の偽データだった)。関連する i18n キー `common.trayResident` も削除。
- 旧パニックフローで使用していた i18n キー `panic.writingProgress` / `panic.targetWindowsDefault` / `panic.targetSnapshot` を削除。代わりに `panic.recoveryStarted` / `panic.recoveryDone` / `panic.recoveryFailed` を追加。

### Internal

- マーケットプレースのフィルタチップ (`All` / `Pixel` / `Minimal` / `Animated` / `Dark`) とレイアウトのスキップナビゲーション (「メインコンテンツへスキップ」) を i18n 化。Creator のデフォルトテーマ名 `'Untitled Theme'` も `creator.untitledThemeName` 経由に。
- `docs/architecture.json` の `useUiTheme` 役割記述、および `docs/ui_map.json` の `titlebar-theme-cycle` インタラクションが「`update_config` IPC を呼ぶ / config に永続化する」と宣言していたが、実際には localStorage のみ操作する設計であるため記述を修正。生成済み HTML (`docs/architecture.html` / `docs/ui_map.html`) も再埋め込み。
- 構造的負債リファクタ (UI 軸監査 Phase 2 + 3 由来) — 6 つの新規 composable に共通パターンを集約:
  - `usePngBlobCache<K, V>` — Map + in-flight Promise + dispose の汎用パターン。`useThemePreviews` / `useMarketplacePreviews` の重複機構を統合 (C20-DUP-001)。
  - `useModalLifecycle` — Teleport modal の body scroll lock (重ね合わせ対応 counter 方式) + Esc 購読 + cleanup を統合。`ThemeDetailModal` / `MarketplaceDetailModal` / `OssLicenseModal` で重複していた ~30 行を解消 (D28-DUP-001)。
  - `useListbox<V>` — `UiSelect.vue` の listbox 状態機械 + キーボードナビ + Teleport 位置計算を分離 (F43-SIZE-001: 528 → 263 行)。
  - `useThemeCardState` — `ThemeCard.vue` / `ThemeRow.vue` の 5 ブロック並行重複を解消 (B9-DUP-001)。
  - `useExternalUrl` (`openExternalUrl`) — `open_url` IPC + `window.open` フォールバックの 6 callsite 重複を 1 行 await に圧縮 (AboutSection / OssLicenseModal / SubmitDeviceFlowModal / marketplace.vue / SubmitThemeDialog ×2 / ThemeDetailDrawer)。
  - `useTagChipInput` — `SubmitThemeDialog` Auto/Manual タブ間の tag chip 入力ロジック重複を解消 (D29 部分)。
- `useThemes` に theme mutation IPC 7 件 (`apply_theme` / `delete_theme` / `duplicate_theme` / `repackage_theme` / `set_theme_favorite` / `inspect_cursorpack` / `import_cursorpack`) のラッパーメソッドを追加。`pages/index.vue` から直接 `invokeTauri` していた経路を composable 経由に集約し、`docs/architecture.json` の `useThemes.ipc_calls` 宣言と実態を一致させた (B8-SIZE-001)。
- composable 総数: 27 → 34 (新規 7 件)。`docs/architecture.json` の `meta.measured_counts.composables` と composable リストを再測定。
- 監査 🔴 のうち `B10-SIZE-001` / `D29-SIZE-001` / `C20-SIZE-001` の純粋な file split を完遂:
  - `SubmitThemeDialog.vue` (576 行) を `SubmitThemeAutoForm.vue` / `SubmitThemeManualForm.vue` の 2 子コンポーネントに分離 (D29-SIZE-001)。`useMarketplaceSubmit` が singleton ではないため submitter は親で保持し、reactive な値を props で子に渡す設計。
  - `ThemeDetailDrawer.vue` (645 行) を `ThemeDetailDrawerHero.vue` / `Strip.vue` / `Footer.vue` の 3 子コンポーネントに分離 (B10-SIZE-001)。activeRole 内部状態は Hero に閉じ、emit はコンテナを通して props down 単方向。
  - `creator.vue` (1269 → 1056 行 / -17%) から `useCreatorMetaState` (メタデータ 6 ref + reset) を抽出、Assign タブ中央エディタを `CreatorEditorCanvas.vue` (~290 行) に切り出し (C20-SIZE-001 部分)。BulkImportPreviewModal の `useBulkImportPreviewState` 抽出は yield が低いため deferred のまま。
- component 総数: 50 → 56 (library +3 / marketplace +2 / creator +1)。`docs/architecture.json` / `docs/ui_map.json` の `measured_counts.components_total` を再測定し、HTML viewer に再埋め込み。

## [0.1.0] - 2026-05-16

### Added

- 初回パブリックリリース。Windows 10 22H2+ / Windows 11 (x64) 対応。ARM64 ターゲットは CI でビルド検証済 (配布は次マイルストーン)。
- ローカルテーマ管理 (Library / Creator)、Marketplace 連携、Tauri Updater + Ed25519 署名 による自動アップデート、パニックリセット (`Ctrl+Alt+Shift+R`)、設定スナップショット / 復元、`.cursorpack` / `.cursorprofile` のインポート / エクスポートなど v0.1.0 仕様の機能を含む。
- HKCU 限定の安全な適用フロー (適用前スナップショット → 失敗時自動ロールバック → 起動時不整合検知)。
- アーカイブ検閲 (path traversal 対策、サイズ上限 50/200/10/1024 MB、image metadata 剥離)。
- Creator 用 Ed25519 鍵管理 (DPAPI 暗号化保存、`.cfkey` import/export は XChaCha20-Poly1305 + Argon2id)。
- マーケットプレース提出フロー (GitHub Device Flow + 自動 PR 作成、署名 / SHA-256 検証は配信側で実施)。

[Unreleased]: https://github.com/nishiuriraku/easy-cursor-swap/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/nishiuriraku/easy-cursor-swap/releases/tag/v0.1.0
