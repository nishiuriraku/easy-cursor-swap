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
    let mut guard = match state.0.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    *guard = Some(path);
}

/// 2 重起動 callback 用: stash した上で即 event を emit する。
pub fn handle_pending_cursorpack(app: &AppHandle, argv: &[String], cwd: &Path) {
    stash_pending_cursorpack(app, argv, cwd);
    let path_string = {
        let state: State<PendingCursorpack> = app.state();
        let guard = match state.0.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        guard.as_ref().map(|p| p.to_string_lossy().to_string())
    };
    if let Some(p) = path_string {
        if let Err(e) = app.emit(EVENT_CURSORPACK_IMPORT_REQUESTED, p) {
            tracing::warn!("cursorpack-import-requested の emit 失敗: {}", e);
        }
    }
}

/// フロントが mount 完了後に呼ぶ IPC。pending パスを 1 件取り出す。
#[tauri::command]
pub fn take_pending_cursorpack(state: State<PendingCursorpack>) -> Option<String> {
    state
        .0
        .lock()
        .ok()
        .and_then(|mut g| g.take())
        .map(|p| p.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
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
        let av2 = argv(&["app.exe", "cursor-forge://x.cursorpack"]);
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
}
