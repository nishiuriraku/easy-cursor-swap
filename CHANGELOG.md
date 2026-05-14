# Changelog

All notable changes to EasyCursorSwap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Pre-release status:** EasyCursorSwap is currently at `0.1.0` and has **not yet had a tagged release**. The `[Unreleased]` section captures changes accumulating toward the first public release. The `0.x` series may contain breaking changes; once `1.0.0` ships, semver guarantees take effect.

## [Unreleased]

### Added

- **Marketplace auto-submit flow**: テーマ選択 1 つで GitHub に PR を自動作成できる新フロー。GitHub Device Flow による 1 回限りの認証、Ed25519 署名、SHA-256 ハッシュ、`.cursorpack` ホスティング (インデックスリポ同梱方式) をアプリが代行する。既存の手動入力フローは `SubmitThemeDialog` のタブ切替で温存。新規 IPC 5 個 (`start_device_flow` / `complete_device_flow` / `cancel_device_flow` / `submit_theme_auto` / `revoke_github_link`) — 54 → 59。設定 → 鍵管理ページから連携解除可能。Client ID は build 時に `GITHUB_OAUTH_CLIENT_ID` 環境変数で注入する。
- Marketplace プレビュー画像: `.cursorpack` アーカイブに `previews/<role>.png` (64×64 PNG, Arrow / Help / AppStarting / Wait / Crosshair / IBeam の 6 ロール) を同梱するようになった。
- `marketplace_fetch_preview` IPC コマンド: インデックス CDN からプレビュー PNG を取得する。URL スキーム・ホスト・ロール名の検証、および 500KB 上限を実施。
- `useMarketplacePreviews` composable: `marketplace_fetch_preview` IPC ラッパー。シングルトンキャッシュ + in-flight 重複排除でカード表示の二重リクエストを防ぐ。
- `MarketplaceDetailModal` コンポーネント: カードクリックで詳細モーダルを開き、プレビューマトリクスと Install ボタンを表示。インストールフローをモーダル内に集約。
- ライブラリのカード / 行 / 詳細ドロワーに Marketplace ソースタグを追加し、マーケットプレース由来テーマを視覚的に区別できるようにした。

### Changed

- 設定スキーマを v1 → v2 に migrate (`AppConfig.github_account: Option<GithubAccount>` を追加)。serde(default) で v1 既存 config は透過マイグレーションされる。
- `MarketplaceCard` と `FeaturedCard` が `install` イベントの代わりに `showDetails` イベントを emit するように変更。インストールフローは `MarketplaceDetailModal` に移動した。
- `ThemeKind` 型に `'marketplace'` 値を追加。`useThemes` が source → kind のマッピングを更新。
- Creator のビッグプレビュー描画倍率を 90% → 80% に変更し、ホットスポット編集領域の周囲余白を広げた。
- `theme.json` の `schema_version` を `1` に統一 (従来 `2`)。リリース前の段階で複数回 bump していた値を v1 へ巻き戻し、初回公開を `schema_version: 1` で揃える。既存の `~/.custom_cursors/` 配下に `schema_version: 2` のテーマが残っている場合は読込時に skip + warning ログとなる (`schema_version != 1`)。
- 公式インデックス (Marketplace) のグリッドカード視覚スタイルを Library の `ThemeCard` と揃えた。プレビューを 3x2 (6 セル) のコンパクトマトリクスに縮小し、`.card-preview` の高さを 112px に詰め、カバレッジ表記を `X%` → `X/17` に変更。meta-row はダウンロード数とバージョンの 2 項目構成にし、`X/17` 重複を排除した。インポートボタン・verified バッジ・ダウンロード数などマーケットプレイス固有機能は維持。実カーソル PNG プレビューは別 issue (`docs/superpowers/issue/2026-05-12-marketplace-icon-preview.md`) のまま保留。
- Marketplace 画面の Featured ストリップを廃止し、全エントリを横並びレイアウト (旧 `FeaturedCard`) の 1 つのグリッドに統一。Featured / 通常の視覚的区別を取り除き、ハイライトラベル (`new` / `popular`) はカード内に残す。
- 詳細モーダル (`MarketplaceDetailModal`) から SHA-256 表示行を削除 (UX 簡素化)。バックグラウンドでの整合性検証は変更なし。

### Deprecated

- _Nothing yet._

### Removed

- OS ダークモード連動による自動テーマ切替機能 (UI 配線が未完了のままだった半実装) を完全に除去。これに伴い `AppConfig.dark_mode` フィールドが削除され、IPC `get_dark_mode_status` も廃止。既存ユーザーの `config.json` 内 `dark_mode` キーは serde の未知フィールド読み飛ばしで透過的に消滅する (`schema_version` は据え置き)。**旧バイナリへのダウングレードは非推奨** (旧バイナリは新スキーマを parse error として扱う)。
- Library / Marketplace 検索ボックスの `⌘K` バッジと、Creator スタート画面の `Ctrl+N` / `Ctrl+O` ヒント表示を削除。いずれも実機能 (キーバインドハンドラ) を持たない装飾だったため、アクセシビリティ的にも誤誘導となるため除去。グローバルホットキーとしては引き続き `Ctrl+Alt+Shift+R` (パニックリセット) のみが有効。あわせて未使用となった i18n キー `creatorStart.kbdNew` / `creatorStart.kbdOpen` を `ja.ts` / `en.ts` の双方から削除。

