//! 非 Windows ビルド用スタブ。
//!
//! 全 fallible メソッドは `AppError::Registry` を返す (P01 のスタブと同形)。
//! P03 で `AppError::UnsupportedPlatform` へ一括置換する。

use super::CursorBackend;
use crate::accessibility::AccessibilityConflicts;
use crate::errors::{AppError, AppResult};
use crate::registry::WindowsScheme;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct NoopCursorBackend;

impl CursorBackend for NoopCursorBackend {
    // TODO(P03): UnsupportedPlatform
    fn read_current_cursors(&self) -> AppResult<HashMap<String, String>> {
        Err(AppError::Registry(
            "read_current_cursors は Windows 専用です".to_string(),
        ))
    }
    // TODO(P03): UnsupportedPlatform
    fn apply_cursors(&self, _cursor_paths: &HashMap<String, PathBuf>) -> AppResult<()> {
        Err(AppError::Registry(
            "apply_cursors は Windows 専用です".to_string(),
        ))
    }
    // TODO(P03): UnsupportedPlatform
    fn reset_to_os_default(&self) -> AppResult<()> {
        Err(AppError::Registry(
            "reset_to_os_default は Windows 専用です".to_string(),
        ))
    }
    // TODO(P03): UnsupportedPlatform
    fn restore_from_initial_snapshot(&self) -> AppResult<()> {
        Err(AppError::Registry(
            "restore_from_initial_snapshot は Windows 専用です".to_string(),
        ))
    }
    // TODO(P03): UnsupportedPlatform
    fn notify_cursor_change(&self) -> AppResult<()> {
        Err(AppError::Registry(
            "notify_cursor_change は Windows 専用です".to_string(),
        ))
    }
    // TODO(P03): UnsupportedPlatform
    fn set_cursor_shadow(&self, _enabled: bool) -> AppResult<()> {
        Err(AppError::Registry(
            "set_cursor_shadow は Windows 専用です".to_string(),
        ))
    }
    // TODO(P03): UnsupportedPlatform
    fn set_cursor_base_size(&self, _size: u32) -> AppResult<u32> {
        Err(AppError::Registry(
            "set_cursor_base_size は Windows 専用です".to_string(),
        ))
    }
    // TODO(P03): UnsupportedPlatform
    fn register_scheme(
        &self,
        _scheme_name: &str,
        _cursor_paths: &HashMap<String, PathBuf>,
    ) -> AppResult<()> {
        Err(AppError::Registry(
            "register_scheme は Windows 専用です".to_string(),
        ))
    }
    // TODO(P03): UnsupportedPlatform
    fn unregister_schemes_for_theme(&self, _theme_dir: &Path) -> AppResult<usize> {
        Err(AppError::Registry(
            "unregister_schemes_for_theme は Windows 専用です".to_string(),
        ))
    }
    // TODO(P03): UnsupportedPlatform
    fn list_os_schemes(&self) -> AppResult<Vec<WindowsScheme>> {
        Err(AppError::Registry(
            "list_os_schemes は Windows 専用です".to_string(),
        ))
    }
    // TODO(P03): UnsupportedPlatform
    fn apply_os_scheme(&self, _scheme: &WindowsScheme) -> AppResult<()> {
        Err(AppError::Registry(
            "apply_os_scheme は Windows 専用です".to_string(),
        ))
    }
    fn accessibility_conflicts(&self) -> AccessibilityConflicts {
        AccessibilityConflicts::default()
    }
}
