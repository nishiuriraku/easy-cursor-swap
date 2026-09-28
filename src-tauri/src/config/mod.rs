//! EasyCursorSwap 設定管理モジュール
//!
//! アプリケーション設定の Source of Truth を Rust 側で管理する。
//! 設定は `config.json` に永続化し、UIが閉じていても常駐プロセスが参照できる。
//!
//! 構成:
//!
//! | サブモジュール | 役割 |
//! |---|---|
//! | `schema`  | `AppConfig` と子型、`CURRENT_SCHEMA_VERSION`、`DEFAULT_*` サイズ上限 (ts-rs 生成対象) |
//! | `store`   | `ConfigManager` (RwLock + atomic temp-write/rename + `config.corrupt.*.json` quarantine + backup) |
//! | `migrate` | schema_version 分岐と v1→v2 変換 (3 入力経路の共有点) |
//! | [`v1`]    | v1 スキーマのスナップショット (変更禁止) |
//! | [`patch`] | `update_config` IPC 用 typed patch |
//!
//! 外部からは従来どおり `crate::config::{AppConfig, ConfigManager, …}` の 1 段パスで参照する
//! (`schema` / `store` / `migrate` は private、ここで再エクスポート)。

mod migrate;
mod schema;
mod store;

/// 設定スキーマ v1 のスナップショットモジュール。
pub mod v1;

/// 設定 update 用の typed patch (Wave 2B / Task 3)。
/// `update_config` IPC の入力型を `AppConfigPatch` に固定し、フロントから
/// `schema_version` / `github_account` / セキュリティ閾値 / 履歴系フィールド
/// (`favorites` / `usage` / `active_theme_id`) を書き換えられないようにする。
pub mod patch;

pub use schema::{
    AppConfig, BackupInfo, GeneralConfig, GithubAccount, LoggingConfig, SecurityConfig, ThemeUsage,
    CURRENT_SCHEMA_VERSION, DEFAULT_MAX_IMAGE_FILE_SIZE, DEFAULT_MAX_PACK_COMPRESSED_SIZE,
    DEFAULT_MAX_PACK_UNCOMPRESSED_SIZE,
};
pub use store::ConfigManager;

#[cfg(test)]
pub(crate) use store::cursors_dir_override_lock;

#[cfg(test)]
mod tests;
