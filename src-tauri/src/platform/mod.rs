//! OS カーソル機構の抽象化層。
//!
//! `CursorBackend` は「レジストリ / OS API に触る操作」だけを trait 境界に載せ、
//! commands / main / tray / theme は `Arc<dyn CursorBackend>` 経由でのみ OS に到達する。
//! Windows 実装は `registry::RegistryManager` へ委譲し (`windows.rs`)、非 Windows ビルドは
//! `noop.rs`、単体テストは `memory.rs` (インメモリ) を差し込む。
//!
//! 不変条件 (shared/invariants.md): apply はトランザクショナル (snapshot → 書込 → commit /
//! rollback)、起動時 leftover snapshot は **Windows 既定へリセット** (pre-apply 値へは戻さない)。
//! これらは `registry::transaction` と `recover_pending_snapshot_on_startup` に実装され、
//! backend 実装はそれを迂回してはならない。

use crate::accessibility::AccessibilityConflicts;
use crate::errors::AppResult;
use crate::registry::{snapshot, PendingSnapshotState, WindowsScheme};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use crate::registry::CursorRole;

#[cfg(test)]
pub mod memory;
#[cfg(not(windows))]
pub mod noop;
#[cfg(windows)]
pub mod windows;

/// tauri `manage()` に登録する共有ハンドル。コマンドは `State<'_, SharedBackend>` で受ける。
pub type SharedBackend = Arc<dyn CursorBackend>;

/// OS カーソル機構への操作境界。
///
/// 既存 `RegistryManager` のメソッド名を維持しつつ、"windows" を含む 3 つだけ
/// 中立名にする (`reset_to_os_default` / `list_os_schemes` / `apply_os_scheme`)。
/// 返り値型は既存のまま (`WindowsScheme` / `PendingSnapshotState` /
/// `AccessibilityConflicts`)。
pub trait CursorBackend: Send + Sync {
    /// 現在の 17 役割の値 (環境変数展開済み)。
    fn read_current_cursors(&self) -> AppResult<HashMap<String, String>>;
    /// テーマ適用 (トランザクショナル)。未指定役割は OS 既定継承。
    fn apply_cursors(&self, cursor_paths: &HashMap<String, PathBuf>) -> AppResult<()>;
    /// OS 既定へリセット (パニックボタン / 起動時復旧)。EmergencyBestEffort。
    fn reset_to_os_default(&self) -> AppResult<()>;
    /// 初回スナップショット (インストール前状態) へ復元。NormalTransactional。
    fn restore_from_initial_snapshot(&self) -> AppResult<()>;
    /// OS に「カーソル設定が変わった」ことを通知 (Windows: SPI_SETCURSORS)。
    fn notify_cursor_change(&self) -> AppResult<()>;
    fn set_cursor_shadow(&self, enabled: bool) -> AppResult<()>;
    /// 戻り値は clamp 後に実際に書いた値。
    fn set_cursor_base_size(&self, size: u32) -> AppResult<u32>;
    fn register_scheme(
        &self,
        scheme_name: &str,
        cursor_paths: &HashMap<String, PathBuf>,
    ) -> AppResult<()>;
    fn unregister_schemes_for_theme(&self, theme_dir: &Path) -> AppResult<usize>;
    fn list_os_schemes(&self) -> AppResult<Vec<WindowsScheme>>;
    fn apply_os_scheme(&self, scheme: &WindowsScheme) -> AppResult<()>;
    fn accessibility_conflicts(&self) -> AccessibilityConflicts;

    // --- default impl (両実装で共用: 純ファイル I/O) ---
    fn inspect_pending_snapshot(&self) -> AppResult<PendingSnapshotState> {
        snapshot::inspect_pending_snapshot()
    }
    fn remove_pending_snapshot(&self) -> AppResult<()> {
        snapshot::remove_pending_snapshot()
    }
    fn save_initial_snapshot(&self) -> AppResult<()> {
        let values = self.read_current_cursors()?;
        snapshot::save_initial_snapshot_with(values)
    }
    /// `registry::paths_match_current_registry` と同じ判定 (Err → false)。
    fn paths_match_current(&self, expected: &HashMap<String, String>) -> bool {
        let non_empty: Vec<(&String, &String)> =
            expected.iter().filter(|(_, v)| !v.is_empty()).collect();
        if non_empty.is_empty() {
            return false;
        }
        let current = match self.read_current_cursors() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("paths_match_current: read failed: {}", e);
                return false;
            }
        };
        non_empty.iter().all(|(role, path)| {
            current
                .get(*role)
                .map(|c| c.eq_ignore_ascii_case(path))
                .unwrap_or(false)
        })
    }
}

/// 実行環境に応じた既定 backend。`main.rs` と Windows 専用 bin から呼ぶ。
pub fn default_backend() -> SharedBackend {
    #[cfg(windows)]
    {
        Arc::new(windows::WindowsCursorBackend)
    }
    #[cfg(not(windows))]
    {
        Arc::new(noop::NoopCursorBackend)
    }
}

/// 起動時 leftover snapshot の復旧判定と実行。`Valid` / `Unreadable` は区別せず
/// **OS 既定へリセット** する (pre-apply 値への部分復元は混在状態を残すため行わない)。
/// 戻り値はログ / テスト用の結果種別。
#[derive(Debug, PartialEq, Eq)]
pub enum StartupRecovery {
    NotNeeded,
    ResetToDefault,
    ResetFailed(String),
    InspectFailed(String),
}

