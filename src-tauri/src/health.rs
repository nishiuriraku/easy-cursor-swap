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
const GITHUB_RELEASES_BASE: &str = "https://github.com/nishiuriraku/easy-cursor-swap/releases";

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

impl StartupState {
    /// 正常起動を記録した新しい state を返す (ファイル I/O を伴わない純変換)。
    ///
    /// `previous_version` はクリアする (G23): 現行版が healthy と確認できた以上、
    /// ロールバック先 (移行元の旧版) はもう不要。残すと、後日この版が *バージョン変更を
    /// 伴わずに* 失敗した際、`rollback_target()` が「既に正常移行済みの旧版」への
    /// ロールバックを誤って提案し続ける。次の真のアップデート時に `begin()` が
    /// 新しい previous_version を再設定する。
    fn marked_healthy(&self, current_version: &str) -> StartupState {
        StartupState {
            pending_failures: 0,
            last_healthy_version: Some(current_version.to_string()),
            last_seen_version: Some(current_version.to_string()),
            previous_version: None,
        }
    }
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

/// 現在のビルドのアーキテクチャを release.yml の命名 (x64 / arm64) に解決する。
///
/// release.yml は x86_64 → "x64"、aarch64 → "arm64" でインストーラを命名する。
/// それ以外の (将来追加されうる) アーキテクチャでは命名規約が未確定なので、
/// 誤った URL を生成せず `None` を返す (呼出側で自動ロールバックをスキップさせる)。
fn arch_label() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Some("x64"),
        "aarch64" => Some("arm64"),
        other => {
            tracing::warn!(
                "未知のアーキテクチャ {} のため自動ロールバック URL を生成できません",
                other
            );
            None
        }
    }
}

/// 指定バージョン・指定アーキテクチャの NSIS インストーラの
/// GitHub Releases ダウンロード URL を組み立てる。
fn installer_url_for_arch(version: &str, arch: &str) -> String {
    format!("{GITHUB_RELEASES_BASE}/download/v{version}/EasyCursorSwap_{version}_{arch}-setup.exe")
}

