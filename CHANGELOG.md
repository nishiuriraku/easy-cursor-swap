# Changelog

All notable changes to EasyCursorSwap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

_(none)_

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
