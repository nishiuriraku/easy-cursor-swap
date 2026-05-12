//! ビルドのキャンセル管理 (build_id ベース)。
//!
//! `OnceLock<Mutex<HashSet<String>>>` で「キャンセル要求済みの build_id 集合」を保持する。
//! 各 role 処理前 / 主要ステップ前にワーカーが [`is_cancelled`] を呼んで早期終了する。

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

/// キャンセル要求済みの build_id 集合。`OnceLock` で初期化、`Mutex` で同期。
fn cancel_set() -> &'static Mutex<HashSet<String>> {
    static SET: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SET.get_or_init(|| Mutex::new(HashSet::new()))
}

pub(super) fn mark_cancelled(build_id: &str) {
    if let Ok(mut s) = cancel_set().lock() {
        s.insert(build_id.to_string());
    }
}

pub(super) fn is_cancelled(build_id: &str) -> bool {
    cancel_set()
        .lock()
        .map(|s| s.contains(build_id))
        .unwrap_or(false)
}

pub(super) fn clear_cancel(build_id: &str) {
    if let Ok(mut s) = cancel_set().lock() {
        s.remove(build_id);
    }
}

/// 進行中の build を中止する。実際の中止は次のチェックポイントで行われる。
#[tauri::command]
pub fn cancel_build(build_id: String) {
    mark_cancelled(&build_id);
    tracing::info!("ビルド中止要求: {}", build_id);
}
