//! インメモリ偽装 backend (`#[cfg(test)]`)。
//!
//! `MemoryRoleStore` (原始レベル) と `MemoryCursorBackend` (操作レベル) を提供し、
//! transaction の commit / rollback 契約と起動時復旧の不変条件を Linux 上で検証する。
//! 実レジストリには一切触らない。`CUSTOM_CURSORS_DIR_OVERRIDE` で snapshot 系の
//! ファイル I/O を tempdir に向ける (registry テストと同じ流儀)。

use super::CursorBackend;
use crate::accessibility::AccessibilityConflicts;
use crate::errors::{AppError, AppResult};
use crate::registry::scheme::compute_apply_values;
use crate::registry::transaction::{
    fill_all_roles, run_cursor_transaction, RoleStore, TransactionMode, TransactionSpec,
};
use crate::registry::{clamp_cursor_base_size, snapshot, CursorRole, WindowsScheme};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize},
    Mutex,
};

/// transaction 用インメモリ原始ストア。
#[derive(Default)]
pub struct MemoryRoleStore {
    pub roles: Mutex<HashMap<String, String>>,
    pub default_scheme_name: Mutex<Option<String>>,
    pub notify_count: AtomicUsize,
    /// この役割への write_roles / restore_roles を失敗させる (rollback 経路のテスト用)。
    pub fail_on_role: Mutex<Option<String>>,
    pub fail_notify: AtomicBool,
}

impl MemoryRoleStore {
    fn should_fail(&self, role: &str) -> bool {
        self.fail_on_role.lock().unwrap().as_deref() == Some(role)
    }
}

impl RoleStore for MemoryRoleStore {
    fn read_roles(&self) -> AppResult<HashMap<String, String>> {
        Ok(self.roles.lock().unwrap().clone())
    }

    /// fail-fast: 失敗した役割で即 Err (旧 write_all_roles と同義)。
    /// 決定性のためソート順に書く (旧実装は CursorRole::all() の固定順)。
    fn write_roles(&self, values: &HashMap<String, String>) -> AppResult<()> {
        let mut keys: Vec<&String> = values.keys().collect();
        keys.sort();
        let mut roles = self.roles.lock().unwrap();
        for k in keys {
            if self.should_fail(k) {
                return Err(AppError::Registry(format!(
                    "memory store write 失敗 ({}): 注入された失敗",
                    k
                )));
            }
            roles.insert(k.clone(), values[k].clone());
        }
        Ok(())
    }

    /// best-effort: 全役割に書き進め、失敗を 1 つの Err に収集
    /// (旧 restore_from_snapshot_pub と同義)。
    fn restore_roles(&self, values: &HashMap<String, String>) -> AppResult<()> {
        let mut keys: Vec<&String> = values.keys().collect();
        keys.sort();
        let mut first_err: Option<String> = None;
        {
            let mut roles = self.roles.lock().unwrap();
            for k in keys {
                if self.should_fail(k) {
                    if first_err.is_none() {
                        first_err = Some(k.clone());
                    }
                    continue;
                }
                roles.insert(k.clone(), values[k].clone());
            }
        }
        match first_err {
            Some(k) => Err(AppError::Registry(format!(
                "memory store restore 失敗 ({}): 注入された失敗",
                k
            ))),
            None => Ok(()),
        }
    }

    fn write_default_scheme_name(&self, name: &str) -> AppResult<()> {
        *self.default_scheme_name.lock().unwrap() = Some(name.to_string());
        Ok(())
    }

    fn notify(&self) -> AppResult<()> {
        if self.fail_notify.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(AppError::Registry(
                "memory store notify 失敗: 注入された失敗".to_string(),
            ));
        }
        self.notify_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

/// 操作レベルインメモリ backend。`RegistryManager` と同じ `TransactionSpec` を
/// 組んで `run_cursor_transaction(&self.store, &spec)` を呼ぶ (mode / theme_id /
/// default_scheme_name の組み合わせは `registry/mod.rs` から写す)。
#[derive(Default)]
pub struct MemoryCursorBackend {
    pub store: MemoryRoleStore,
    pub schemes: Mutex<BTreeMap<String, HashMap<String, String>>>,
    pub base_size: Mutex<u32>,
    pub shadow: Mutex<Option<bool>>,
    pub accessibility: Mutex<AccessibilityConflicts>,
}

impl CursorBackend for MemoryCursorBackend {
    fn read_current_cursors(&self) -> AppResult<HashMap<String, String>> {
        self.store.read_roles()
    }

