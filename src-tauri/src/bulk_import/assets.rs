//! 単一ファイル / フォルダ走査による画像アセット解決。
//!
//! Creator の「画像をまとめて選択」「フォルダから読み込む」フローで使う。
//! PNG / SVG / .cur / .ico を対応形式として扱い、`ResolvedAsset` に正規化する。

use super::{
    AssetKind, BulkImportProgress, BulkResolveRequest, BulkResolveResult, CancelRegistry,
    ResolveFailure, ResolvedAsset, MAX_FILE_BYTES, MAX_TOTAL_BYTES,
};
use crate::errors::AppError;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, State};

const SUPPORTED_EXTS: &[&str] = &["png", "svg", "cur", "ico"];

fn ext_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTS.iter().any(|s| s.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

/// `paths` を走査して対応拡張子のファイルパスのみを集める。
/// パスがファイルなら拡張子チェックの上 1 件、ディレクトリなら直下 (recursive=true なら再帰) を走査。
pub fn collect_target_files(paths: &[String], recursive: bool) -> Vec<String> {
    let mut out = Vec::new();
    for raw in paths {
        let p = Path::new(raw);
        if p.is_file() {
            if ext_supported(p) {
                out.push(raw.clone());
            }
        } else if p.is_dir() {
            walk_dir(p, recursive, &mut out);
        }
    }
    out
}

fn walk_dir(dir: &Path, recursive: bool, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_file() {
            if ext_supported(&path) {
                if let Some(s) = path.to_str() {
                    out.push(s.to_string());
                }
            }
        } else if recursive && path.is_dir() {
            walk_dir(&path, true, out);
        }
    }
}

/// 単一ファイルを `ResolvedAsset` に変換する。
pub fn resolve_one(path: &str) -> Result<ResolvedAsset, AppError> {
    let p = Path::new(path);
    let metadata = std::fs::metadata(p)
        .map_err(|e| AppError::ImageProcessing(format!("metadata 取得失敗: {}", e)))?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(AppError::OversizeFile {
            path: path.to_string(),
            size: metadata.len(),
        });
    }
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let bytes =
        std::fs::read(p).map_err(|e| AppError::ImageProcessing(format!("読み込み失敗: {}", e)))?;
    let basename = p
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    match ext.as_str() {
        "png" => resolve_png(path, basename, &bytes),
        "svg" => resolve_svg(path, basename, &bytes),
        "cur" | "ico" => resolve_cur_or_ico(path, basename, &bytes, ext == "cur"),
        _ => Err(AppError::ImageProcessing(format!("未対応拡張子: {}", ext))),
    }
}

fn resolve_png(path: &str, basename: String, bytes: &[u8]) -> Result<ResolvedAsset, AppError> {
    if bytes.len() < 8 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(AppError::ImageProcessing("PNG マジックバイト不一致".into()));
    }
    let img = image::load_from_memory(bytes)
        .map_err(|e| AppError::ImageProcessing(format!("PNG decode 失敗: {}", e)))?;
    let size = img.width().min(img.height());
    Ok(ResolvedAsset {
        source_file: basename,
        source_path: path.to_string(),
        kind: AssetKind::Png,
        png_bytes: bytes.to_vec(),
        svg_text: None,
        native_size: size,
        hotspot_x: 0,
        hotspot_y: 0,
        available_sizes: vec![size],
    })
}

fn resolve_svg(path: &str, basename: String, bytes: &[u8]) -> Result<ResolvedAsset, AppError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| AppError::ImageProcessing(format!("SVG が UTF-8 ではありません: {}", e)))?
        .to_string();
    Ok(ResolvedAsset {
        source_file: basename,
        source_path: path.to_string(),
        kind: AssetKind::Svg,
        png_bytes: Vec::new(),
        svg_text: Some(text),
        native_size: 256,
        hotspot_x: 0,
        hotspot_y: 0,
        available_sizes: vec![256],
    })
}

