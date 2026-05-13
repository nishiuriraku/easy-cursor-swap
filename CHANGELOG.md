# Changelog

All notable changes to EasyCursorSwap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Pre-release status:** EasyCursorSwap is currently at `0.1.0` and has **not yet had a tagged release**. The `[Unreleased]` section captures changes accumulating toward the first public release. The `0.x` series may contain breaking changes; once `1.0.0` ships, semver guarantees take effect.

## [Unreleased]

### Added

- _Nothing yet._

### Changed

- Creator のビッグプレビュー描画倍率を 90% → 80% に変更し、ホットスポット編集領域の周囲余白を広げた。
- `theme.json` の `schema_version` を `1` に統一 (従来 `2`)。リリース前の段階で複数回 bump していた値を v1 へ巻き戻し、初回公開を `schema_version: 1` で揃える。既存の `~/.custom_cursors/` 配下に `schema_version: 2` のテーマが残っている場合は読込時に skip + warning ログとなる (`schema_version != 1`)。

### Deprecated

- _Nothing yet._

### Removed

- OS ダークモード連動による自動テーマ切替機能 (UI 配線が未完了のままだった半実装) を完全に除去。これに伴い `AppConfig.dark_mode` フィールドが削除され、IPC `get_dark_mode_status` も廃止。既存ユーザーの `config.json` 内 `dark_mode` キーは serde の未知フィールド読み飛ばしで透過的に消滅する (`schema_version` は据え置き)。**旧バイナリへのダウングレードは非推奨** (旧バイナリは新スキーマを parse error として扱う)。

### Fixed

- _Nothing yet._

### Security

- _Nothing yet._

---

[Unreleased]: https://github.com/nishiuriraku/easy-cursor-swap/compare/main...HEAD