    fn apply_cursors(&self, cursor_paths: &HashMap<String, PathBuf>) -> AppResult<()> {
        let write_values: HashMap<String, String> = compute_apply_values(cursor_paths)
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let spec = TransactionSpec {
            mode: TransactionMode::NormalTransactional,
            theme_id: None,
            write_values: &write_values,
            default_scheme_name: None,
        };
        run_cursor_transaction(&self.store, &spec)
    }

    fn reset_to_os_default(&self) -> AppResult<()> {
        // 17 役割すべてを空文字列にする (= Windows 既定継承)。
        let mut write_values: HashMap<String, String> = HashMap::new();
        for role in CursorRole::all() {
            write_values.insert(role.registry_name().to_string(), String::new());
        }
        let spec = TransactionSpec {
            mode: TransactionMode::EmergencyBestEffort,
            theme_id: None,
            write_values: &write_values,
            default_scheme_name: Some("Windows Default"),
        };
        run_cursor_transaction(&self.store, &spec)?;
        Ok(())
    }

    fn restore_from_initial_snapshot(&self) -> AppResult<()> {
        let snapshot = snapshot::load_initial_snapshot()?;
        let spec = TransactionSpec {
            mode: TransactionMode::NormalTransactional,
            theme_id: None,
            write_values: &snapshot.original_values,
            default_scheme_name: None,
        };
        run_cursor_transaction(&self.store, &spec)?;
        Ok(())
    }

    fn notify_cursor_change(&self) -> AppResult<()> {
        self.store.notify()
    }

    fn set_cursor_shadow(&self, enabled: bool) -> AppResult<()> {
        *self.shadow.lock().unwrap() = Some(enabled);
        Ok(())
    }

    fn set_cursor_base_size(&self, size: u32) -> AppResult<u32> {
        let clamped = clamp_cursor_base_size(size);
        *self.base_size.lock().unwrap() = clamped;
        Ok(clamped)
    }

