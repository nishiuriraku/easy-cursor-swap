//! EasyCursorSwap テーマ管理モジュール
//!
//! `.cursorpack` パッケージの作成・解凍・バリデーション、ライブラリの一覧、
//! レジストリへの適用、Windows スキーム連携などを担う。
//!
//! 構成 (2026-05-18 で 4 サブモジュールに分割、refactor/yellow-items):
//!
//! | サブモジュール | 役割 |
//! |---|---|
//! | [`types`] | `ThemeMetadata` / `LocalizedString` / `CursorDefinition` / `ThemeSummary` 等の DTO + 内部 helper |
//! | [`sanitize`] | ZIP エントリ名のパストラバーサル対策 |
//! | [`listing`] | テーマ一覧 / metadata 読込 / 孤児カーソル復旧 + `set_metadata_source` 共有 helper |
//! | [`preview`] | PNG プレビュー生成 (ユーザーテーマ / Windows スキーム / `.cursorpack` 内蔵 previews) + `RolePreview` |
//! | [`apply`] | テーマ適用 + 現在レジストリと一致するか判定 |
//! | [`package`] | `.cursorpack` zip 入出力 / 削除 / 複製 / 再パッケージ / スキーム書出 |
//!
//! [`ThemeManager`] (ZST + 大きな `impl`) を本ファイルで定義し、各サブモジュールが
//! 追加の `impl ThemeManager { ... }` で機能を足す形にしている。サブモジュール間で
//! 共有する `pub(crate)` 定数 (`PREVIEW_ROLES` 等) は [`preview`] に定義。

pub mod apply;
pub mod listing;
pub mod package;
pub mod preview;
pub mod sanitize;
pub mod types;

pub use preview::RolePreview;
pub use sanitize::sanitize_archive_path_pub;
pub use types::{
    CursorDefinition, CursorpackInspection, ExistingTheme, LocalizedString, SizeOverride,
    ThemeMetadata, ThemeSummary,
};

// 旧 mod.rs 直下にあった `set_metadata_source` の外部 caller (marketplace::install) 用の
// 再エクスポート。crate 内部からは listing::set_metadata_source として直接呼ばれる。
pub(crate) use listing::set_metadata_source;

/// テーママネージャー (ZST、メソッドは各サブモジュールの `impl ThemeManager` ブロックに分散)。
pub struct ThemeManager;
