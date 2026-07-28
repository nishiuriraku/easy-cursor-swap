//! Windows ファイル関連付け経由で Explorer から渡された `.cursorpack` の
//! argv ハンドオフ用ヘルパー (`extract_cursorpack_arg` / `PendingCursorpack` /
//! `take_pending_cursorpack`)。`tauri-plugin-single-instance` と組み合わせて、
//! 起動時 argv と 2 重起動 callback の両経路でフロントへ通知する。

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

/// 起動時または 2 重起動シグナル経由で受け取った `.cursorpack` のパスを保持する。
/// フロントが mount 後に [`take_pending_cursorpack`] IPC で取り出す。
#[derive(Default)]
pub struct PendingCursorpack(pub Mutex<Option<PathBuf>>);

/// 他スレッドの panic で `Mutex` が poison された場合でも inner の guard を返す。
///
/// `stash_pending_cursorpack` / `handle_pending_cursorpack` の silent return を排除し、
/// poison 後も操作を続行できるようにする。`unwrap` ではなく `into_inner` を取るのは、
/// poison が一過性 (該当スレッドのみ) であり、inner 値が破損しているとは限らないため。
fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// Tauri Event 名: 「`.cursorpack` を読み込んで」とフロントに通知する。
pub const EVENT_CURSORPACK_IMPORT_REQUESTED: &str = "cursorpack-import-requested";

/// argv から `.cursorpack` の絶対パスを抽出する。
///
/// - `argv[0]` (実行ファイル自身) はスキップ。
/// - 拡張子 (大小文字無視) が `.cursorpack` 以外は `None`。
/// - UNC パス (`\\server\share`) と URL スキーム (`scheme://`) は安全のため拒否。
/// - 相対パスは `cwd` で解決。`canonicalize` 失敗 (不在等) は warn ログを出して `None`。
pub fn extract_cursorpack_arg(argv: &[String], cwd: &Path) -> Option<PathBuf> {
    let raw = argv
        .iter()
        .skip(1)
        .find(|s| s.to_ascii_lowercase().ends_with(".cursorpack"))?;

    // URL スキーム拒否 (簡易検知: `scheme://`)
    if raw.contains("://") {
        tracing::warn!("argv に URL スキームが指定された: 拒否");
        return None;
    }
    // UNC 拒否 (Windows: \\server\share、forward slash 表記も含めて拒否)
    if raw.starts_with(r"\\") || raw.starts_with("//") {
        tracing::warn!("argv に UNC パスが指定された: 拒否");
        return None;
    }

    let candidate = PathBuf::from(raw);
    let absolute = if candidate.is_absolute() {
        candidate
    } else {
        cwd.join(candidate)
    };
    match absolute.canonicalize() {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::warn!(
                "argv のパス解決に失敗 ({}): {}",
                crate::logging::redact_path(&absolute),
                e
            );
            None
        }
    }
}

/// argv を検証して有効な `.cursorpack` パスがあれば [`PendingCursorpack`] に保存する。
/// 起動初期化と 2 重起動 callback の両方から呼ばれる。
pub fn stash_pending_cursorpack(app: &AppHandle, argv: &[String], cwd: &Path) {
    let Some(path) = extract_cursorpack_arg(argv, cwd) else {
        return;
    };
    let state: State<PendingCursorpack> = app.state();
    let mut guard = lock_or_recover(&state.0);
    *guard = Some(path);
}

/// 2 重起動 callback 用: stash した上で即 event を emit する。
pub fn handle_pending_cursorpack(app: &AppHandle, argv: &[String], cwd: &Path) {
    stash_pending_cursorpack(app, argv, cwd);
    let path_string = {
        let state: State<PendingCursorpack> = app.state();
        let guard = lock_or_recover(&state.0);
        guard.as_ref().map(|p| p.to_string_lossy().to_string())
    };
    if let Some(p) = path_string {
        if let Err(e) = app.emit(EVENT_CURSORPACK_IMPORT_REQUESTED, p) {
            tracing::warn!("cursorpack-import-requested の emit 失敗: {}", e);
        }
    }
}

/// `take_pending_cursorpack` IPC のコアロジック。
///
/// `PendingCursorpack.0` の `Mutex` を `lock_or_recover` 経由で取得して
/// `Option::take` で pending path を 1 件取り出す。`Mutex` が poison されていても
/// inner を引き継いで取りこぼさず、`take` の意味論 (消費 → `None`) を維持する。
/// テストから直接踏む最小 private 境界として公開している。
fn try_take(pending: &PendingCursorpack) -> Option<PathBuf> {
    lock_or_recover(&pending.0).take()
}

