//! EasyCursorSwap レジストリスナップショット I/O (Wave 2AB Task 8)
//!
//! `RegistrySnapshot` の永続化・読み込み・削除、および初回起動時の
//! `~/.custom_cursors/_initial_snapshot.json` の読み込みをここに集約する。
//!
//! ## 構成
//!
//! - [`RegistrySnapshot`]: pending / initial スナップショット共通の JSON 表現
//! - [`PendingSnapshotState`]: 起動時チェックの結果を示す 3 状態 enum
//!   ([`PendingSnapshotState::Absent`] / [`PendingSnapshotState::Valid`] /
//!   [`PendingSnapshotState::Unreadable`])
//!
//! ## 原子書込み
//!
//! すべてのスナップショット書込は temp ファイル → `fs::rename` で本ファイルに置換
//! する「atomic temp-write/rename」パターンを使う ([`atomic_write`])。電源断 /
//! プロセス落ちで書込が中断しても、本ファイルは常に「旧版」か「新版」のいずれか
//! が完全に観測できる状態を維持する。これにより `check_pending_snapshot` /
//! `inspect_pending_snapshot` が中途半端な JSON を読み取って panic することを防ぐ。
//!
//! ## Invariant
//!
//! - **書込 → 読込 → 削除** の順序を厳守する。
//!   - mutation の **前** に `save_pending_snapshot` する (snapshot-before-mutate)。
//!   - mutation 成功 **後** に `remove_pending_snapshot` する (delete-on-success)。
//! - 起動時 leftover snapshot (`PendingSnapshotState::Valid` または
//!   `PendingSnapshotState::Unreadable`) は **適用前値の復元ではなく**
//!   Windows 既定リセットを選択する。これは「レジストリが混在状態である可能性が
//!   あるため、適用前値復元では不整合が残る」という意図的な安全選択
//!   (CLAUDE.md 「Critical invariants」 / open-tasks.md gotchas 参照)。
//!
//! ## このモジュールが行わないこと
//!
//! - HKCU の 17 役割レジストリ書込 (`crate::registry::transaction` 側の責務)
//! - トレイ・IPC イベント発火 (`crate::commands::system` 側の責務)
//! - `~/.custom_cursors/_initial_snapshot.json` の **存在判定と救出**以外の
//!   `restore_from_initial_snapshot` の Registry 書き戻し (= 役割値を書き戻す
//!   mutation) はこちらではなく transaction 経由で行う

use crate::config::ConfigManager;
use crate::errors::{AppError, AppResult};
use crate::registry::RegistryManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 起動時に観測する pending snapshot の状態。
///
/// 旧実装の `check_pending_snapshot() -> AppResult<Option<RegistrySnapshot>>` は
/// JSON パース失敗 (= ファイル破損) を `Err` で返していた。これだと呼び出し元は
/// 「破損を Err で握って無視」か「Err を propagate してアプリ起動を諦める」の
/// 2 択になりがちで、いずれも Windows 既定への reset (= 安全な選択肢) を阻害する。
///
/// 新実装は「ファイルは存在するが読めない」を 1 つの状態 `Unreadable` として
/// 表現し、呼び出し元が panic せずに「Valid と同じ扱い (= Windows 既定リセット)」
/// を選べるようにする。メタデータのパース試行は診断用 (logging) のみで、起動時
/// リカバリ判定には **ファイル存在のみ** を反映する。これにより
/// 「unreadable だから何もしない」事故を防ぐ。
#[derive(Debug)]
pub enum PendingSnapshotState {
    /// pending snapshot ファイルが存在しない (= 正常状態)
    Absent,
    /// pending snapshot ファイルが存在し、JSON もパースできた (前回 apply が中断された疑い)
    Valid(RegistrySnapshot),
    /// pending snapshot ファイルが存在するが、JSON パースに失敗 (= 破損 / 中途書込)
    Unreadable { reason: String },
}