    fn register_scheme(
        &self,
        scheme_name: &str,
        cursor_paths: &HashMap<String, PathBuf>,
    ) -> AppResult<()> {
        let map: HashMap<String, String> = cursor_paths
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string_lossy().to_string()))
            .collect();
        self.schemes
            .lock()
            .unwrap()
            .insert(scheme_name.to_string(), map);
        Ok(())
    }

    fn unregister_schemes_for_theme(&self, theme_dir: &Path) -> AppResult<usize> {
        let raw = theme_dir.to_string_lossy().to_lowercase();
        if raw.is_empty() {
            return Ok(0);
        }
        let mut prefix = raw;
        if !prefix.ends_with('\\') && !prefix.ends_with('/') {
            prefix.push('\\');
        }
        let mut schemes = self.schemes.lock().unwrap();
        let doomed: Vec<String> = schemes
            .iter()
            .filter(|(_, paths)| {
                let non_empty: Vec<&String> = paths.values().filter(|p| !p.is_empty()).collect();
                !non_empty.is_empty()
                    && non_empty
                        .iter()
                        .all(|p| p.to_lowercase().starts_with(&prefix))
            })
            .map(|(name, _)| name.clone())
            .collect();
        let removed = doomed.len();
        for name in doomed {
            schemes.remove(&name);
        }
        Ok(removed)
    }

    fn list_os_schemes(&self) -> AppResult<Vec<WindowsScheme>> {
        let schemes = self.schemes.lock().unwrap();
        let mut out: Vec<WindowsScheme> = Vec::new();
        for (name, paths) in schemes.iter() {
            if !paths.values().any(|p| !p.is_empty()) {
                continue;
            }
            let mut scheme = WindowsScheme {
                name: name.clone(),
                cursor_paths: paths.clone(),
                role_count: paths.values().filter(|p| !p.is_empty()).count(),
                is_active: false,
            };
            scheme.is_active = self.paths_match_current(&scheme.cursor_paths);
            out.push(scheme);
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn apply_os_scheme(&self, scheme: &WindowsScheme) -> AppResult<()> {
        let cursor_paths: HashMap<String, PathBuf> = scheme
            .cursor_paths
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.clone(), PathBuf::from(v)))
            .collect();
        self.apply_cursors(&cursor_paths)?;

        let empty = HashMap::new();
        let spec = TransactionSpec {
            mode: TransactionMode::NormalTransactional,
            theme_id: None,
            write_values: &empty,
            default_scheme_name: Some(&scheme.name),
        };
        run_cursor_transaction(&self.store, &spec)?;
        Ok(())
    }

    fn accessibility_conflicts(&self) -> AccessibilityConflicts {
        self.accessibility.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::cursors_dir_override_lock;
    use std::sync::atomic::Ordering;

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

    fn backend_with_roles(pairs: &[(&str, &str)]) -> MemoryCursorBackend {
        let backend = MemoryCursorBackend::default();
        let mut roles = backend.store.roles.lock().unwrap();
        for (k, v) in pairs {
            roles.insert(k.to_string(), v.to_string());
        }
        drop(roles);
        backend
    }

    fn write_values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn cursor_paths(pairs: &[(&str, &str)]) -> HashMap<String, PathBuf> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), PathBuf::from(v)))
            .collect()
    }

    fn pending_state(backend: &MemoryCursorBackend) -> snapshot::PendingSnapshotState {
        backend.inspect_pending_snapshot().unwrap()
    }

    fn is_absent(backend: &MemoryCursorBackend) -> bool {
        matches!(
            pending_state(backend),
            snapshot::PendingSnapshotState::Absent
        )
    }

    #[test]
    fn apply_writes_specified_roles_and_clears_others() {
        let _dir = SnapshotDir::new();
        let backend = backend_with_roles(&[("Arrow", "C:\\old.cur")]);
        backend
            .apply_cursors(&cursor_paths(&[("Arrow", "C:\\new.cur")]))
            .unwrap();
        let roles = backend.store.roles.lock().unwrap();
        assert_eq!(roles.get("Arrow").map(String::as_str), Some("C:\\new.cur"));
        assert_eq!(roles.len(), 17);
        assert!(roles.values().filter(|v| !v.is_empty()).count() == 1);
        drop(roles);
        assert_eq!(backend.store.notify_count.load(Ordering::SeqCst), 1);
        assert!(is_absent(&backend));
    }

    #[test]
    fn apply_failure_rolls_back_to_pre_apply_values_and_removes_snapshot() {
        let _dir = SnapshotDir::new();
        let backend = backend_with_roles(&[("Arrow", "C:\\pre.cur"), ("IBeam", "C:\\pre2.cur")]);
        // current 値に無い役割で mutation を失敗させる。rollback は current 全体を
        // 戻すため fail 対象外 → exact restore される。
        *backend.store.fail_on_role.lock().unwrap() = Some("No".to_string());
        let err = backend
            .apply_cursors(&cursor_paths(&[
                ("Arrow", "C:\\new.cur"),
                ("IBeam", "C:\\new2.cur"),
            ]))
            .unwrap_err();
        assert!(
            format!("{}", err).contains("No"),
            "{}",
            err
        );
        // 経路 (a) の exact restore: 適用前値に戻っている
        let roles = backend.store.roles.lock().unwrap();
        assert_eq!(roles.get("Arrow").map(String::as_str), Some("C:\\pre.cur"));
        assert_eq!(roles.get("IBeam").map(String::as_str), Some("C:\\pre2.cur"));
        drop(roles);
        assert!(is_absent(&backend));
    }

    #[test]
    fn apply_failure_with_rollback_failure_keeps_snapshot() {
        let _dir = SnapshotDir::new();
        let backend = backend_with_roles(&[("Arrow", "C:\\pre.cur")]);
        // write と restore の両方で IBeam を失敗させる: restore_roles も
        // fail_on_role を見るため、mutation 失敗後の rollback も失敗する。
        // ただし rollback は current_values 全体を戻すので、Arrow の restore も
        // 失敗させるには Arrow を fail 対象にする必要がある。ここでは Arrow を
        // fail させ、mutation (Arrow 書込) と rollback (Arrow 復元) の両方を
        // 失敗させる。
        *backend.store.fail_on_role.lock().unwrap() = Some("Arrow".to_string());
        let err = backend
            .apply_cursors(&cursor_paths(&[("Arrow", "C:\\new.cur")]))
            .unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("Ctrl+Alt+Shift+R"), "{}", msg);
        assert!(
            matches!(
                pending_state(&backend),
                snapshot::PendingSnapshotState::Valid(_)
            ),
            "rollback 失敗時は snapshot を残す"
        );
    }

    #[test]
    fn notify_failure_keeps_snapshot() {
        let _dir = SnapshotDir::new();
        let backend = backend_with_roles(&[]);
        backend.store.fail_notify.store(true, Ordering::SeqCst);
        let err = backend
            .apply_cursors(&cursor_paths(&[("Arrow", "C:\\new.cur")]))
            .unwrap_err();
        assert!(format!("{}", err).contains("notify"), "{}", err);
        // roles は新値のまま (mutation 成功)、snapshot は残る (契約 4)
        assert_eq!(
            backend
                .store
                .roles
                .lock()
                .unwrap()
                .get("Arrow")
                .map(String::as_str),
            Some("C:\\new.cur")
        );
        assert!(matches!(
            pending_state(&backend),
            snapshot::PendingSnapshotState::Valid(_)
        ));
    }

    #[test]
    fn reset_to_os_default_clears_all_17_roles_and_writes_default_name() {
        let _dir = SnapshotDir::new();
        let backend = backend_with_roles(&[("Arrow", "C:\\x.cur"), ("IBeam", "C:\\y.cur")]);
        backend.reset_to_os_default().unwrap();
        let roles = backend.store.roles.lock().unwrap();
        assert_eq!(roles.len(), 17);
        assert!(roles.values().all(|v| v.is_empty()));
        drop(roles);
        assert_eq!(
            backend.store.default_scheme_name.lock().unwrap().as_deref(),
            Some("Windows Default")
        );
    }

    #[test]
    fn reset_to_os_default_continues_when_snapshot_dir_unwritable() {
        // override dir を存在しないパスにして snapshot 保存を失敗させる。
        // EmergencyBestEffort は mutation を続行する。
        let _lock = cursors_dir_override_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        std::env::set_var(
            "CUSTOM_CURSORS_DIR_OVERRIDE",
            "C:\\definitely\\not\\exist\\ecs-test",
        );
        let backend = MemoryCursorBackend::default();
        backend.reset_to_os_default().unwrap();
        assert_eq!(backend.store.roles.lock().unwrap().len(), 17);
        std::env::remove_var("CUSTOM_CURSORS_DIR_OVERRIDE");
    }

    #[test]
    fn initial_snapshot_round_trip() {
        let _dir = SnapshotDir::new();
        let backend = backend_with_roles(&[("Arrow", "C:\\v1.cur")]);
        backend.save_initial_snapshot().unwrap();
        backend
            .apply_cursors(&cursor_paths(&[("Arrow", "C:\\v2.cur")]))
            .unwrap();
        backend.restore_from_initial_snapshot().unwrap();
        assert_eq!(
            backend
                .store
                .roles
                .lock()
                .unwrap()
                .get("Arrow")
                .map(String::as_str),
            Some("C:\\v1.cur")
        );
    }

    #[test]
    fn transaction_default_scheme_name_none_does_not_touch_default_value() {
        let _dir = SnapshotDir::new();
        let backend = MemoryCursorBackend::default();
        let write_values = write_values(&[("Arrow", "C:\\a.cur")]);
        let spec = TransactionSpec {
            mode: TransactionMode::NormalTransactional,
            theme_id: None,
            write_values: &write_values,
            default_scheme_name: None,
        };
        run_cursor_transaction(&backend.store, &spec).unwrap();
        assert_eq!(*backend.store.default_scheme_name.lock().unwrap(), None);
    }

    #[test]
    fn fill_all_roles_returns_17_entries_with_empty_defaults() {
        let filled = fill_all_roles(&write_values(&[("Arrow", "C:\\a.cur")]));
        assert_eq!(filled.len(), 17);
        assert_eq!(filled.get("Arrow").map(String::as_str), Some("C:\\a.cur"));
        assert!(filled
            .iter()
            .filter(|(k, _)| *k != "Arrow")
            .all(|(_, v)| v.is_empty()));
    }
}