pub fn recover_pending_snapshot_on_startup(backend: &dyn CursorBackend) -> StartupRecovery {
    match backend.inspect_pending_snapshot() {
        Ok(PendingSnapshotState::Valid(_snapshot)) => {
            tracing::warn!(
                "前回の適用処理が中断されていました。Windows 既定へリセットします (適用前への復元ではない)"
            );
            let result = match backend.reset_to_os_default() {
                Ok(()) => {
                    tracing::info!("クラッシュリカバリ完了 (Windows 既定へリセット)");
                    StartupRecovery::ResetToDefault
                }
                Err(e) => {
                    tracing::error!("クラッシュリカバリに失敗: {}", e);
                    StartupRecovery::ResetFailed(e.to_string())
                }
            };
            let _ = backend.remove_pending_snapshot();
            result
        }
        Ok(PendingSnapshotState::Unreadable { reason }) => {
            // 破損 / 中途書込 → ファイルの中身は無視し、安全側 (= Windows 既定
            // リセット) に倒す。
            tracing::warn!(
                "pending スナップショットが破損しています ({}). Windows 既定へリセットします",
                reason
            );
            let result = match backend.reset_to_os_default() {
                Ok(()) => {
                    tracing::info!(
                        "クラッシュリカバリ完了 (Windows 既定へリセット; unreadable snapshot)"
                    );
                    StartupRecovery::ResetToDefault
                }
                Err(e) => {
                    tracing::error!("クラッシュリカバリ (unreadable snapshot) に失敗: {}", e);
                    StartupRecovery::ResetFailed(e.to_string())
                }
            };
            let _ = backend.remove_pending_snapshot();
            result
        }
        Ok(PendingSnapshotState::Absent) => {
            tracing::debug!("pending スナップショットなし（正常）");
            StartupRecovery::NotNeeded
        }
        Err(e) => {
            tracing::warn!("pending スナップショットの確認に失敗: {}", e);
            StartupRecovery::InspectFailed(e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::cursors_dir_override_lock;

    /// snapshot 系ファイル I/O を tempdir に向ける (env はプロセス共有のため直列化)。
    struct SnapshotDir {
        _tmp: tempfile::TempDir,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl SnapshotDir {
        fn new() -> Self {
            let tmp = tempfile::TempDir::new().unwrap();
            let lock = cursors_dir_override_lock()
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            std::env::set_var("CUSTOM_CURSORS_DIR_OVERRIDE", tmp.path());
            Self {
                _tmp: tmp,
                _lock: lock,
            }
        }
    }

    impl Drop for SnapshotDir {
        fn drop(&mut self) {
            std::env::remove_var("CUSTOM_CURSORS_DIR_OVERRIDE");
        }
    }

    fn pending_path() -> std::path::PathBuf {
        crate::config::ConfigManager::cursors_dir()
            .unwrap()
            .join("_pending_apply.snapshot")
    }

    #[test]
    fn startup_recovery_absent_snapshot_is_noop() {
        let _dir = SnapshotDir::new();
        let backend = crate::platform::memory::MemoryCursorBackend::default();
        backend
            .store
            .roles
            .lock()
            .unwrap()
            .insert("Arrow".to_string(), "C:\\keep.cur".to_string());
        assert_eq!(
            recover_pending_snapshot_on_startup(&backend),
            StartupRecovery::NotNeeded
        );
        assert_eq!(
            backend
                .store
                .roles
                .lock()
                .unwrap()
                .get("Arrow")
                .map(String::as_str),
            Some("C:\\keep.cur")
        );
    }

    #[test]
    fn startup_recovery_valid_snapshot_resets_to_default_not_pre_apply_values() {
        let _dir = SnapshotDir::new();
        let backend = crate::platform::memory::MemoryCursorBackend::default();
        backend
            .store
            .roles
            .lock()
            .unwrap()
            .insert("Arrow".to_string(), "C:\\current.cur".to_string());
        // pending snapshot の original_values は使われない (経路 b の不変条件)。
        let snap = crate::registry::snapshot::RegistrySnapshot {
            schema_version: 1,
            original_values: [("Arrow".to_string(), "C:\\pre.cur".to_string())]
                .into_iter()
                .collect(),
            applied_at: "2026-01-01T00:00:00Z".to_string(),
            target_theme_id: None,
        };
        let content = serde_json::to_string_pretty(&snap).unwrap();
        std::fs::write(pending_path(), content).unwrap();
        assert_eq!(
            recover_pending_snapshot_on_startup(&backend),
            StartupRecovery::ResetToDefault
        );
        // 全役割 "" (Windows 既定) であり、pre-apply 値ではない
        let roles = backend.store.roles.lock().unwrap();
        assert_eq!(roles.len(), 17);
        assert!(roles.values().all(|v| v.is_empty()));
        assert_ne!(roles.get("Arrow").map(String::as_str), Some("C:\\pre.cur"));
        drop(roles);
        assert!(matches!(
            backend.inspect_pending_snapshot().unwrap(),
            crate::registry::PendingSnapshotState::Absent
        ));
    }

    #[test]
    fn startup_recovery_unreadable_snapshot_also_resets_to_default() {
        let _dir = SnapshotDir::new();
        let backend = crate::platform::memory::MemoryCursorBackend::default();
        std::fs::write(pending_path(), "{\"broken\"").unwrap();
        assert_eq!(
            recover_pending_snapshot_on_startup(&backend),
            StartupRecovery::ResetToDefault
        );
        assert!(backend
            .store
            .roles
            .lock()
            .unwrap()
            .values()
            .all(|v| v.is_empty()));
    }
}