/// レジストリのスナップショット（適用トランザクション用）
///
/// `pending` と `initial` の両方で同じ型を使う。`applied_at` は pending の場合に
/// 「適用直前の snapshot 取得時刻」、initial の場合に「初回起動時の snapshot
/// 取得時刻」を保持する。`target_theme_id` は pending でのみ使う (initial は
/// 「インストール前の状態」= テーマ未確定なので None)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistrySnapshot {
    /// スキーマバージョン
    pub schema_version: u32,
    /// 各役割のカーソルファイルパス
    pub original_values: HashMap<String, String>,
    /// スナップショット取得日時
    pub applied_at: String,
    /// 適用対象のテーマID
    pub target_theme_id: Option<String>,
}

/// pending スナップショットのパス (`_pending_apply.snapshot`)
fn pending_snapshot_path() -> AppResult<PathBuf> {
    let cursors_dir = ConfigManager::cursors_dir()?;
    Ok(cursors_dir.join("_pending_apply.snapshot"))
}

/// initial スナップショットのパス (`_initial_snapshot.json`)
fn initial_snapshot_path() -> AppResult<PathBuf> {
    let cursors_dir = ConfigManager::cursors_dir()?;
    Ok(cursors_dir.join("_initial_snapshot.json"))
}

/// temp 拡張子。同一ディレクトリに `*snapshot` と `*.snapshot.tmp` が並ぶ形になり、
/// 電源断 / プロセス落ちで atomic 置換が中断しても temp ファイルが残るのみで
/// 本ファイルは無傷。
const SNAPSHOT_TEMP_SUFFIX: &str = "tmp";

/// 指定パスへファイル内容を原子的に書き込む。
///
/// temp ファイルへ書いてから `fs::rename` で 1 ステップで置換する。
/// 失敗時は temp ファイルを削除して元のパスを一切変更しない (呼び出し側が
/// in-memory ロールバックを判断する)。
///
/// 設定ファイル向けの `config.rs::atomic_write` と同じパターンだが、
/// snapshot は「JSON パース失敗 = Windows 既定リセット」が安全側の挙動なので、
/// 確実に「本ファイルは常に有効な JSON」状態を作りたいという意図は同じ。
fn atomic_write(path: &Path, content: &str) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::Registry(format!(
            "snapshot の親ディレクトリが取得できません: {}",
            crate::logging::redact_path(path)
        ))
    })?;
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    let temp = path.with_extension(SNAPSHOT_TEMP_SUFFIX);
    // 前回クラッシュ時に temp が残っていても今回書込みで上書きする前に掃除する
    // (複数 temp が累積する事態を防ぐ)。
    let _ = fs::remove_file(&temp);

    fs::write(&temp, content).map_err(|e| {
        AppError::Registry(format!(
            "snapshot temp への書き出し失敗 ({}): {}",
            crate::logging::redact_path(&temp),
            e
        ))
    })?;

    if let Err(e) = fs::rename(&temp, path) {
        // rename 失敗: temp が残骸として残らないよう掃除してからエラーを返す。
        let _ = fs::remove_file(&temp);
        return Err(AppError::Registry(format!(
            "snapshot atomic 置換失敗 ({} → {}): {}",
            crate::logging::redact_path(&temp),
            crate::logging::redact_path(path),
            e
        )));
    }
    Ok(())
}

