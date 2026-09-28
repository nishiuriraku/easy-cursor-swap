//! 設定 JSON → 現行スキーマ (`AppConfig`) への変換。
//!
//! 通常起動 (`store::ConfigManager::load_or_initialize`) / `restore_config_backup` /
//! `.cursorprofile` import の 3 入力経路が共有する唯一の変換点。公開 API は
//! 従来どおり `ConfigManager::migrate` (store.rs の薄いラッパ) で提供する。

use super::schema::{AppConfig, CURRENT_SCHEMA_VERSION};
use super::v1;
use crate::errors::{AppError, AppResult};

/// 設定 JSON 文字列を V2 AppConfig へ変換する共通エントリ。
///
/// 3 つの入力経路(通常起動 / `restore_config_backup` / `.cursorprofile` import)が
/// ここを共有する。返り値:
///   - `(AppConfig, false)`: 入力は既に V2。永続化不要。
///   - `(AppConfig, true)`: 入力は V1。V2 へ昇格したので永続化が望ましい
///     (= 呼び出し側が atomic_write で書き戻す)。
///   - `Err`: 不正 JSON / V1 としてパース不能 / 新しすぎる schema_version。
///
/// V1 → V2 変換ではユーザー設定 (auto_start / language / favorites / usage /
/// github_account / max_pack_* / storage_warning_threshold / logging) はそのまま
/// 引き継ぎ、V2 で追加された 6 フィールドは V2 既定値で初期化する。
pub(super) fn migrate(raw_json: &str) -> AppResult<(AppConfig, bool)> {
    // まず schema_version だけ先読みして分岐する。
    // 直接 AppConfig / AppConfigV1 のどちらにも deserialize せず、Value 経由で
    // スキーマ番号を確実に拾う (未知フィールドで失敗しないように)。
    let value: serde_json::Value = serde_json::from_str(raw_json)
        .map_err(|e| AppError::Config(format!("設定 JSON のパースに失敗: {}", e)))?;
    let schema_version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| AppError::Config("設定 JSON に schema_version がありません".to_string()))?
        as u32;

    match schema_version {
            1 => {
                // V1 → V2 変換
                let v1: v1::AppConfigV1 = serde_json::from_value(value).map_err(|e| {
                    AppError::Config(format!("v1 設定の解釈に失敗: {}", e))
                })?;
                Ok((v1.into_v2(), true))
            }
            n if n == CURRENT_SCHEMA_VERSION => {
                // 既に V2。
                let cfg: AppConfig = serde_json::from_value(value).map_err(|e| {
                    AppError::Config(format!("v2 設定の解釈に失敗: {}", e))
                })?;
                Ok((cfg, false))
            }
            n if n > CURRENT_SCHEMA_VERSION => Err(AppError::Config(format!(
                "設定ファイルのバージョン ({}) はこのアプリ ({}) より新しいです。\nアプリの更新が必要です。",
                n, CURRENT_SCHEMA_VERSION
            ))),
            n => Err(AppError::Config(format!(
                "設定ファイルのバージョン ({}) はこのアプリ ({}) より古すぎて解釈できません。",
                n, CURRENT_SCHEMA_VERSION
            ))),
        }
}
