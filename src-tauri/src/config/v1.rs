//! EasyCursorSwap 設定スキーマ v1 (Wave 1A 以前の旧版)
//!
//! v1 構造体は **明示的に** 残す:
//! - `ConfigManager::migrate` が v1 → v2 変換の入力型として使う
//! - v1 JSON のテストフィクスチャ (5 種) を作成するときの型ヒントとして参照される
//! - serde の未知フィールド黙殺 (`dark_mode` 等の旧データが残っていても安全に読み飛ばせる)
//!   を独立してテストできる
//!
//! フィールド追加 / 削除は **v2 以降への変更は禁止**。変更したい場合は v3 を新設する。
//! これにより「v1 ユーザーが持っているかもしれない JSON が、ある日突然 deserialize 失敗する」
//! 事故を防ぐ。

use crate::config::{
    AppConfig, GeneralConfig, LoggingConfig, SecurityConfig, CURRENT_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// v1 設定スキーマ (Wave 1A 以前のスナップショット)。
///
/// v2 で追加される 6 フィールド (`show_apply_toast` / `apply_shadow_control` /
/// `start_minimized` / `show_storage_warning` / `require_signed_themes` /
/// `warn_unsigned_import`) は **存在しない**。`AppConfigV1::into_v2` で v2 既定値
/// (true / true / false / true / false / true) を補填して AppConfig に変換する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfigV1 {
    pub schema_version: u32,
    pub general: GeneralConfigV1,
    pub security: SecurityConfigV1,
    pub logging: LoggingConfigV1,

    /// v1 末期に導入された GitHub 連携メタ。v1 初期の JSON には存在しないため
    /// `serde(default)` で `None` フォールバック。
    #[serde(default)]
    pub github_account: Option<crate::config::GithubAccount>,
}

/// v1 の `general` セクション。v2 で 4 フィールドが追加される。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfigV1 {
    pub auto_start: bool,
    pub auto_update: bool,
    pub language: String,
    pub active_theme_id: Option<Uuid>,
    pub panic_hotkey: String,
    /// v1 末期に追加。旧 JSON には存在しないので `serde(default)` で false 補填。
    #[serde(default)]
    pub crash_reporting: bool,
    /// v1 末期に追加。旧 JSON には存在しないので `serde(default)` で空 Vec 補填。
    #[serde(default)]
    pub favorites: Vec<Uuid>,
    /// v1 末期に追加。旧 JSON には存在しないので `serde(default)` で空 HashMap 補填。
    #[serde(default)]
    pub usage: HashMap<Uuid, crate::config::ThemeUsage>,
}

/// v1 の `security` セクション。v2 で 2 フィールド (`require_signed_themes` /
/// `warn_unsigned_import`) が追加される。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfigV1 {
    pub max_pack_compressed_size: u64,
    pub max_pack_uncompressed_size: u64,
    pub max_image_file_size: u64,
    pub storage_warning_threshold: u64,
}

/// v1 の `logging` セクション。v2 ではフィールド構成は変わらない。
pub type LoggingConfigV1 = LoggingConfig;

