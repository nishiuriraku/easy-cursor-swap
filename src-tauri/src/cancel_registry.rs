//! 進行中の長時間ジョブ (cursorpack ビルド、bulk-import asset 解決) に対する
//! キャンセル機構を共通化したレジストリ。
//!
//! Tauri v2 の App state として `manage(CancelRegistry::default())` し、
//! 各 IPC ハンドラから `State<'_, CancelRegistry>` で受け取る。
//!
//! API は 2 つの利用パターンを両立する:
//! - **register-first**: `register(job_id)` してから `is_active` で polling する
//!   (bulk_import が使う形)
//! - **cancel-only**: `cancel(job_id)` だけ呼んで、ワーカーは `is_cancelled` で
//!   早期終了する (cursor_build が使う形)
//!
//! 内部表現は `HashMap<String, bool>` で `true = active`、`false = cancelled`。
//! 未登録の job_id は `is_active=false` / `is_cancelled=false` のニュートラル。

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct CancelRegistry {
    inner: Mutex<HashMap<String, bool>>,
}

impl CancelRegistry {
    /// ジョブを active 状態で登録する (bulk_import 系)。
    pub fn register(&self, job_id: &str) {
        if let Ok(mut g) = self.inner.lock() {
            g.insert(job_id.to_string(), true);
        }
    }

    /// ジョブのキャンセルを要求する。
    ///
    /// **登録済みのジョブにのみ作用する** (未登録 / 完了済で drop 済のジョブには
    /// エントリを作らない)。以前は `or_insert(false)` で未登録ジョブにもエントリを
    /// 作っていたが、完了後の遅延キャンセルがエントリを再生成し drop されず leak して
    /// いた (Y15)。ワーカーはジョブ開始時に必ず `register` / `register_guard` するため、
    /// 進行中のキャンセルはこの `if let Some` 経路で確実に拾える。
    pub fn cancel(&self, job_id: &str) {
        if let Ok(mut g) = self.inner.lock() {
            if let Some(v) = g.get_mut(job_id) {
                *v = false;
            }
        }
    }

    /// 「登録済 かつ キャンセルされていない」場合のみ true。
    /// (bulk_import が main loop 内で polling する形に使う)
    pub fn is_active(&self, job_id: &str) -> bool {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.get(job_id).copied())
            .unwrap_or(false)
    }

    /// 「明示的に cancel された」場合のみ true。
    /// (cursor_build が「キャンセル要求が来たか」だけ知りたいときに使う)
    pub fn is_cancelled(&self, job_id: &str) -> bool {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.get(job_id).map(|v| !*v))
            .unwrap_or(false)
    }

    /// ジョブをレジストリから削除 (完了・失敗時の cleanup)。
    pub fn drop_job(&self, job_id: &str) {
        if let Ok(mut g) = self.inner.lock() {
            g.remove(job_id);
        }
    }

    /// ジョブを active 登録し、スコープを抜けるときに自動で `drop_job` する RAII ガードを返す。
    ///
    /// 重量ワーカーは途中に多数の `?` early-return / panic 経路を持つため、手動 `drop_job`
    /// だと取りこぼしてエントリが leak する (Y15)。このガードを `let _job = ...` で握れば、
    /// 成功・エラー・キャンセルのいずれの経路でも `Drop` で確実に cleanup される。
    pub fn register_guard<'a>(&'a self, job_id: &str) -> JobGuard<'a> {
        self.register(job_id);
        JobGuard {
            registry: self,
            job_id: job_id.to_string(),
        }
    }
}

/// `register_guard` が返す RAII ガード。`Drop` 時に `drop_job` を呼ぶ。
pub struct JobGuard<'a> {
    registry: &'a CancelRegistry,
    job_id: String,
}

impl Drop for JobGuard<'_> {
    fn drop(&mut self) {
        self.registry.drop_job(&self.job_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unregistered_job_is_inactive_and_not_cancelled() {
        let r = CancelRegistry::default();
        assert!(!r.is_active("nope"));
        assert!(!r.is_cancelled("nope"));
    }

    #[test]
    fn registered_then_cancelled_flow() {
        let r = CancelRegistry::default();
        r.register("job1");
        assert!(r.is_active("job1"));
        assert!(!r.is_cancelled("job1"));

        r.cancel("job1");
        assert!(!r.is_active("job1"));
        assert!(r.is_cancelled("job1"));

        r.drop_job("job1");
        assert!(!r.is_active("job1"));
        assert!(!r.is_cancelled("job1"));
    }

    #[test]
    fn cancel_before_register_is_noop() {
        // 未登録ジョブへの cancel はエントリを作らない (完了後の遅延 cancel が
        // エントリを再生成して leak するのを防ぐため, Y15)。
        let r = CancelRegistry::default();
        r.cancel("job2");
        assert!(!r.is_cancelled("job2"));
        assert!(!r.is_active("job2"));
    }

    #[test]
    fn register_guard_drops_on_scope_exit() {
        let r = CancelRegistry::default();
        {
            let _g = r.register_guard("g1");
            assert!(r.is_active("g1"));
        }
        // ガードが Drop で drop_job を呼びエントリが消える (leak しない)
        assert!(!r.is_active("g1"));
        assert!(!r.is_cancelled("g1"));
    }

    #[test]
    fn cancel_after_drop_does_not_recreate_entry() {
        let r = CancelRegistry::default();
        {
            let _g = r.register_guard("g2");
        } // スコープ離脱で完了 (drop)
        r.cancel("g2"); // 完了後の遅延 cancel
        assert!(!r.is_cancelled("g2")); // エントリを再生成しない (Y15)
    }

    #[test]
    fn distinct_jobs_do_not_interfere() {
        let r = CancelRegistry::default();
        r.register("a");
        // cancel() は登録済みジョブにのみ作用するため "b" も register してから cancel する (Y15)。
        r.register("b");
        r.cancel("b");
        assert!(r.is_active("a"));
        assert!(r.is_cancelled("b"));
        assert!(!r.is_active("b"));
        assert!(!r.is_cancelled("a"));
    }
}
