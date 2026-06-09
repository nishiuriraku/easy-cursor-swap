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
    // 設定もファイル経由で復元。ただし github_account は現マシンの値を保持する (Y16)。
    config.update(|c| {
        *c = apply_imported_config(c, &envelope.config);
    })?;
    Ok(envelope)
}

/// import 時に、バックアップの config を適用しつつ現マシンの `github_account` を保持する (Y16)。
///
/// GitHub 連携メタ (`github_account`) は keystore のトークン (DPAPI 暗号化・マシン固有) と
/// 対で初めて意味を持つ。`.cursorprofile` はトークンを含まず、別マシン由来 / 連携解除後の
/// 古い状態である可能性がある。そのまま上書きすると「login は表示されるがトークンが無い /
/// 食い違う」孤児状態になるため、現在マシンの値を維持して keystore と整合させる。
fn apply_imported_config(
    current: &crate::config::AppConfig,
    imported: &crate::config::AppConfig,
) -> crate::config::AppConfig {
    crate::config::AppConfig {
        github_account: current.github_account.clone(),
        ..imported.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, GithubAccount};

    #[test]
    fn apply_imported_config_preserves_current_github_account() {
        let current = AppConfig {
            github_account: Some(GithubAccount {
                login: "me".to_string(),
                token_saved_at: "2026-06-09T00:00:00Z".to_string(),
            }),
            ..AppConfig::default()
        };
        let imported = AppConfig {
            github_account: Some(GithubAccount {
                login: "stale-other".to_string(),
                token_saved_at: "2020-01-01T00:00:00Z".to_string(),
            }),
            ..AppConfig::default()
        };

        let merged = apply_imported_config(&current, &imported);
        // github_account は現マシンの値を保持 (バックアップ由来の login で上書きしない)
        assert_eq!(merged.github_account.as_ref().unwrap().login, "me");
    }

    #[test]
    fn apply_imported_config_keeps_none_when_current_unlinked() {
        // 現マシンが未連携なら、バックアップに連携メタがあっても None を保持する
        // (トークンを持たない孤児メタの復元を防ぐ)。
        let current = AppConfig::default();
        let imported = AppConfig {
            github_account: Some(GithubAccount {
                login: "x".to_string(),
                token_saved_at: "t".to_string(),
            }),
            ..AppConfig::default()
        };
        let merged = apply_imported_config(&current, &imported);
        assert!(merged.github_account.is_none());
    }
}