/// pending スナップショットの存在と中身を診断する (起動時チェック)。
///
/// `PendingSnapshotState::Unreadable` は「JSON パース失敗 = ファイル破損 /
/// 中途書込」だが、**適用前値復元には使わない** (混在状態の可能性を否定できない
/// ため)。呼び出し元は `Valid` / `Unreadable` を区別せず「Windows 既定へ
/// リセット」すれば安全。
pub fn inspect_pending_snapshot() -> AppResult<PendingSnapshotState> {
    let path = pending_snapshot_path()?;
    if !path.exists() {
        return Ok(PendingSnapshotState::Absent);
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            // ファイルは存在するが読めない (権限 / OS エラー)。破損と同じく
            // "Windows 既定へリセット" ルートに乗せる。
            return Ok(PendingSnapshotState::Unreadable {
                reason: format!("read failed: {}", e),
            });
        }
    };

    match serde_json::from_str::<RegistrySnapshot>(&content) {
        Ok(snapshot) => Ok(PendingSnapshotState::Valid(snapshot)),
        Err(e) => Ok(PendingSnapshotState::Unreadable {
            reason: format!("json parse failed: {}", e),
        }),
    }
}

/// 適用前のスナップショットをディスクに保存する
/// クラッシュ時の復旧に使用
pub fn save_pending_snapshot(
    values: &HashMap<String, String>,
    theme_id: Option<&str>,
) -> AppResult<()> {
    let snapshot = RegistrySnapshot {
        schema_version: 1,
        original_values: values.clone(),
        applied_at: chrono::Utc::now().to_rfc3339(),
        target_theme_id: theme_id.map(|s| s.to_string()),
    };

    let path = pending_snapshot_path()?;
    let content = serde_json::to_string_pretty(&snapshot)?;
    atomic_write(&path, &content)?;
    Ok(())
}

