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

- マーケットプレースのフィルタチップ (`All` / `Pixel` / `Minimal` / `Animated` / `Dark`) とレイアウトのスキップナビゲーション (「メインコンテンツへスキップ」) を i18n 化。
- `docs/architecture.json` の `useUiTheme` 役割記述、および `docs/ui_map.json` の `titlebar-theme-cycle` インタラクションが「`update_config` IPC を呼ぶ / config に永続化する」と宣言していたが、実際には localStorage のみ操作する設計であるため記述を修正。生成済み HTML (`docs/architecture.html` / `docs/ui_map.html`) も再埋め込み。

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
