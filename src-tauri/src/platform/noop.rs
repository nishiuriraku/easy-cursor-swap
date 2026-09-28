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
    fn read_current_cursors(&self) -> AppResult<HashMap<String, String>> {
        Err(AppError::UnsupportedPlatform(
            "read_current_cursors".to_string(),
        ))
    }
    fn apply_cursors(&self, _cursor_paths: &HashMap<String, PathBuf>) -> AppResult<()> {
        Err(AppError::UnsupportedPlatform("apply_cursors".to_string()))
    }
    fn reset_to_os_default(&self) -> AppResult<()> {
        Err(AppError::UnsupportedPlatform(
            "reset_to_os_default".to_string(),
        ))
    }
    fn restore_from_initial_snapshot(&self) -> AppResult<()> {
        Err(AppError::UnsupportedPlatform(
            "restore_from_initial_snapshot".to_string(),
        ))
    }
    fn notify_cursor_change(&self) -> AppResult<()> {
        Err(AppError::UnsupportedPlatform(
            "notify_cursor_change".to_string(),
        ))
    }
    fn set_cursor_shadow(&self, _enabled: bool) -> AppResult<()> {
        Err(AppError::UnsupportedPlatform(
            "set_cursor_shadow".to_string(),
        ))
    }
    fn set_cursor_base_size(&self, _size: u32) -> AppResult<u32> {
        Err(AppError::UnsupportedPlatform(
            "set_cursor_base_size".to_string(),
        ))
    }
    fn register_scheme(
        &self,
        _scheme_name: &str,
        _cursor_paths: &HashMap<String, PathBuf>,
    ) -> AppResult<()> {
        Err(AppError::UnsupportedPlatform("register_scheme".to_string()))
    }
    fn unregister_schemes_for_theme(&self, _theme_dir: &Path) -> AppResult<usize> {
        Err(AppError::UnsupportedPlatform(
            "unregister_schemes_for_theme".to_string(),
        ))
    }
    fn list_os_schemes(&self) -> AppResult<Vec<WindowsScheme>> {
        Err(AppError::UnsupportedPlatform("list_os_schemes".to_string()))
    }
    fn apply_os_scheme(&self, _scheme: &WindowsScheme) -> AppResult<()> {
        Err(AppError::UnsupportedPlatform("apply_os_scheme".to_string()))
    }
    fn accessibility_conflicts(&self) -> AccessibilityConflicts {
        AccessibilityConflicts::default()
    }
}