impl AppConfigV1 {
    /// v1 → v2 変換。v2 で追加されたフィールドは **v2 既定値** で初期化する。
    ///
    /// ユーザー設定 (auto_start / language / favorites / usage 等) はそのまま v2 に
    /// 引き継ぐ。これにより Wave 1A 移行時に既存ユーザーの設定が消えないことを保証する。
    pub fn into_v2(self) -> AppConfig {
        let v2_general_default = GeneralConfig::default();
        AppConfig {
            schema_version: CURRENT_SCHEMA_VERSION,
            general: GeneralConfig {
                // v1 から引き継ぐフィールド
                auto_start: self.general.auto_start,
                auto_update: self.general.auto_update,
                language: self.general.language,
                active_theme_id: self.general.active_theme_id,
                panic_hotkey: self.general.panic_hotkey,
                crash_reporting: self.general.crash_reporting,
                favorites: self.general.favorites,
                usage: self.general.usage,
                // v2 で追加: すべて v2 既定値で初期化
                show_apply_toast: v2_general_default.show_apply_toast,
                apply_shadow_control: v2_general_default.apply_shadow_control,
                start_minimized: v2_general_default.start_minimized,
                show_storage_warning: v2_general_default.show_storage_warning,
            },
            security: SecurityConfig {
                // v1 から引き継ぐフィールド
                max_pack_compressed_size: self.security.max_pack_compressed_size,
                max_pack_uncompressed_size: self.security.max_pack_uncompressed_size,
                max_image_file_size: self.security.max_image_file_size,
                storage_warning_threshold: self.security.storage_warning_threshold,
                // v2 で追加: すべて v2 既定値で初期化
                require_signed_themes: SecurityConfig::default().require_signed_themes,
                warn_unsigned_import: SecurityConfig::default().warn_unsigned_import,
            },
            logging: self.logging,
            github_account: self.github_account,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v1 JSON 5 種のうち代表 1 種(未知キー `dark_mode` 含む旧 config)→ v1 struct 復元の
    /// 通過確認。`serde_json` は既定で未知フィールドを黙殺するため、
    /// `dark_mode` ブロックは自然に読み飛ばされる。
    #[test]
    fn v1_with_dark_mode_deserializes_into_v1_struct() {
        let json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false
            },
            "dark_mode": {
                "enabled": false,
                "light_theme_id": null,
                "dark_theme_id": null
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
        let v1: AppConfigV1 = serde_json::from_str(json).expect("v1 JSON should parse");
        assert_eq!(v1.schema_version, 1);
        assert_eq!(v1.general.language, "ja");
        assert!(v1.github_account.is_none());
    }

    /// `into_v2` がユーザー設定 (auto_start / language / favorites / usage / github_account)
    /// を保持しつつ、v2 で追加された 6 フィールドを v2 既定値で埋めることを確認する。
    #[test]
    fn into_v2_preserves_user_settings_and_fills_v2_defaults() {
        let v1 = AppConfigV1 {
            schema_version: 1,
            general: GeneralConfigV1 {
                auto_start: false,
                auto_update: true,
                language: "en".to_string(),
                active_theme_id: Some(Uuid::nil()),
                panic_hotkey: "Ctrl+Alt+Shift+R".to_string(),
                crash_reporting: true,
                favorites: vec![Uuid::nil()],
                usage: HashMap::new(),
            },
            security: SecurityConfigV1 {
                max_pack_compressed_size: 1024,
                max_pack_uncompressed_size: 2048,
                max_image_file_size: 512,
                storage_warning_threshold: 999_999,
            },
            logging: LoggingConfig {
                level: "DEBUG".to_string(),
                retention_days: 7,
                max_total_size: 50 * 1024 * 1024,
            },
            github_account: None,
        };

        let v2 = v1.into_v2();

        // v1 から引き継いだフィールド
        assert!(!v2.general.auto_start);
        assert_eq!(v2.general.language, "en");
        assert!(v2.general.crash_reporting);
        assert_eq!(v2.general.favorites, vec![Uuid::nil()]);
        assert_eq!(v2.security.max_pack_compressed_size, 1024);
        assert_eq!(v2.security.storage_warning_threshold, 999_999);
        assert_eq!(v2.logging.level, "DEBUG");
        assert_eq!(v2.logging.retention_days, 7);

        // v2 で追加されたフィールド: v2 既定値
        assert!(v2.general.show_apply_toast, "show_apply_toast 既定 true");
        assert!(
            v2.general.apply_shadow_control,
            "apply_shadow_control 既定 true"
        );
        assert!(!v2.general.start_minimized, "start_minimized 既定 false");
        assert!(
            v2.general.show_storage_warning,
            "show_storage_warning 既定 true"
        );
        assert!(
            !v2.security.require_signed_themes,
            "require_signed_themes 既定 false"
        );
        assert!(
            v2.security.warn_unsigned_import,
            "warn_unsigned_import 既定 true"
        );

        // schema_version は v2 (CURRENT_SCHEMA_VERSION) になっている
        assert_eq!(v2.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(v2.schema_version, 2);
    }
}