fn resolve_cur_or_ico(
    path: &str,
    basename: String,
    bytes: &[u8],
    is_cur_hint: bool,
) -> Result<ResolvedAsset, AppError> {
    let parsed = crate::cursor::parse_ico_cur(bytes)?;
    let available_sizes: Vec<u32> = parsed.entries.iter().map(|e| e.width).collect();
    let (largest, png_bytes) = crate::cursor::pick_largest_as_png(&parsed)?;
    let kind = if is_cur_hint || parsed.is_cur {
        AssetKind::Cur
    } else {
        AssetKind::Ico
    };
    Ok(ResolvedAsset {
        source_file: basename,
        source_path: path.to_string(),
        kind,
        png_bytes,
        svg_text: None,
        native_size: largest.width,
        hotspot_x: largest.hotspot_x,
        hotspot_y: largest.hotspot_y,
        available_sizes,
    })
}

/// 複数の入力パスから対応形式のファイルを集めて `ResolvedAsset` のバッチを生成する。
/// `on_progress` が `Some` の場合、各ファイル処理ごとに進捗イベントを発火する。
/// 合計サイズが `MAX_TOTAL_BYTES` を超えた場合は break して以降を `failures` 行きとはせずに打ち切る。
pub fn bulk_resolve_inner(
    paths: &[String],
    recursive: bool,
    job_id: &str,
    on_progress: Option<&dyn Fn(BulkImportProgress)>,
) -> Result<BulkResolveResult, AppError> {
    let files = collect_target_files(paths, recursive);
    if files.is_empty() {
        return Err(AppError::NoSupportedFiles {
            path: paths.first().cloned().unwrap_or_default(),
        });
    }
    let total = files.len() as u32;
    let mut assets = Vec::new();
    let mut failures = Vec::new();
    let mut total_bytes: u64 = 0;

    for (idx, path) in files.iter().enumerate() {
        if let Some(cb) = on_progress {
            cb(BulkImportProgress {
                job_id: job_id.to_string(),
                stage: "parse",
                current: idx as u32,
                total,
                message: Some(path.clone()),
            });
        }
        match resolve_one(path) {
            Ok(asset) => {
                total_bytes = total_bytes.saturating_add(asset.png_bytes.len() as u64);
                if total_bytes > MAX_TOTAL_BYTES {
                    failures.push(ResolveFailure {
                        source_path: path.clone(),
                        reason: "総容量制限超過".into(),
                    });
                    break;
                }
                assets.push(asset);
            }
            Err(e) => {
                failures.push(ResolveFailure {
                    source_path: path.clone(),
                    reason: e.to_string(),
                });
            }
        }
    }

    if let Some(cb) = on_progress {
        cb(BulkImportProgress {
            job_id: job_id.to_string(),
            stage: "done",
            current: total,
            total,
            message: None,
        });
    }
    Ok(BulkResolveResult { assets, failures })
}

/// クリエイター一括インポートのメイン IPC。
/// `bulk-import-progress` イベントで進捗を通知する。
#[tauri::command]
pub async fn bulk_resolve_assets(
    app: AppHandle,
    registry: State<'_, CancelRegistry>,
    req: BulkResolveRequest,
) -> Result<BulkResolveResult, AppError> {
    registry.register(&req.job_id);
    let job_id = req.job_id.clone();
    let app_clone = app.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let cb = |p: BulkImportProgress| {
            let _ = app_clone.emit("bulk-import-progress", p);
        };
        bulk_resolve_inner(&req.paths, req.recursive, &req.job_id, Some(&cb))
    })
    .await
    .map_err(|e| AppError::ImageProcessing(format!("join 失敗: {}", e)))?;

    app.state::<CancelRegistry>().drop_job(&job_id);
    result
}

