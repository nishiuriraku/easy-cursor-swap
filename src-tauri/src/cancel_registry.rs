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

    /// ジョブのキャンセルを要求する。register 前でもエントリを作る。
    pub fn cancel(&self, job_id: &str) {
        if let Ok(mut g) = self.inner.lock() {
            g.entry(job_id.to_string())
                .and_modify(|v| *v = false)
                .or_insert(false);
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
    fn cancel_before_register_marks_cancelled() {
        let r = CancelRegistry::default();
        r.cancel("job2");
        assert!(r.is_cancelled("job2"));
        assert!(!r.is_active("job2"));
    }

    #[test]
    fn distinct_jobs_do_not_interfere() {
        let r = CancelRegistry::default();
        r.register("a");
        r.cancel("b");
        assert!(r.is_active("a"));
        assert!(r.is_cancelled("b"));
        assert!(!r.is_active("b"));
        assert!(!r.is_cancelled("a"));
    }
}
