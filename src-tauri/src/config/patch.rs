//! 設定 update のための typed patch 定義 (Wave 2B / Task 3)
//!
//! ## なぜ full AppConfig ではなく patch なのか
//!
//! 旧 `update_config(updates: AppConfig)` は `*c = updates` で **全フィールドを
//! フロントから上書き** できてしまい、以下の安全上の問題があった:
//!
//!   1. フロントが `schema_version` を 1 に書き戻すと、起動時の `migrate` 判定が
//!      「V1 → V2 昇格が必要」と誤認して設定全体を壊す。
//!   2. フロントが `github_account` を `null` で上書きすると、Device Flow で連携
//!      済みのユーザーが keystore 側の OAuth トークンだけ取り残されて「幽霊連携」
//!      状態になる (keystore のトークン消し忘れと `login` 表示消失の二重事故)。
//!   3. セキュリティ閾値 (`max_pack_compressed_size` 等) や `favorites` /
//!      `usage` も `Some` 一致のみで混入可能で、悪意ある UI 拡張や将来のリファクタ
//!      で意図せぬ書き換えが起こる余地がある。
//!
//! `AppConfigPatch` は「ユーザー入力が変えて良い範囲」だけを `Option<T>` で持つ
//! ことで、上記 1〜3 を型レベルで防ぐ。`schema_version` / `github_account` /
//! `favorites` / `usage` / `active_theme_id` は patch 経由では一切変更できない。
//!
//! なお `update_config` の現行 `AppConfig` 受信は **削除** し、IPC を patch 化
//! する。frontend の `useAppSettings.update(mutator)` は渡された mutator の
//! 差分だけを patch に詰めて送る (差分がないフィールドは `None` にして送らない)。

use super::{GeneralConfig, LoggingConfig, SecurityConfig};
use serde::{Deserialize, Serialize};

/// `update_config` IPC が受け取る typed patch。
///
/// すべて Option なので、含まれるセクションだけ merge される。未指定セクションは
/// そのまま保持される (Rust 側 `ConfigManager::apply_patch` の責務)。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(export, rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct AppConfigPatch {
    /// 一般設定セクションの差分。`None` のとき general は無変更。
    pub general: Option<GeneralConfigPatch>,

    /// セキュリティ設定セクションの差分。`None` のとき security は無変更。
    pub security: Option<SecurityConfigPatch>,

    /// ログ設定セクションの差分。`None` のとき logging は無変更。
    pub logging: Option<LoggingConfigPatch>,
}

impl AppConfigPatch {
    /// Rust 側 `ConfigManager::apply_patch` が patch を実値に適用する。
    /// 変更があったセクションだけを書き換える。
    pub fn apply_to(self, current: &mut super::AppConfig) {
        if let Some(g) = self.general {
            g.apply_to(&mut current.general);
        }
        if let Some(s) = self.security {
            s.apply_to(&mut current.security);
        }
        if let Some(l) = self.logging {
            l.apply_to(&mut current.logging);
        }
    }
}

/// `GeneralConfig` の中でユーザー入力が変えて良いフィールドだけを `Option` で持つ。
///
/// `favorites` / `usage` / `active_theme_id` / `panic_hotkey` はこの patch には
/// 含めない (Rust 側の実装で直接書き換えるか別 IPC で扱う)。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct GeneralConfigPatch {
    pub auto_start: Option<bool>,
    pub auto_update: Option<bool>,
    pub language: Option<String>,
    pub crash_reporting: Option<bool>,

    // Wave 1A / 1B で追加された 6 フィールド。
    pub show_apply_toast: Option<bool>,
    pub apply_shadow_control: Option<bool>,
    pub start_minimized: Option<bool>,
    pub show_storage_warning: Option<bool>,
}

impl GeneralConfigPatch {
    pub fn apply_to(self, current: &mut GeneralConfig) {
        if let Some(v) = self.auto_start {
            current.auto_start = v;
        }
        if let Some(v) = self.auto_update {
            current.auto_update = v;
        }
        if let Some(v) = self.language {
            current.language = v;
        }
        if let Some(v) = self.crash_reporting {
            current.crash_reporting = v;
        }
        if let Some(v) = self.show_apply_toast {
            current.show_apply_toast = v;
        }
        if let Some(v) = self.apply_shadow_control {
            current.apply_shadow_control = v;
        }
        if let Some(v) = self.start_minimized {
            current.start_minimized = v;
        }
        if let Some(v) = self.show_storage_warning {
            current.show_storage_warning = v;
        }
    }
}

/// `SecurityConfig` の中でユーザー入力が変えて良いフィールドだけを `Option` で持つ。
/// 閾値 (`max_*` / `storage_warning_threshold`) はセキュリティ境界なので、
/// 将来閾値変更 UI を追加する場合も別 IPC (`update_security_thresholds`) を
/// 設けること (この patch では許可しない)。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct SecurityConfigPatch {
    pub require_signed_themes: Option<bool>,
    pub warn_unsigned_import: Option<bool>,
}

impl SecurityConfigPatch {
    pub fn apply_to(self, current: &mut SecurityConfig) {
        if let Some(v) = self.require_signed_themes {
            current.require_signed_themes = v;
        }
        if let Some(v) = self.warn_unsigned_import {
            current.warn_unsigned_import = v;
        }
    }
}

/// `LoggingConfig` の中でユーザー入力が変えて良いフィールドは `level` のみ。
/// `retention_days` / `max_total_size` は運用上の固定値なので patch に含めない。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfigPatch {
    pub level: Option<String>,
}

impl LoggingConfigPatch {
    pub fn apply_to(self, current: &mut LoggingConfig) {
        if let Some(v) = self.level {
            current.level = v;
        }
    }
}