/// 進行中の `bulk_resolve_assets` ジョブをキャンセルする。
#[tauri::command]
pub fn cancel_bulk_import(
    registry: State<'_, CancelRegistry>,
    job_id: String,
) -> Result<(), AppError> {
    registry.cancel(&job_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../sample-icon");
        p
    }

    /// `sample-icon` ディレクトリは git 管理外なので、ローカルに無いときは
    /// fixture 依存テストをスキップする (CI / 他開発者の環境で誤って失敗扱いにしない)。
    /// 17 個以上の `.png` が見つかれば「ちゃんと用意されている」と判定する。
    fn fixture_available() -> bool {
        let dir = fixture_dir();
        if !dir.is_dir() {
            return false;
        }
        let files = collect_target_files(&[dir.to_string_lossy().to_string()], false);
        files.iter().filter(|p| p.ends_with(".png")).count() >= 17
    }

    #[test]
    fn collect_files_non_recursive_finds_pngs() {
        if !fixture_available() {
            eprintln!("skipping: sample-icon fixture not present");
            return;
        }
        let dir = fixture_dir();
        let files = collect_target_files(&[dir.to_string_lossy().to_string()], false);
        let pngs: Vec<_> = files.iter().filter(|p| p.ends_with(".png")).collect();
        assert!(
            pngs.len() >= 17,
            "expected >=17 PNGs in sample-icon, got {}",
            pngs.len()
        );
    }

    #[test]
    fn collect_files_skips_unsupported_extensions() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("ok.png"), b"x").unwrap();
        fs::write(tmp.path().join("readme.txt"), b"x").unwrap();
        fs::write(tmp.path().join("foo.exe"), b"x").unwrap();
        let files = collect_target_files(&[tmp.path().to_string_lossy().to_string()], false);
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn resolve_one_png_returns_native_size() {
        if !fixture_available() {
            eprintln!("skipping: sample-icon fixture not present");
            return;
        }
        let mut p = fixture_dir();
        p.push("easy-cursor-swap-mint__Arrow.png");
        let asset = resolve_one(&p.to_string_lossy()).unwrap();
        assert_eq!(asset.kind, AssetKind::Png);
        // sample-icon の PNG はすべて 128x128
        assert_eq!(asset.native_size, 128);
        assert_eq!(asset.hotspot_x, 0);
        assert_eq!(asset.source_file, "easy-cursor-swap-mint__Arrow.png");
        assert!(asset.svg_text.is_none());
        assert!(!asset.png_bytes.is_empty());
    }

    #[test]
    fn resolve_one_oversize_returns_err() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("big.png");
        std::fs::write(&path, vec![0u8; (MAX_FILE_BYTES + 1) as usize]).unwrap();
        let err = resolve_one(&path.to_string_lossy()).unwrap_err();
        match err {
            crate::errors::AppError::OversizeFile { .. } => {}
            other => panic!("expected OversizeFile, got {:?}", other),
        }
    }

    #[test]
    fn bulk_resolve_with_sample_dir_returns_17_assets() {
        if !fixture_available() {
            eprintln!("skipping: sample-icon fixture not present");
            return;
        }
        let dir = fixture_dir();
        let result = bulk_resolve_inner(
            &[dir.to_string_lossy().to_string()],
            false,
            "test-job",
            None,
        )
        .unwrap();
        assert!(
            result.assets.len() >= 17,
            "expected >=17, got {}",
            result.assets.len()
        );
        assert!(
            result.failures.is_empty(),
            "no failures expected, got {:?}",
            result.failures
        );
    }

    #[test]
    fn bulk_resolve_with_oversize_collects_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("big.png");
        std::fs::write(&path, vec![0u8; (MAX_FILE_BYTES + 1) as usize]).unwrap();
        let small = tmp.path().join("ok.png");
        // 1x1 PNG (8byte signature + IHDR + IDAT + IEND の最小限)
        let one_pix = include_bytes!("../../tests/fixtures/1x1.png");
        std::fs::write(&small, one_pix).unwrap();

        let result = bulk_resolve_inner(
            &[tmp.path().to_string_lossy().to_string()],
            false,
            "test-job-2",
            None,
        )
        .unwrap();
        assert_eq!(result.assets.len(), 1);
        assert_eq!(result.failures.len(), 1);
    }
}
