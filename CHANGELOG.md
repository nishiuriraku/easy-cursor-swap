# Changelog

All notable changes to EasyCursorSwap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.2] - 2026-05-19 (pre-release)

v0.0.1 と同じく仮リリース系列 (provisional, SemVer 0.0.x で API 安定保証なし)。Marketplace 提出フローのバグ修正と、Windows カーソルサイズ拡大時に発生していたレジストリ書き込み偽陽性の修正が中心。

### Fixed

- マーケットプレイス『公式インデックスに提出』モーダル (Auto / Manual 両タブ) のテーマ選択ドロップダウンで、`name` が `LocalizedString` マップ形式のテーマ (マーケットプレイス由来テーマなど) に対して `[object Object]` と表示される問題を修正。`MarketplaceName` 型と `pickLocalizedName(name, locale)` composable を通じて現在ロケールのラベルに解決するよう変更し、`themeOptions` を script-side computed として宣言することで unimport の auto-import を確実に発火させた。`entryJson` 側は `th.name` を raw のまま保持し、公式インデックス entry の `LocalizedString` 形式 name と整合 (`FeaturedCard.vue` の `displayName` パターンと一致)。`SubmitThemeDialog.test.ts` に `name: { ja, en }` でマウントして `[object Object]` が出ないことを保証するリグレッションテストを追加。
- Creator の『複製するテーマを選択』モーダル、および マーケットプレイスの『公式インデックスに提出』モーダル (Auto / Manual 両タブ) で、`source/kind` が `marketplace` のテーマを選択肢から除外。マーケットプレイス由来テーマからの派生作品で出所が曖昧になる問題と、公式由来テーマの二重提出を防止する。
- Windows の「マウス ポインターとタッチ」でカーソルサイズを拡大した状態で Library からテーマを「適用」すると、カーソル画像自体は正しく差し替わっているのに `適用に失敗しました: レジストリエラー: SystemParametersInfoW の呼び出しに失敗: ハンドルが無効です。 (0x80070006)` というエラートーストが出る問題を修正。`SPI_SETCURSORS` に同梱される `SPIF_SENDCHANGE` の `WM_SETTINGCHANGE` ブロードキャスト経路で受信側 (シェル / アクセシビリティサービス) のカーネルハンドルがライフサイクル境界で `ERROR_INVALID_HANDLE (6)` を返した場合の偽陽性。レジストリ書き込みは既に完了しているため、既存の `ERROR_INVALID_WINDOW_HANDLE (1400)` と同じく `is_broadcast_false_positive` のホワイトリストに `HRESULT_FROM_WIN32(6) = 0x80070006` を追加し、debug ログ扱いで握りつぶす。ACCESS_DENIED など本物の Win32 エラーは引き続き伝播する。
- マーケットプレイス自動提出 (`submit_theme_auto`) が公式インデックス entry の `author` フィールドに提出者の GitHub username を上書きしていたバグを修正。`theme.json` の `author` フィールドが優先採用されるようになり、第三者が代理提出しても本来の作者クレジットが保持される (Manual タブの `th.author ?? githubUsername.value` と挙動を一致)。`theme.json` に `author` が無い場合のみ提出者 GitHub username にフォールバックする。あわせて公式インデックスリポ ([`easy-cursor-swap-index`](https://github.com/nishiuriraku/easy-cursor-swap-index)) の `scripts/marketplace/validate.mjs` に `.cursorpack` 内 `theme.json` と `entry.author` の一致チェックを追加し、CI で drift を検出するようにした。既存 entry `f2d3825c` (Hamuchi Mouse Cursor) の `author` も `"nishiuriraku"` → `"無ナ"` に修正。

## [0.0.1] - 2026-05-18

仮リリース (provisional first release)。配布チャネル / 署名 / Updater 配線の試走を含む早期版で、機能セットは v0.1.0 計画相当だが、SemVer 上は安定 API を未保証として `0.0.x` にとどめている。

### Added

- 初回パブリックリリース (仮)。Windows 10 22H2+ / Windows 11 (x64) 対応。ARM64 ターゲットは CI でビルド検証済 (配布は次マイルストーン)。
- ローカルテーマ管理 (Library / Creator)、Marketplace 連携、Tauri Updater + Ed25519 署名 による自動アップデート、パニックリセット (`Ctrl+Alt+Shift+R`)、設定スナップショット / 復元、`.cursorpack` / `.cursorprofile` のインポート / エクスポートなどを含む。
- HKCU 限定の安全な適用フロー (適用前スナップショット → 失敗時自動ロールバック → 起動時不整合検知)。
- アーカイブ検閲 (path traversal 対策、サイズ上限 50/200/10/1024 MB、image metadata 剥離)。
- Creator 用 Ed25519 鍵管理 (DPAPI 暗号化保存、`.cfkey` import/export は XChaCha20-Poly1305 + Argon2id)。
- マーケットプレース提出フロー (GitHub Device Flow + 自動 PR 作成、署名 / SHA-256 検証は配信側で実施)。

### Fixed

- `get_app_info` IPC が常に `os_version: "Windows 0.0"` を返していたバグを修正 (`OSVERSIONINFOW::default()` のフィールドゼロのままだった)。`ntdll!RtlGetVersion` 経由でクランプされない真の OS バージョンを返す。
- `export_cursorpack_streamed` の sign / package 段階でビルドを中断したとき `build-progress` イベントに `stage: "cancelled"` が発火されず、Creator の進捗バーが固まる問題を修正。
- メインウィンドウの閉じるボタン押下時に Tauri ランタイムがそのままプロセス終了してしまい、トレイ常駐が機能していなかった問題を修正。`tauri::Builder::build()` + `App::run()` に切り替え、`RunEvent::ExitRequested { code: None, .. }` (ユーザー操作起点) のみ `api.prevent_exit()` で抑止することで、tray メニュー「終了」(`AppHandle::exit(0)` 経由 = `code: Some(0)`) と `AppHandle::restart` の終了経路は素通ししつつ、close ボタンでは WebView 破棄 + トレイ常駐 (Phase 4-1 メモリ最適化の設計意図) を回復。グローバルパニックホットキー (`Ctrl+Alt+Shift+R`) と `cursor_watcher`、`auto_start` のサイレント常駐モードが GUI クローズ後も継続して機能する。あわせて設定画面の自動起動説明文に残っていた dark-mode 連動の文言 (`(ダークモード自動切替を有効化)` / `(enables dark-mode auto-switch)`) を削除。
- トレイ常駐 (close ボタンによる `window.destroy()`) から「EasyCursorSwap を開く」「ダブルクリック」「`Ctrl+Alt+Shift+R`」「2 重起動」で WebView を再生成した際、`tray::show_or_recreate_main_window` の `WebviewWindowBuilder` が `tauri.conf.json` の `app.windows[0]` と乖離しており、ネイティブの Windows タイトルバー (`decorations(true)`) が `AppTitlebar.vue` の上に重なって描画され、ウィンドウサイズも 1100×750 (本来 1280×820) に縮んでいた問題を修正。再生成時のプロパティを conf.json と一致させ、`decorations(false)` のフレームレス前提を回復した。

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
- composable 総数: 28 → 36 (新規 8 件)。`docs/architecture.json` の `meta.measured_counts.composables` と composable リストを再測定。
- 監査 🔴 のうち `B10-SIZE-001` / `D29-SIZE-001` / `C20-SIZE-001` の純粋な file split を完遂:
  - `SubmitThemeDialog.vue` (576 行) を `SubmitThemeAutoForm.vue` / `SubmitThemeManualForm.vue` の 2 子コンポーネントに分離 (D29-SIZE-001)。`useMarketplaceSubmit` が singleton ではないため submitter は親で保持し、reactive な値を props で子に渡す設計。
  - `ThemeDetailDrawer.vue` (645 行) を `ThemeDetailDrawerHero.vue` / `Strip.vue` / `Footer.vue` の 3 子コンポーネントに分離 (B10-SIZE-001)。activeRole 内部状態は Hero に閉じ、emit はコンテナを通して props down 単方向。
  - `creator.vue` (1269 → 1056 行 / -17%) から `useCreatorMetaState` (メタデータ 6 ref + reset) を抽出、Assign タブ中央エディタを `CreatorEditorCanvas.vue` (~290 行) に切り出し (C20-SIZE-001 部分)。
  - `BulkImportPreviewModal.vue` (579 → 297 行 / -49%) から `useBulkImportPreviewState` を抽出。matches/unmatched の三方移動 state machine + props.open 連動の初期マッチ watch + Blob URL ライフサイクル + ApplyPayload 組立を composable に閉じ込め、SFC は presentation に専念 (audit C21-SIZE 部分)。`ApplyPayload` 型の output 場所も SFC から composable に移動 (`useCreatorBulkImportFlow` 側 import を更新)。
- component 総数: 50 → 56 (library +3 / marketplace +2 / creator +1)。`docs/architecture.json` / `docs/ui_map.json` の `measured_counts.components_total` を再測定し、HTML viewer に再埋め込み。

[Unreleased]: https://github.com/nishiuriraku/easy-cursor-swap/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/nishiuriraku/easy-cursor-swap/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/nishiuriraku/easy-cursor-swap/releases/tag/v0.0.1