/// pending スナップショットを削除する（適用成功時に呼ぶ）
pub fn remove_pending_snapshot() -> AppResult<()> {
    let path = pending_snapshot_path()?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// 初回起動時のスナップショットを保存する
pub fn save_initial_snapshot() -> AppResult<()> {
    let path = initial_snapshot_path()?;

    // 既に存在する場合は上書きしない (本物の初回以降に上書きすると
    // 「ユーザーが任意にカスタムテーマを適用した状態」が initial snapshot に
    // なってしまい、パニックボタンで「テーマ適用後」の状態に戻ってしまう。
    // initial は文字通り「インストール前の状態」だけを保存する)
    if path.exists() {
        return Ok(());
    }

    let values = RegistryManager::read_current_cursors()?;
    let snapshot = RegistrySnapshot {
        schema_version: 1,
        original_values: values,
        applied_at: chrono::Utc::now().to_rfc3339(),
        target_theme_id: None,
    };

    let content = serde_json::to_string_pretty(&snapshot)?;
    atomic_write(&path, &content)?;

    tracing::info!("初回スナップショットを保存しました");
    Ok(())
}

/// 初回スナップショットから `RegistrySnapshot` を読み込む (パースまで)。
///
/// `restore_from_initial_snapshot` のトランザクション経路で使うため、
/// 「読み込み」と「mutation」を分離している。読み込み失敗時は呼び出し元が
/// エラー処理する。
pub fn load_initial_snapshot() -> AppResult<RegistrySnapshot> {
    let path = initial_snapshot_path()?;

    if !path.exists() {
        return Err(AppError::Registry(
            "初回スナップショットが見つかりません".to_string(),
        ));
    }

    let content = fs::read_to_string(&path)?;
    let snapshot: RegistrySnapshot = serde_json::from_str(&content)?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `PendingSnapshotState` のバリアントが存在する (= リファクタ時の網羅性チェック)。
    #[test]
    fn pending_snapshot_state_variants_are_defined() {
        // バリアントが消えたらコンパイル失敗。
        let _a = PendingSnapshotState::Absent;
        let _u = PendingSnapshotState::Unreadable {
            reason: String::new(),
        };
        let mut values = HashMap::new();
        values.insert("Arrow".to_string(), String::new());
        let snapshot = RegistrySnapshot {
            schema_version: 1,
            original_values: values,
            applied_at: String::new(),
            target_theme_id: None,
        };
        let _v = PendingSnapshotState::Valid(snapshot);
    }

    /// snapshot の保存→読み込み→削除の往復で `original_values` / `target_theme_id` /
    /// `schema_version` が保持される契約。TempDir を `CUSTOM_CURSORS_DIR_OVERRIDE`
    /// に向けるだけで完結する (レジストリ非依存)。
    #[cfg(windows)]
    #[test]
    fn snapshot_lifecycle_round_trip_via_module_functions() {
        use tempfile::TempDir;

        let _override_lock = crate::config::cursors_dir_override_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let tmp = TempDir::new().unwrap();
        let prev = std::env::var("CUSTOM_CURSORS_DIR_OVERRIDE").ok();
        std::env::set_var("CUSTOM_CURSORS_DIR_OVERRIDE", tmp.path());
        // panic でも env を復元する RAII ガード
        let _restore = EnvRestore {
            key: "CUSTOM_CURSORS_DIR_OVERRIDE",
            prev,
        };

        let mut values = HashMap::new();
        values.insert("Arrow".to_string(), "C:\\snap\\arrow.cur".to_string());
        values.insert("Wait".to_string(), String::new());

        // 1. save_pending_snapshot → ファイルが存在する。
        save_pending_snapshot(&values, Some("theme-x")).expect("save_pending_snapshot");
        assert!(
            pending_snapshot_path().unwrap().exists(),
            "pending snapshot が TempDir に存在すべき"
        );

        // 2. inspect_pending_snapshot → Valid でロードできる。
        match inspect_pending_snapshot().expect("inspect_pending_snapshot") {
            PendingSnapshotState::Valid(snapshot) => {
                assert_eq!(snapshot.original_values, values);
                assert_eq!(snapshot.target_theme_id.as_deref(), Some("theme-x"));
                assert_eq!(snapshot.schema_version, 1);
            }
            other => panic!("expected Valid, got {:?}", other),
        }

        // 3. remove_pending_snapshot → Absent。
        remove_pending_snapshot().expect("remove_pending_snapshot");
        match inspect_pending_snapshot().expect("inspect_pending_snapshot") {
            PendingSnapshotState::Absent => {}
            other => panic!("expected Absent, got {:?}", other),
        }
    }

    /// 破損 JSON は `Valid` ではなく `Unreadable` として観測される契約。
    /// 旧実装 (`check_pending_snapshot`) は `Err` を返していたため、呼び出し元が
    /// 「握り潰して何もしない」事故が起きやすかった。新 enum では型で
    /// 「unreadable でも安全に Windows 既定リセットする」契約を強制する。
    #[cfg(windows)]
    #[test]
    fn inspect_pending_snapshot_returns_unreadable_on_corrupt_json() {
        use tempfile::TempDir;

        let _override_lock = crate::config::cursors_dir_override_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let tmp = TempDir::new().unwrap();
        let prev = std::env::var("CUSTOM_CURSORS_DIR_OVERRIDE").ok();
        std::env::set_var("CUSTOM_CURSORS_DIR_OVERRIDE", tmp.path());
        let _restore = EnvRestore {
            key: "CUSTOM_CURSORS_DIR_OVERRIDE",
            prev,
        };

        // 直接壊れた JSON を書く (atomic_write は使わない — 故障状態を模倣するため)。
        fs::write(
            pending_snapshot_path().unwrap(),
            "{ this is not valid json ]]]",
        )
        .expect("破損 JSON 書込");

        match inspect_pending_snapshot().expect("inspect_pending_snapshot") {
            PendingSnapshotState::Unreadable { reason } => {
                assert!(
                    reason.contains("json parse"),
                    "reason にパース失敗が含まれるべき, got: {reason}"
                );
            }
            other => panic!(
                "破損 JSON は Unreadable として観測されるべき, got {:?}",
                other
            ),
        }
    }

    /// panic 時に env を必ず復元する RAII ガード (registry/mod.rs の同名パターンと同じ)。
    struct EnvRestore {
        key: &'static str,
        prev: Option<String>,
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
