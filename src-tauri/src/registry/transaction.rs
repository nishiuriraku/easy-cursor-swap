//! EasyCursorSwap registry transaction ヘルパー (Wave 1C)
//!
//! `apply_cursors` で確立した「snapshot を書く → 17 役割を書き換える → 成功なら
//! snapshot を消す / 失敗なら snapshot から復元」のパターンを、ヘルパーに抽出して
//! 全レジストリ操作 (apply / 通常 reset / panic / initial restore) で共有する。
//!
//! ## モード
//!
//! - [`TransactionMode::NormalTransactional`]: 通常のテーマ適用 / 通常 reset / initial
//!   restore で使う。snapshot 保存失敗で mutation を中止する (= ユーザーが前の状態に
//!   確実に戻れることを保証)。
//! - [`TransactionMode::EmergencyBestEffort`]: panic ボタン (= 緊急リセット) で使う。
//!   snapshot 保存失敗をログ・通知した上で Windows Default reset を続行する。
//!   「ユーザーが今すぐ確実に元に戻したい」が目的のため、安全側 (= 続行) に倒す。
//!
//! ## commit / rollback 契約
//!
//! 1. **NormalTransactional + snapshot 失敗** → Err を返し、mutation ゼロ (一貫性保持)
//! 2. **NormalTransactional + mutation 失敗** → snapshot から復元を試み、Err を返す
//!    (= ロールバック失敗時にパニックボタン誘導をエラーに付記)
//! 3. **NormalTransactional + commit 成功** → snapshot 削除 (次回起動で leftover 検出しない)
//! 4. **EmergencyBestEffort + snapshot 失敗** → 警告ログ + mutation を続行
//!    (= ロールバック手段が無いまま進むが、緊急リセットが目的なので容認)
//!
//! 起動時 leftover snapshot (`main.rs` の `check_pending_snapshot` 経路) は
//! **Windows Default リセット** に倒す既存挙動 (= open-tasks.md gotchas の
//! 「startup leftover は pre-apply exact restore ではなく Windows default reset
//! が現行の意図的 invariant」) を維持する。本ヘルパーはその不変条件を壊さない。

use crate::errors::{AppError, AppResult};
use crate::logging;
use crate::registry::RegistryManager;
use std::collections::HashMap;
use winreg::enums::*;
use winreg::RegKey;

/// 操作モード。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionMode {
    /// 通常のテーマ適用 / 通常 reset / initial restore。
    ///
    /// snapshot 保存失敗時に Err で中断 (mutation ゼロ)。中途失敗時は snapshot から
    /// 復元を試みる。
    NormalTransactional,

    /// 緊急リセット (panic ボタン)。snapshot 保存失敗を警告ログ + ユーザー通知で
    /// 知らせるのみで、mutation を続行する (= 「今すぐ確実に戻す」が目的のため)。
    EmergencyBestEffort,
}

/// transaction ヘルパーへの入力。
///
/// `write_values`: 各役割レジストリ値名 → 書き込む値のマップ。
///   - 値が存在する役割: その値で `cursors_key.set_value` を実行
///   - `write_values` に登録されていない役割: 空文字列で上書き (Windows 既定継承)
///
/// `theme_id`: snapshot メタに保存する対象テーマ ID。`None` のときは panic / reset
/// 用途。
pub struct TransactionSpec<'a> {
    pub mode: TransactionMode,
    pub theme_id: Option<&'a str>,
    pub write_values: &'a HashMap<String, String>,
}

/// 共通の transaction を実行する。
///
/// 詳細はモジュール docstring の commit / rollback 契約を参照。
pub fn run_cursor_transaction(spec: &TransactionSpec<'_>) -> AppResult<()> {
    match spec.mode {
        TransactionMode::NormalTransactional => run_normal_transactional(spec),
        TransactionMode::EmergencyBestEffort => run_emergency_best_effort(spec),
    }
}