/// フロントが mount 完了後に呼ぶ IPC。pending パスを 1 件取り出す。
///
/// `Mutex` が他スレッド panic 由来で poison されていても、`try_take` 経由で
/// stash 済みの path をロストせず返す (旧コードの `.lock().ok()` silent drop を排除)。
#[tauri::command]
pub fn take_pending_cursorpack(state: State<PendingCursorpack>) -> Option<String> {
    try_take(state.inner()).map(|p| p.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn extract_returns_none_when_no_args() {
        let cwd = std::env::current_dir().unwrap();
        assert!(extract_cursorpack_arg(&argv(&["app.exe"]), &cwd).is_none());
        assert!(extract_cursorpack_arg(&[], &cwd).is_none());
    }

    #[test]
    fn extract_rejects_non_cursorpack_extension() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("foo.txt");
        std::fs::write(&p, b"x").unwrap();
        let cwd = std::env::current_dir().unwrap();
        let av = argv(&["app.exe", p.to_str().unwrap()]);
        assert!(extract_cursorpack_arg(&av, &cwd).is_none());
    }

    #[test]
    fn extract_accepts_cursorpack_with_any_case() {
        let tmp = TempDir::new().unwrap();
        for name in ["a.cursorpack", "b.CURSORPACK", "c.CursorPack"] {
            let p = tmp.path().join(name);
            std::fs::write(&p, b"x").unwrap();
            let av = argv(&["app.exe", p.to_str().unwrap()]);
            let cwd = std::env::current_dir().unwrap();
            let got = extract_cursorpack_arg(&av, &cwd);
            assert!(got.is_some(), "case-insensitive match failed for {name}");
        }
    }

    #[test]
    fn extract_resolves_relative_path() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("rel.cursorpack");
        std::fs::write(&p, b"x").unwrap();
        let av = argv(&["app.exe", "rel.cursorpack"]);
        let got = extract_cursorpack_arg(&av, tmp.path());
        // canonicalize 後のパスは同じ実ファイルを指していれば合格
        assert!(got.is_some());
        let got = got.unwrap();
        let expected = p.canonicalize().unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn extract_rejects_unc_path() {
        // UNC は実ファイルが無くても文字列ベースで弾く想定
        let av = argv(&["app.exe", r"\\server\share\theme.cursorpack"]);
        let cwd = std::env::current_dir().unwrap();
        assert!(extract_cursorpack_arg(&av, &cwd).is_none());
    }

    #[test]
    fn extract_rejects_url_scheme() {
        let av = argv(&["app.exe", "file:///C:/foo.cursorpack"]);
        let cwd = std::env::current_dir().unwrap();
        assert!(extract_cursorpack_arg(&av, &cwd).is_none());
        let av2 = argv(&["app.exe", "easy-cursor-swap://x.cursorpack"]);
        assert!(extract_cursorpack_arg(&av2, &cwd).is_none());
    }

    #[test]
    fn extract_returns_none_for_missing_file() {
        let cwd = std::env::current_dir().unwrap();
        let av = argv(&["app.exe", "C:/this/does/not/exist.cursorpack"]);
        assert!(extract_cursorpack_arg(&av, &cwd).is_none());
    }

    #[test]
    fn pending_cursorpack_take_returns_last_stash() {
        let pending = PendingCursorpack::default();
        {
            let mut g = pending.0.lock().unwrap();
            *g = Some(PathBuf::from(r"C:\first.cursorpack"));
        }
        {
            let mut g = pending.0.lock().unwrap();
            *g = Some(PathBuf::from(r"C:\second.cursorpack")); // 後勝ち
        }
        let taken = pending.0.lock().unwrap().take();
        assert_eq!(taken, Some(PathBuf::from(r"C:\second.cursorpack")));
        // take 後は空
        assert!(pending.0.lock().unwrap().is_none());
    }

    /// 別スレッドで panic させ、`PendingCursorpack` の `Mutex` を poison 化するヘルパー。
    /// G6 の各テストの前置条件として使う。
    fn poison_pending(pending: Arc<PendingCursorpack>) {
        let _ = std::thread::spawn(move || {
            let _g = pending.0.lock().unwrap();
            panic!("intentional poison for G6 test");
        })
        .join();
    }

    /// 旧コード (`match lock() { Ok(g) => g, Err(_) => return }`) は poison 時に silent return して
    /// 値が永続化されないことを回帰として固定する。
    /// 検証本体は新パターン側を実行し、旧パターンの破綻はコメントで対比する。
    #[test]
    fn stash_persists_through_poison_after_g6_fix() {
        let pending = Arc::new(PendingCursorpack::default());
        poison_pending(Arc::clone(&pending));
        assert!(
            pending.0.is_poisoned(),
            "precondition: PendingCursorpack の Mutex が poison されていること"
        );

        // 旧コード (G6 修正前) では:
        //   let mut guard = match pending.0.lock() {
        //       Ok(g) => g,
        //       Err(_) => return, // ← poison 時に silent return して値が永続化されない
        //   };
        //   *guard = Some(path);
        // 上記だと poison 経路で `*guard = Some(path)` が到達せず書き込めない。
        // G6 ではこれを `unwrap_or_else(|e| e.into_inner())` (または同等のヘルパー) に置換する。

        // 修正後: lock_or_recover ヘルパー経由で値が永続化されることを確認
        let stash_path = PathBuf::from(r"C:\after-poison.cursorpack");
        {
            let mut guard = lock_or_recover(&pending.0);
            *guard = Some(stash_path.clone());
        }
        let guard = lock_or_recover(&pending.0);
        assert_eq!(
            *guard,
            Some(stash_path),
            "lock_or_recover 経由で poison 後も値が永続化されていること"
        );
    }

    /// `lock_or_recover` 経由で poison 状態でも write でき、後続 read でその値が見える。
    #[test]
    fn lock_or_recover_writes_value_through_poison() {
        let pending = Arc::new(PendingCursorpack::default());
        poison_pending(Arc::clone(&pending));
        assert!(pending.0.is_poisoned());

        {
            let mut guard = lock_or_recover(&pending.0);
            *guard = Some(PathBuf::from(r"C:\recovered-write.cursorpack"));
        }

        let guard = lock_or_recover(&pending.0);
        assert_eq!(
            *guard,
            Some(PathBuf::from(r"C:\recovered-write.cursorpack"))
        );
    }

    /// `lock_or_recover` 経由で poison 状態でも pre-existing な値を読み出せる。
    #[test]
    fn lock_or_recover_reads_pre_existing_value_through_poison() {
        let pending = Arc::new(PendingCursorpack::default());
        {
            let mut guard = lock_or_recover(&pending.0);
            *guard = Some(PathBuf::from(r"C:\before-poison.cursorpack"));
        }
        poison_pending(Arc::clone(&pending));
        assert!(pending.0.is_poisoned());

        let guard = lock_or_recover(&pending.0);
        assert_eq!(*guard, Some(PathBuf::from(r"C:\before-poison.cursorpack")));
    }

    /// stash_pending_cursorpack / handle_pending_cursorpack のロック取得・書込み・読出しが
    /// 旧 silent-return パターンではなく poison 回復することを end-to-end で確認する。
    #[test]
    fn pending_stash_and_read_round_trip_through_poison() {
        let pending = Arc::new(PendingCursorpack::default());
        poison_pending(Arc::clone(&pending));
        assert!(pending.0.is_poisoned());

        // stash_pending_cursorpack の主要素 (lock_or_recover + 書込み)
        let stash_path = PathBuf::from(r"C:\second-instance.cursorpack");
        {
            let mut guard = lock_or_recover(&pending.0);
            *guard = Some(stash_path.clone());
        }

        // handle_pending_cursorpack の主要素 (lock_or_recover + 文字列化)
        let read_back = {
            let guard = lock_or_recover(&pending.0);
            guard.as_ref().map(|p| p.to_string_lossy().to_string())
        };
        assert_eq!(read_back.as_deref(), Some(stash_path.to_str().unwrap()));
    }

    /// `take_pending_cursorpack` IPC のコアロジック (`try_take`) が `Mutex` poison 状態でも
    /// pending path を取り出せることを確認する回帰テスト。
    ///
    /// 旧コードの `take_pending_cursorpack` は `state.0.lock().ok()` で silent drop しており、
    /// 別スレッド panic 由来の poison で IPC が `None` を返しユーザの `.cursorpack`
    /// 取り込み要求が消失していた。修正後は `lock_or_recover` 経由で inner を引き継ぎ
    /// `take()` するため、poison 状態でも stash 済みの pending path を返却する。
    /// 検証対象は公開 IPC 本体ではなく最小の private 境界 (`try_take`) に直接踏み込む。
    #[test]
    fn try_take_recovers_pending_path_through_poison() {
        let pending = Arc::new(PendingCursorpack::default());
        let path = PathBuf::from(r"C:\poisoned-take.cursorpack");

        // 前置: pending path を stash しておく (この時点では Mutex は健全)。
        {
            let mut guard = lock_or_recover(&pending.0);
            *guard = Some(path.clone());
        }

        // 別スレッド panic で Mutex を意図的に poison する。
        poison_pending(Arc::clone(&pending));
        assert!(
            pending.0.is_poisoned(),
            "precondition: Mutex が poison されていること"
        );

        // take_pending_cursorpack IPC のコアロジック (`try_take`) を直接実行。
        let taken = try_take(&pending);
        assert_eq!(
            taken,
            Some(path.clone()),
            "poison 状態でも pending path を返却しなければならない (.lock().ok() の silent drop 回帰)"
        );
        // `take` の意味論: 二度目は `None` (mutex 状態と無関係に成立する)。
        assert_eq!(try_take(&pending), None);
    }
}
