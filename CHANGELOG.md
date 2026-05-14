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

- ライブラリのテーマ詳細モーダルがコンテンツ高さに追従せず、常にビューポート最大サイズで開いていた不具合を修正。`.td-standalone` から `height: 100%` (`h-full`) を外し、レイアウトを `.td-modal-shell` の `h-auto` + `max-h` に一本化。あわせて `.td-modal-body` に `min-h-0` を付与し、コンテンツ超過時のみ body 内でスクロールするようにした。
- Creator のビッグプレビューがロール未取り込み (`empty`) 状態でも既定のカーソルアイコンとホットスポット dot を描画してしまい、取り込み済みロールとの判別がつかなかった問題を修正。`creator.vue` の `<CursorPreview>` 呼び出しから `role-id` / `fallback-icon-size` を外し、`:hide-dot="activePreviewAsset.kind === 'empty'"` を付与して未取り込み時は完全に空表示にした (共有コンポーネント側の semantics は維持し、Library のテーマ詳細ドロワーのプレースホルダ表示には影響しない)。
- WCAG 2.1 AA コントラスト監査 (`docs/superpowers/2026-05-14-wcag-aa-audit.md`) で違反 560 件の約 79% が `--fg-mute` / `--fg-faint` の輝度不足に起因すると判明したため、両テーマの該当トークンを AA 通過値へ引き上げた (Dark `--fg-mute` `#5a6076` → `#9aa0b3` / `--fg-faint` `#3a3f50` → `#6e7488`、Light `--fg-mute` `#8b91a3` → `#5b6070` / `--fg-faint` `#c2c7d4` → `#7a8095`)。あわせてライトモードでのみ閾値割れしていた `--violet` を `#6a5cff` → `#4f3fde` に補正。これによりサイドバーのセクション見出し・`nav-count` バッジ・カードのメタ行・breadcrumb 区切り・キーキャップ等のコントラストが AA を満たすようになる。

### Security

- _Nothing yet._

---

[Unreleased]: https://github.com/nishiuriraku/easy-cursor-swap/compare/main...HEAD