/// 指定バージョンの NSIS インストーラの GitHub Releases ダウンロード URL を返す。
/// アーキテクチャは実行中ビルドから解決する (x86_64 → x64 / aarch64 → arm64)。
/// 未知のアーキテクチャでは `None` を返す。
pub fn installer_url_for(version: &str) -> Option<String> {
    arch_label().map(|arch| installer_url_for_arch(version, arch))
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
        let state = self.state.marked_healthy(current_version);
        let json = serde_json::to_string_pretty(&state)?;
        std::fs::write(&self.state_path, json)?;
        tracing::debug!("startup health: marked healthy (v{})", current_version);
        Ok(())
    }

    /// ロールバック先情報を返す。
    /// `previous_version` が記録されており、かつ現在のアーキテクチャの
    /// インストーラ URL を生成できる場合のみ `Some` を返す。
    /// 未知のアーキテクチャでは URL を組み立てられないため `None` (自動ロールバックをスキップ)。
    pub fn rollback_target(&self) -> Option<RollbackTarget> {
        let version = self.state.previous_version.clone()?;
        let installer_url = installer_url_for(&version)?;
        Some(RollbackTarget {
            installer_url,
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
    fn marked_healthy_resets_counter_and_clears_rollback_target() {
        // version 変更で previous_version がセットされ、pending_failures が立っている状態
        let state = StartupState {
            pending_failures: 2,
            last_seen_version: Some("0.0.7".to_string()),
            previous_version: Some("0.0.6".to_string()),
            last_healthy_version: None,
        };
        let healthy = state.marked_healthy("0.0.7");
        assert_eq!(healthy.pending_failures, 0);
        assert_eq!(healthy.last_healthy_version.as_deref(), Some("0.0.7"));
        assert_eq!(healthy.last_seen_version.as_deref(), Some("0.0.7"));
        // G23: 正常起動を記録したら previous_version はクリアされる
        // (バージョン変更を伴わない後日の失敗で旧版へのロールバックを誤提案しないため)
        assert_eq!(healthy.previous_version, None);
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
    fn is_major_bump_returns_false_for_same_version() {
        // 同一バージョン (= 再インストール) は major bump ではない
        assert!(!is_major_bump("1.0.0", "1.0.0"));
        assert!(!is_major_bump("0.1.0", "0.1.0"));
    }

    #[test]
    fn is_major_bump_returns_false_for_downgrade() {
        // ダウングレードは「メジャー跨ぎ警告」の対象ではない (旧版インストーラ)
        assert!(!is_major_bump("2.0.0", "1.9.9"));
        assert!(!is_major_bump("3.0.0", "1.0.0"));
    }

    #[test]
    fn is_major_bump_handles_minor_with_no_patch() {
        // "1.2" → "1.3" は minor のみ変化、major は同じ
        assert!(!is_major_bump("1.2", "1.3"));
        // "1" → "2" は major
        assert!(is_major_bump("1", "2"));
    }

    #[test]
    fn is_major_bump_handles_pre_release_suffix() {
        // 現実装は最初の `.` 区切り → parse なので、0.1.0-rc.1 は "0" として扱う
        // major 部だけ見ているのでサフィックス付きでも誤判定しない
        assert!(!is_major_bump("0.1.0-rc.1", "0.1.0"));
        assert!(is_major_bump("0.9.0", "1.0.0-rc.1"));
    }

    #[test]
    fn is_major_bump_returns_false_for_garbage_input() {
        // どちらか/両方が parse できなければ false (跨ぎなし扱いで安全側)
        assert!(!is_major_bump("not-a-version", "still-not"));
        assert!(!is_major_bump("1.0.0", "abc"));
        assert!(!is_major_bump("", ""));
    }

    #[test]
    fn is_major_bump_handles_large_major_numbers() {
        // u64 範囲ぎりぎりまで扱える
        assert!(is_major_bump("99.0.0", "100.0.0"));
        assert!(!is_major_bump("100.0.0", "99.0.0"));
    }

    #[test]
    fn major_of_extracts_first_segment() {
        // 内部 helper の境界条件
        assert_eq!(major_of("1.2.3"), Some(1));
        assert_eq!(major_of("0"), Some(0));
        assert_eq!(major_of("v1.0.0"), None); // v プレフィックスは未対応
        assert_eq!(major_of(""), None);
        assert_eq!(major_of("abc"), None);
    }

    #[test]
    fn installer_url_has_correct_format() {
        // arch 非依存で URL 組み立てを検証 (installer_url_for は実行ビルドの arch に依存するため)
        let url = installer_url_for_arch("1.2.3", "x64");
        assert!(url.contains("/v1.2.3/"));
        assert!(url.ends_with("_x64-setup.exe"));
    }

    #[test]
    fn installer_url_uses_https_github() {
        // フィッシング防止: ホスト名は github.com 系列に固定
        let url = installer_url_for_arch("1.0.0", "x64");
        assert!(
            url.starts_with("https://github.com/"),
            "unexpected base: {}",
            url
        );
    }

    #[test]
    fn installer_url_embeds_exact_version() {
        // バージョン文字列がそのまま埋め込まれる (path traversal 等は呼出側責任)
        let url = installer_url_for_arch("0.1.0", "x64");
        assert!(url.contains("v0.1.0"));
        assert!(url.contains("EasyCursorSwap_0.1.0"));
    }

    #[test]
    fn installer_url_for_arch_builds_x64_and_arm64() {
        // release.yml の命名規約 (x64 / arm64) を担保
        let x64 = installer_url_for_arch("1.2.3", "x64");
        assert!(x64.ends_with("EasyCursorSwap_1.2.3_x64-setup.exe"));
        assert!(x64.contains("/download/v1.2.3/"));

        let arm64 = installer_url_for_arch("1.2.3", "arm64");
        assert!(arm64.ends_with("EasyCursorSwap_1.2.3_arm64-setup.exe"));
        assert!(arm64.contains("/download/v1.2.3/"));
    }

    #[test]
    fn installer_url_for_matches_build_arch() {
        // 実行中ビルドの arch に応じて Some/None と URL 末尾が決まる
        let url = installer_url_for("1.2.3");
        match std::env::consts::ARCH {
            "x86_64" => {
                let url = url.expect("x86_64 では Some を返すべき");
                assert!(url.ends_with("EasyCursorSwap_1.2.3_x64-setup.exe"));
            }
            "aarch64" => {
                let url = url.expect("aarch64 では Some を返すべき");
                assert!(url.ends_with("EasyCursorSwap_1.2.3_arm64-setup.exe"));
            }
            _ => {
                // 未知のアーキテクチャでは自動ロールバック URL を生成しない
                assert!(url.is_none(), "未知 arch では None を返すべき");
            }
        }
    }
}
