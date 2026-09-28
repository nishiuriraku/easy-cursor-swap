//! Windows 実装: 既存 `registry::RegistryManager` への 1 行委譲。
//!
//! 判定ロジックはすべて `registry/` 側に残し、ここでは trait 境界への適合だけを行う。

use super::CursorBackend;
use crate::accessibility::AccessibilityConflicts;
use crate::errors::AppResult;
use crate::registry::{RegistryManager, WindowsScheme};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct WindowsCursorBackend;

impl CursorBackend for WindowsCursorBackend {
    fn read_current_cursors(&self) -> AppResult<HashMap<String, String>> {
        RegistryManager::read_current_cursors()
    }
    fn apply_cursors(&self, cursor_paths: &HashMap<String, PathBuf>) -> AppResult<()> {
        RegistryManager::apply_cursors(cursor_paths)
    }
    fn reset_to_os_default(&self) -> AppResult<()> {
        RegistryManager::reset_to_windows_default()
    }
    fn restore_from_initial_snapshot(&self) -> AppResult<()> {
        RegistryManager::restore_from_initial_snapshot()
    }
    fn notify_cursor_change(&self) -> AppResult<()> {
        RegistryManager::notify_cursor_change_pub()
    }
    fn set_cursor_shadow(&self, enabled: bool) -> AppResult<()> {
        RegistryManager::set_cursor_shadow(enabled)
    }
    fn set_cursor_base_size(&self, size: u32) -> AppResult<u32> {
        RegistryManager::set_cursor_base_size(size)
    }
    fn register_scheme(
        &self,
        scheme_name: &str,
        cursor_paths: &HashMap<String, PathBuf>,
    ) -> AppResult<()> {
        RegistryManager::register_scheme(scheme_name, cursor_paths)
    }
    fn unregister_schemes_for_theme(&self, theme_dir: &Path) -> AppResult<usize> {
        RegistryManager::unregister_schemes_for_theme(theme_dir)
    }
    fn list_os_schemes(&self) -> AppResult<Vec<WindowsScheme>> {
        RegistryManager::list_windows_schemes()
    }
    fn apply_os_scheme(&self, scheme: &WindowsScheme) -> AppResult<()> {
        RegistryManager::apply_windows_scheme(scheme)
    }
    fn accessibility_conflicts(&self) -> AccessibilityConflicts {
        AccessibilityConflicts::detect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 委譲経路のカバレッジ用: 実レジストリ読み取りが 17 役割を返すこと。
    #[test]
    fn windows_backend_read_current_cursors_returns_17_roles() {
        let backend = WindowsCursorBackend;
        let map = backend.read_current_cursors().unwrap();
        assert_eq!(map.len(), 17);
    }
}