/// NormalTransactional: snapshot 保存 → mutation → notify → commit / rollback。
fn run_normal_transactional(spec: &TransactionSpec<'_>) -> AppResult<()> {
    // 1. 現在のレジストリ値を snapshot として保存。
    let current_values = RegistryManager::read_current_cursors()?;
    if let Err(e) = RegistryManager::save_pending_snapshot(&current_values, spec.theme_id) {
        // snapshot 失敗 → mutation 中止 (= ユーザーが元に戻れる手段が無い状態での
        // mutation は禁止)。
        return Err(AppError::Registry(format!(
            "transaction snapshot 保存失敗 (apply 中止): {}",
            e
        )));
    }

    // 2. mutation。1 役割でも失敗したら snapshot から復元。
    if let Err(e) = write_all_roles(spec.write_values) {
        tracing::error!("transaction mutation 失敗: {}", e);
        match RegistryManager::restore_from_snapshot_pub(&current_values) {
            Ok(()) => tracing::info!("ロールバック成功: 適用前値へ復元"),
            Err(re) => {
                tracing::error!("ロールバック失敗: {}", re);
                return Err(AppError::Registry(format!(
                    "transaction mutation 失敗 + ロールバック失敗: {} / Ctrl+Alt+Shift+R でリセットしてください ({})",
                    e, re
                )));
            }
        }
        // ロールバック成功時は元の mutation エラーをそのまま返す。
        let _ = RegistryManager::remove_pending_snapshot();
        return Err(e);
    }

    // 3. 即時反映 (SPI_SETCURSORS 等)。
    if let Err(e) = RegistryManager::notify_cursor_change_pub() {
        // SPI 失敗は mutation 自体は成功しているのでベストエフォート: snapshot は
        // 残す (= 次回起動で再試行 or Windows Default へリセット)。
        tracing::error!("transaction notify 失敗: {}", e);
        return Err(e);
    }

    // 4. commit 成功 → snapshot 削除。
    if let Err(e) = RegistryManager::remove_pending_snapshot() {
        // snapshot 削除失敗は warn 扱い (mutation / notify は成功済み)。
        tracing::warn!(
            "pending snapshot 削除失敗 (次回起動で re-detect される): {}",
            e
        );
    }

    tracing::info!(
        "transaction 成功 (mode=NormalTransactional, 上書き={} / 既定継承={})",
        spec.write_values.len(),
        17 - spec.write_values.len()
    );
    Ok(())
}

/// EmergencyBestEffort: snapshot 保存失敗をログ + 通知のみで続行。
///
/// 「ユーザーが今すぐ確実に元に戻したい」が目的のため、snapshot 失敗を理由に
/// mutation を止める = ユーザーが望む「即時リセット」を阻害する。緊急用途では
/// 安全側 (= 続行) に倒す。
fn run_emergency_best_effort(spec: &TransactionSpec<'_>) -> AppResult<()> {
    let mut snapshot_committed = false;
    if let Err(e) = RegistryManager::save_pending_snapshot(&HashMap::new(), spec.theme_id) {
        tracing::warn!(
            "emergency transaction: snapshot 保存失敗を無視して続行 ({}); ユーザーは手動で元に戻せません",
            logging::short_hash(format!("{}", e).as_bytes())
        );
    } else {
        snapshot_committed = true;
    }

    let mutation_result = write_all_roles(spec.write_values);
    if let Err(e) = &mutation_result {
        tracing::error!("emergency transaction: mutation 失敗 {}", e);
    }

    if let Err(e) = RegistryManager::notify_cursor_change_pub() {
        tracing::warn!("emergency transaction: notify 失敗 {}", e);
    }

    if snapshot_committed {
        if let Err(e) = RegistryManager::remove_pending_snapshot() {
            tracing::warn!("emergency transaction: snapshot 削除失敗 {}", e);
        }
    }

    mutation_result
}

