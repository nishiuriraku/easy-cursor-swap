//! `.cursorprofile` バックアップの IPC コマンド。
//!
//! `.cursorprofile` は AppConfig + 全テーマディレクトリを 1 つの ZIP にまとめたもの。
//! PC 移行 / OS 再インストール時の復元用。マージ・上書きの両モードがある。

use crate::backup::{BackupManager, ProfileEnvelope};
use crate::config::ConfigManager;
use crate::errors::AppError;
use tauri::State;

/// `.cursorprofile` (設定 + 全テーマ) を指定パスに書き出す。
#[tauri::command]
pub fn export_profile(config: State<'_, ConfigManager>, path: String) -> Result<(), AppError> {
    let cfg = config.get()?;
    let target = std::path::PathBuf::from(&path);
    BackupManager::export(&target, &cfg)
}

/// `.cursorprofile` を読み込んで設定と全テーマを復元する。
/// `merge=true` なら既存テーマを保持し新規分のみ反映、`false` なら完全上書き。
#[tauri::command]
pub fn import_profile(
    config: State<'_, ConfigManager>,
    path: String,
    merge: bool,
) -> Result<ProfileEnvelope, AppError> {
    let buf = std::path::PathBuf::from(&path);
    if !buf.exists() {
        return Err(AppError::Theme(format!(
            "ファイルが見つかりません: {}",
            path
        )));
    }
    let envelope = BackupManager::import(&buf, merge)?;
    // 設定もファイル経由で復元
    config.update(|c| {
        *c = envelope.config.clone();
    })?;
    Ok(envelope)
}
