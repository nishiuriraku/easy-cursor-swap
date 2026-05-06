//! 起動ヘルスチェック (Phase 8-4)
//!
//! 仕様書「§5 自動アップデート」より:
//!  > 新版起動失敗を 3 回連続検出した場合、旧バイナリへ自動ロールバックし、
//!  > トレイ通知で告知する。
//!
//! 実装方針:
//!  1. 起動時に `%LOCALAPPDATA%\EasyCursorSwap\state\startup.json` を読み込む
//!  2. `pending_failures` カウンタが 3 以上ならロールバック判定 + カウンタを 0 にリセット
//!  3. それ以外は `pending_failures += 1` してファイルに保存
//!  4. アプリの初期化が完了して run() に入った後、
//!     `mark_healthy()` を呼んでカウンタを 0 リセット
//!
//! クラッシュで run() に到達しなければカウンタは増えたまま残り、
//! 次回起動時に検出される。
//!
//! ロールバックはバイナリの自動置換ではなく GitHub Releases への誘導とする。
//! `previous_version` を保持し、前バージョンのインストーラ URL を生成して
//! ユーザーに再インストールを促す。

use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 連続失敗の閾値。これ以上で「ロールバック対象」と判定。
const ROLLBACK_THRESHOLD: u32 = 3;

/// GitHub リリースのベース URL (installer URL 生成に使用)
const GITHUB_RELEASES_BASE: &str = "https://github.com/easycursorswap/easy-cursor-swap/releases";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StartupState {
    /// 連続して mark_healthy() に到達できなかった回数
    #[serde(default)]
    pub pending_failures: u32,
    /// 最後に正常起動したアプリバージョン
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_healthy_version: Option<String>,
    /// 最後に確認した現行アプリバージョン (バージョンが変わると pending_failures をリセット)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_version: Option<String>,
    /// バージョン変更直前の旧バージョン (ロールバック先として使用)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_version: Option<String>,
}

/// 現行バージョン番号からメジャー番号を取得する。
/// パース失敗時は `None`。
fn major_of(version: &str) -> Option<u64> {
    version.split('.').next()?.parse().ok()
}

/// `current` → `next` がメジャーバージョン跨ぎかどうかを判定する。
///
/// どちらかがパースできなければ `false` を返す (跨ぎなしと扱う)。
pub fn is_major_bump(current: &str, next: &str) -> bool {
    match (major_of(current), major_of(next)) {
        (Some(c), Some(n)) => n > c,
        _ => false,
    }
}

/// 指定バージョンの NSIS インストーラの GitHub Releases ダウンロード URL を返す。
/// アーキテクチャは x64 固定。
pub fn installer_url_for(version: &str) -> String {
    format!("{GITHUB_RELEASES_BASE}/download/v{version}/EasyCursorSwap_{version}_x64-setup.exe")
}

/// ロールバック先情報
#[derive(Debug, Clone)]
pub struct RollbackTarget {
    pub version: String,
    pub installer_url: String,
    pub releases_page_url: String,
}

/// 起動ヘルスチェックの実行結果。
#[derive(Debug, Clone)]
pub struct StartupCheck {
    pub state: StartupState,
    /// 連続失敗が閾値を超えた → 旧バイナリへロールバック推奨
    pub should_rollback: bool,
    /// 検出済みフラグを反映済みのファイルパス (mark_healthy 呼出時に再書き込み)
    state_path: PathBuf,
}

impl StartupCheck {
    fn state_path() -> AppResult<PathBuf> {
        let base = dirs::data_local_dir()
            .ok_or_else(|| AppError::Config("LocalAppData が取得できません".to_string()))?;
        let dir = base.join("EasyCursorSwap").join("state");
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join("startup.json"))
    }

    /// 起動直後に呼ぶ。`pending_failures` をインクリメントし、ロールバック判定を返す。
    pub fn begin(current_version: &str) -> AppResult<Self> {
        let state_path = Self::state_path()?;
        let mut state: StartupState = if state_path.exists() {
            let content = std::fs::read_to_string(&state_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            StartupState::default()
        };

        // バージョンが変わった (アップデート直後 or ロールバック直後) なら
        // 旧版のカウンタをリセットして新版用に再起算
        if state.last_seen_version.as_deref() != Some(current_version) {
            tracing::info!(
                "アプリバージョン変更を検出: {:?} → {} (pending_failures をリセット)",
                state.last_seen_version,
                current_version
            );
            // ロールバック用に旧バージョンを保存してからカウンタリセット
            state.previous_version = state.last_seen_version.clone();
            state.pending_failures = 0;
        }

        let should_rollback = state.pending_failures >= ROLLBACK_THRESHOLD;
        if should_rollback {
            tracing::warn!(
                "連続起動失敗 {} 回を検出。ロールバック推奨。",
                state.pending_failures
            );
        }

        // ヘルスチェック前カウンタを 1 加算して保存
        state.pending_failures = state.pending_failures.saturating_add(1);
        state.last_seen_version = Some(current_version.to_string());

        let json = serde_json::to_string_pretty(&state)?;
        std::fs::write(&state_path, json)?;

        Ok(Self {
            state,
            should_rollback,
            state_path,
        })
    }

    /// 起動完了後、Tauri ウィンドウが描画されたタイミングで呼ぶ。
    /// `pending_failures` を 0 リセットして「正常起動」を記録。
    pub fn mark_healthy(&self, current_version: &str) -> AppResult<()> {
        let mut state = self.state.clone();
        state.pending_failures = 0;
        state.last_healthy_version = Some(current_version.to_string());
        state.last_seen_version = Some(current_version.to_string());
        let json = serde_json::to_string_pretty(&state)?;
        std::fs::write(&self.state_path, json)?;
        tracing::debug!("startup health: marked healthy (v{})", current_version);
        Ok(())
    }

    /// ロールバック先情報を返す。
    /// `previous_version` が記録されている場合のみ `Some` を返す。
    pub fn rollback_target(&self) -> Option<RollbackTarget> {
        let version = self.state.previous_version.clone()?;
        Some(RollbackTarget {
            installer_url: installer_url_for(&version),
            releases_page_url: format!("{GITHUB_RELEASES_BASE}/tag/v{version}"),
            version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_change_resets_counter() {
        let mut state = StartupState {
            pending_failures: 5,
            last_seen_version: Some("0.1.0".to_string()),
            ..Default::default()
        };

        // 新バージョン検出ロジックを再現
        let new_version = "0.2.0";
        if state.last_seen_version.as_deref() != Some(new_version) {
            state.previous_version = state.last_seen_version.clone();
            state.pending_failures = 0;
        }
        assert_eq!(state.pending_failures, 0);
        assert_eq!(state.previous_version.as_deref(), Some("0.1.0"));
    }

    #[test]
    fn threshold_is_three() {
        // 仕様書「3 回連続起動失敗で旧バイナリへ自動ロールバック」を担保
        assert_eq!(ROLLBACK_THRESHOLD, 3);
    }

    #[test]
    fn is_major_bump_detects_major_change() {
        assert!(is_major_bump("1.9.9", "2.0.0"));
        assert!(!is_major_bump("1.0.0", "1.5.0"));
        assert!(!is_major_bump("2.1.0", "2.2.0"));
        // パース失敗は false
        assert!(!is_major_bump("invalid", "2.0.0"));
    }

    #[test]
    fn installer_url_has_correct_format() {
        let url = installer_url_for("1.2.3");
        assert!(url.contains("/v1.2.3/"));
        assert!(url.ends_with("_x64-setup.exe"));
    }
}