/// 17 役割全部に値を書き込む。`write_values` 未登録の役割は空文字列 (= Windows 既定継承)。
fn write_all_roles(write_values: &HashMap<String, String>) -> AppResult<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let cursors_key = hkcu
        .open_subkey_with_flags("Control Panel\\Cursors", KEY_READ | KEY_WRITE)
        .map_err(|e| AppError::Registry(format!("Cursors キーを開けません: {}", e)))?;

    for role in crate::registry::CursorRole::all() {
        let name = role.registry_name();
        let value = write_values.get(name).cloned().unwrap_or_default();
        if let Err(e) = cursors_key.set_value(name, &value) {
            return Err(AppError::Registry(format!(
                "レジストリ書き込み失敗 ({}): {}",
                name, e
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::cursors_dir_override_lock;

    /// `CUSTOM_CURSORS_DIR_OVERRIDE` 用の RAII ガード。Drop で env を復元。
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn new(key: &'static str, value: &std::path::Path) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    /// 現在の 17 役割値を退避する RAII ガード。テスト終了時に必ず復元する。
    struct CursorValuesCleanup {
        original: HashMap<String, String>,
    }

    impl CursorValuesCleanup {
        fn capture() -> Self {
            Self {
                original: RegistryManager::read_current_cursors().unwrap_or_default(),
            }
        }
    }

    impl Drop for CursorValuesCleanup {
        fn drop(&mut self) {
            let _ = RegistryManager::restore_from_snapshot_pub(&self.original);
        }
    }

    /// `run_cursor_transaction(NormalTransactional)` の happy path。
    ///  1. 指定役割の値がレジストリに書き込まれている
    ///  2. 未指定役割は空文字列 (= Windows 既定継承)
    ///  3. 成功時に pending snapshot が削除されている
    ///
    /// グローバルレジストリ値を触るため `RegistryManager::apply_cursors_test_lock`
    /// で他 registry テストとシリアライズする。
    #[cfg(windows)]
    #[test]
    fn normal_transactional_happy_path_writes_and_clears_snapshot() {
        use tempfile::TempDir;

        let _lock = crate::registry::tests::apply_cursors_test_lock();
        let _override_lock = cursors_dir_override_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let tmp = TempDir::new().unwrap();
        let _env_guard = EnvGuard::new("CUSTOM_CURSORS_DIR_OVERRIDE", tmp.path());
        let _cleanup = CursorValuesCleanup::capture();

        // 実在する Windows 既定カーソルを指定 (SPI_SETCURSORS の再ロードを通すため)。
        const ARROW_CUR: &str = "C:\\Windows\\Cursors\\aero_arrow.cur";
        const IBEAM_CUR: &str = "C:\\Windows\\Cursors\\beam_i.cur";
        let mut write_values = HashMap::new();
        write_values.insert("Arrow".to_string(), ARROW_CUR.to_string());
        write_values.insert("IBeam".to_string(), IBEAM_CUR.to_string());

        let spec = TransactionSpec {
            mode: TransactionMode::NormalTransactional,
            theme_id: Some("test-theme"),
            write_values: &write_values,
        };
        run_cursor_transaction(&spec).expect("NormalTransactional happy path");

        // 1 + 2
        let current = RegistryManager::read_current_cursors().unwrap();
        assert_eq!(current.get("Arrow").map(String::as_str), Some(ARROW_CUR));
        assert_eq!(current.get("IBeam").map(String::as_str), Some(IBEAM_CUR));
        assert_eq!(current.get("Wait").map(String::as_str), Some(""));
        assert_eq!(current.get("Hand").map(String::as_str), Some(""));

        // 3
        let pending = RegistryManager::check_pending_snapshot()
            .expect("check_pending_snapshot が成功するべき");
        assert!(pending.is_none(), "commit 成功時は pending snapshot が無い");
    }

    /// `TransactionMode` のバリアントが定義済み (= リファクタ時の網羅性チェック)。
    #[test]
    fn transaction_mode_variants_are_defined() {
        // enum 自体の存在確認 (リファクタでバリアントが消えたらコンパイル失敗)。
        let _n = TransactionMode::NormalTransactional;
        let _e = TransactionMode::EmergencyBestEffort;
    }

    /// 17 役割より少ない write_values を渡しても、未指定役割は空文字列で埋まる契約。
    /// レジストリには触らず、`write_all_roles` の値マッピングだけを pure に検証する
    /// のは難しい (winreg 呼び出しが本体) ので、ここでは構造体生成で型レベルの契約を確認。
    #[test]
    fn transaction_spec_constructs_with_empty_write_values() {
        let write_values = HashMap::new();
        let spec = TransactionSpec {
            mode: TransactionMode::EmergencyBestEffort,
            theme_id: None,
            write_values: &write_values,
        };
        assert_eq!(spec.mode, TransactionMode::EmergencyBestEffort);
        assert!(spec.theme_id.is_none());
        assert!(spec.write_values.is_empty());
    }
}