### Fixed

- ライブラリのテーマ詳細モーダルがコンテンツ高さに追従せず、常にビューポート最大サイズで開いていた不具合を修正。`.td-standalone` から `height: 100%` (`h-full`) を外し、レイアウトを `.td-modal-shell` の `h-auto` + `max-h` に一本化。あわせて `.td-modal-body` に `min-h-0` を付与し、コンテンツ超過時のみ body 内でスクロールするようにした。
- Creator のビッグプレビューがロール未取り込み (`empty`) 状態でも既定のカーソルアイコンとホットスポット dot を描画してしまい、取り込み済みロールとの判別がつかなかった問題を修正。`creator.vue` の `<CursorPreview>` 呼び出しから `role-id` / `fallback-icon-size` を外し、`:hide-dot="activePreviewAsset.kind === 'empty'"` を付与して未取り込み時は完全に空表示にした (共有コンポーネント側の semantics は維持し、Library のテーマ詳細ドロワーのプレースホルダ表示には影響しない)。
- ライブラリ詳細モーダル「Creator で編集」経由で Creator に取り込んだテーマを保存するときに、SaveDestinationModal の「既存テーマを上書き保存 / 複製として保存」セクションが表示されず常に新規 UUID で複製されてしまっていた問題を修正。原因は `parse_cursorpack_for_creator` IPC の返却型 `CursorpackMetadata` (Rust 側) と `ParsedCursorpack.metadata` (TS 側) の双方で `id` フィールドが定義から漏れており、`creator.vue` の `sourceThemeId.value = parsed.metadata.id ?? null` が常に `null` を読んでいたため。両側に `id: Option<String>` / `id: string | null` を追加し、Rust 側 `metadata_from_theme` で `meta.id.to_string()` を返すよう修正。回帰防止のため `parse_cursorpack_basic_returns_roles` テストに `metadata.id` の UUID 検証を追加。あわせて Creator から上書き保存した直後に Library 側のプレビューキャッシュ (`useThemePreviews`) が無効化されず古い PNG が表示され続けていた問題も修正 (`useCreatorExport` の Library 保存成功時に `invalidate(theme_id)` を呼び、`cursor-changed` リスナー側でも旧アクティブ id を invalidate する 2 経路防御)。
- WCAG 2.1 AA コントラスト監査 (`docs/superpowers/2026-05-14-wcag-aa-audit.md`) で違反 560 件の約 79% が `--fg-mute` / `--fg-faint` の輝度不足に起因すると判明したため、両テーマの該当トークンを AA 通過値へ引き上げた (Dark `--fg-mute` `#5a6076` → `#9aa0b3` / `--fg-faint` `#3a3f50` → `#787e94`、Light `--fg-mute` `#8b91a3` → `#626872` / `--fg-faint` `#c2c7d4` → `#696e80`)。あわせてライトモードでのみ閾値割れしていた `--violet` を `#6a5cff` → `#4f3fde` に補正。これによりサイドバーのセクション見出し・`nav-count` バッジ・カードのメタ行・breadcrumb 区切り・キーキャップ等のコントラストが AA を満たすようになる。
- 公式インデックス由来テーマの `preview_base_url` を Rust 側 `MarketplaceEntry` が受け取れず、`MarketplaceDetailModal` の PNG プレビューが一度もフェッチされなかった問題を修正。
- Marketplace 画面で `highlight` 付きエントリが Featured ストリップとメイン Grid に二重表示される問題を修正。
- Library 画面で `get_themes` IPC が返す `source: "marketplace"` を読まず `kind: "local"` がハードコードされていたため、MARKETPLACE タグ表示と編集/エクスポートの readonly ガード UI が効いていなかった問題を修正。
- 公式インデックスのインストール時に `JSONエラー: missing field "github_username"` が出てダウンロードできない問題を修正。Rust 側 `AuthorRecord` が server の `"github"` キーを `serde(alias)` で受け入れるようにした。
- 適用中のテーマを Library 詳細ドロワーから削除できてしまう問題を修正。`isActive=true` のとき削除ボタンは disabled になり、IPC 経由でも `delete_theme` がレジストリ実態を照合してアクティブテーマの削除要求を拒否する。

### Security

- Marketplace テーマの読み取り専用ガード: `repackage_theme` IPC がソースを確認し、公式インデックス由来のテーマに対する編集・エクスポート要求を拒否するようになった。

---

[Unreleased]: https://github.com/nishiuriraku/easy-cursor-swap/compare/main...HEAD
