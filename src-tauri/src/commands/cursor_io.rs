//! `.cur` / `.ico` / `.ani` 単一ファイルの読み込み IPC。
//!
//! クリエイター画面の「既存カーソルを取り込む」用途で使う。
//! バイナリパースは `crate::cursor` 側、ここはそれをラップして PNG + メタ情報を返すだけ。
//!
//! あわせて、Windows ファイル関連付け経由で Explorer から渡された `.cursorpack` の
//! argv ハンドオフ用ヘルパー (`extract_cursorpack_arg` / `PendingCursorpack` /
//! `take_pending_cursorpack`) もここに置いている。`tauri-plugin-single-instance`
//! と組み合わせて、起動時 argv と 2 重起動 callback の両経路でフロントへ通知する。

use crate::cursor::{parse_ani, parse_ico_cur, pick_largest_as_png};
use crate::errors::AppError;
use serde::Serialize;
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

/// `.cur` / `.ico` を取り込んだ結果のフロント返却型。
///
/// `pngBytes` が最大解像度のラスタ画像 (RGBA PNG)。
/// `availableSizes` は元ファイルに含まれていた全エントリの幅 (= 高さ前提)。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCursorFile {
    pub is_cur: bool,
    pub width: u32,
    pub height: u32,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    pub png_bytes: Vec<u8>,
    pub available_sizes: Vec<u32>,
}

/// `.ico` / `.cur` ファイルを読み込み、最大解像度を PNG 化して返す。
///
/// クリエイターモードで「既存の Windows カーソルを取り込む」用途。
/// 全解像度を返すと IPC ペイロードが膨らむため、最大サイズのみを返却する設計。
#[tauri::command]
pub fn import_cursor_file(path: String) -> Result<ImportedCursorFile, AppError> {
    let bytes = std::fs::read(&path).map_err(|e| {
        AppError::ImageProcessing(format!(
            "ファイル読込失敗 ({}): {}",
            crate::logging::redact_path(std::path::Path::new(&path)),
            e
        ))
    })?;
    let parsed = parse_ico_cur(&bytes)?;
    let available_sizes = parsed.entries.iter().map(|e| e.width).collect();
    let (largest, png_bytes) = pick_largest_as_png(&parsed)?;
    tracing::info!(
        "import_cursor_file: is_cur={} entries={} largest={}x{}",
        parsed.is_cur,
        parsed.entries.len(),
        largest.width,
        largest.height
    );
    Ok(ImportedCursorFile {
        is_cur: parsed.is_cur,
        width: largest.width,
        height: largest.height,
        hotspot_x: largest.hotspot_x,
        hotspot_y: largest.hotspot_y,
        png_bytes,
        available_sizes,
    })
}

/// `.ani` ファイル検査結果。クリエイター UI のプレビュー / 情報表示用。
///
/// `framePngs` は再生順 (= sequence インデックスを展開した順) ではなく、
/// 格納順 (フレーム 0..num_frames-1) に並ぶ。UI 側で `sequence` と
/// `perStepDurationsMs` を見ながら setInterval / requestAnimationFrame で
/// アニメーション再生する。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AniInspection {
    pub num_frames: u32,
    pub num_steps: u32,
    pub default_rate_jiffies: u32,
    pub per_step_durations_ms: Vec<u32>,
    pub sequence: Vec<u32>,
    pub total_duration_ms: u64,
    /// 各フレームの最大解像度 PNG バイト列
    pub frame_pngs: Vec<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
}

/// `.ani` ファイルを読み込み、フレームごとの PNG とアニメーション情報を返す。
#[tauri::command]
pub fn inspect_ani_file(path: String) -> Result<AniInspection, AppError> {
    let bytes = std::fs::read(&path).map_err(|e| {
        AppError::ImageProcessing(format!(
            "ファイル読込失敗 ({}): {}",
            crate::logging::redact_path(std::path::Path::new(&path)),
            e
        ))
    })?;
    let parsed = parse_ani(&bytes)?;

    let mut frame_pngs: Vec<Vec<u8>> = Vec::with_capacity(parsed.frames.len());
    for entry in &parsed.frames {
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        image::ImageEncoder::write_image(
            encoder,
            entry.image.as_raw(),
            entry.image.width(),
            entry.image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| AppError::ImageProcessing(format!("PNG エンコード失敗: {}", e)))?;
        frame_pngs.push(png);
    }

    let per_step_durations_ms: Vec<u32> = parsed
        .per_step_rate_jiffies
        .iter()
        .map(|j| ((*j as u64 * 1000) / 60) as u32)
        .collect();
    let total_duration_ms = parsed.total_duration_ms();

    let (width, height, hotspot_x, hotspot_y) = parsed
        .frames
        .first()
        .map(|f| (f.width, f.height, f.hotspot_x, f.hotspot_y))
        .unwrap_or((0, 0, 0, 0));

    tracing::info!(
        "inspect_ani_file: frames={} steps={} total={}ms",
        parsed.num_frames,
        parsed.num_steps,
        total_duration_ms
    );

    Ok(AniInspection {
        num_frames: parsed.num_frames,
        num_steps: parsed.num_steps,
        default_rate_jiffies: parsed.default_rate_jiffies,
        per_step_durations_ms,
        sequence: parsed.sequence,
        total_duration_ms,
        frame_pngs,
        width,
        height,
        hotspot_x,
        hotspot_y,
    })
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
