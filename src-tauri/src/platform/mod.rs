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

#[cfg(windows)]
pub mod windows;
#[cfg(not(windows))]
pub mod noop;
#[cfg(test)]
pub mod memory;

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
