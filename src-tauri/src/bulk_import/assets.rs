//! 単一ファイル / フォルダ走査による画像アセット解決。
//!
//! Creator の「画像をまとめて選択」「フォルダから読み込む」フローで使う。
//! PNG / SVG / .cur / .ico を対応形式として扱い、`ResolvedAsset` に正規化する。

use super::{
    AssetKind, BulkImportProgress, BulkResolveRequest, BulkResolveResult, CancelRegistry,
    ResolveFailure, ResolvedAsset, MAX_FILE_BYTES, MAX_TOTAL_BYTES,
};
use crate::errors::AppError;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};

const SUPPORTED_EXTS: &[&str] = &["png", "svg", "cur", "ico", "ani"];

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

/// ディレクトリを再帰走査して対応拡張子のファイルパスを集める。
///
/// symlink / junction による自己ループで無限再帰しないよう、各ディレクトリの
/// canonical path を  HashSet に記録し、2 度目の訪問では即座に return する。
/// canonicalize は symlink を 1 段展開して絶対パス化するため、`dir/loop` が `dir` 自身
/// を指す symlink なら canonical が一致して重複訪問が検出される。
/// recursive = false (既定) の挙動は変えず、トップディレクトリのみをスキャンする。
fn walk_dir(dir: &Path, recursive: bool, out: &mut Vec<String>) {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    walk_dir_inner(dir, recursive, out, &mut visited);
}

/// walk_dir の再帰本体。visited に訪問済み canonical path を積み、
/// canonicalize 失敗時 (権限なし等) は黙ってスキップする。
/// canonicalize は I/O を伴うためロックは持たず、visited は関数引数で受け渡して
/// ヒープ再確保を避ける。
fn walk_dir_inner(
    dir: &Path,
    recursive: bool,
    out: &mut Vec<String>,
    visited: &mut HashSet<PathBuf>,
) {
    // canonicalize に失敗するケース (権限なし / パスが消えた等) はそのディレクトリの
    // 中身ごとスキップして上位呼び出し側へ戻る。warning ログは G20 で別タスク扱い。
    let Ok(canonical) = dir.canonicalize() else {
        return;
    };
    if !visited.insert(canonical) {
        // 既に訪問済み → symlink/junction ループ。打ち切り。
        return;
    }
    // read_dir に失敗するケース (パスがディレクトリではなかった / 権限剥奪 / 消失等)
    // も当該ディレクトリの中身ごとスキップするが、silent skip は silent failure の
    // 温床になるので G20 で WARN を 1 行発火する。PII 不変条件に従い raw path は
    // `logging::redact_path` で `~/...` 形式に縮約してから渡す (canonicalize 側で
    // 既に visit 済みに登録済みなので、ここで return しても visited のサイズは爆発しない)。
    let Ok(rd) = std::fs::read_dir(dir) else {
        tracing::warn!(
            path = %crate::logging::redact_path(dir),
            "bulk_import walk_dir: read_dir に失敗したためこのディレクトリ配下はスキップします",
        );
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
            walk_dir_inner(&path, true, out, visited);
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
        "ani" => resolve_ani(path, basename, &bytes),
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
        asset: crate::theme::types::CursorAssetDescriptor {
            png_bytes: bytes.to_vec(),
            width: size,
            height: size,
            hotspot: crate::theme::types::Hotspot::ZERO,
        },
        svg_text: None,
        available_sizes: vec![size],
        ani: None,
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
        asset: crate::theme::types::CursorAssetDescriptor {
            png_bytes: Vec::new(),
            width: 256,
            height: 256,
            hotspot: crate::theme::types::Hotspot::ZERO,
        },
        svg_text: Some(text),
        available_sizes: vec![256],
        ani: None,
    })
}

fn resolve_ani(path: &str, basename: String, bytes: &[u8]) -> Result<ResolvedAsset, AppError> {
    use crate::cursor::parse_ani;

    let parsed = parse_ani(bytes)?;

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

    let (width, hotspot) = parsed
        .frames
        .first()
        .map(|f| {
            let hs = crate::theme::types::Hotspot::from_px(f.hotspot_x, f.hotspot_y, f.width);
            (f.width, hs)
        })
        .unwrap_or((0, crate::theme::types::Hotspot::ZERO));

    let primary_png = frame_pngs.first().cloned().unwrap_or_default();

    Ok(ResolvedAsset {
        source_file: basename,
        source_path: path.to_string(),
        kind: AssetKind::Ani,
        asset: crate::theme::types::CursorAssetDescriptor {
            png_bytes: primary_png,
            width,
            height: width,
            hotspot,
        },
        svg_text: None,
        available_sizes: vec![width],
        ani: Some(crate::theme::types::AniFrameData {
            frame_pngs,
            sequence: parsed.sequence,
            per_step_durations_ms,
            is_legacy_raw_dib: parsed.is_legacy_raw_dib,
        }),
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
        asset: crate::theme::types::CursorAssetDescriptor {
            png_bytes,
            width: largest.width,
            height: largest.height,
            hotspot: crate::theme::types::Hotspot::from_px(
                largest.hotspot_x,
                largest.hotspot_y,
                largest.width,
            ),
        },
        svg_text: None,
        available_sizes,
        ani: None,
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
    should_cancel: Option<&dyn Fn() -> bool>,
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
        // 各ファイル処理の前にキャンセル要求を polling する。要求があれば
        // 以降のファイルを処理せず即座に打ち切る。
        if let Some(check) = should_cancel {
            if check() {
                return Err(AppError::BulkImportCancelled);
            }
        }
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
                total_bytes = total_bytes.saturating_add(asset.asset.png_bytes.len() as u64);
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
    // RAII ガードで register。関数を抜けるとき (成功・エラー・join 失敗の ? 経路すべて) に
    // 自動で drop_job されるため、以前 join 失敗パスで drop_job を取りこぼしていた leak を防ぐ (Y15)。
    let _job = registry.register_guard(&req.job_id);
    let app_clone = app.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let registry = app_clone.state::<CancelRegistry>();
        let cb = |p: BulkImportProgress| {
            let _ = app_clone.emit("bulk-import-progress", p);
        };
        // ループ各反復の先頭で「明示的に cancel されたか」を polling する。
        // 未登録ジョブでは false を返す is_cancelled を使い、誤キャンセルを避ける
        // (cursor_build/stream.rs と同じキャンセル意味論)。
        let should_cancel = || registry.is_cancelled(&req.job_id);
        bulk_resolve_inner(
            &req.paths,
            req.recursive,
            &req.job_id,
            Some(&cb),
            Some(&should_cancel),
        )
    })
    .await
    .map_err(|e| AppError::ImageProcessing(format!("join 失敗: {}", e)))?;

    // drop_job は _job (RAII ガード) が return 時に確実に実行する。
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
        assert_eq!(asset.asset.width, 128);
        assert_eq!(asset.asset.height, 128);
        assert_eq!(asset.asset.hotspot, crate::theme::types::Hotspot::ZERO);
        assert_eq!(asset.source_file, "easy-cursor-swap-mint__Arrow.png");
        assert!(asset.svg_text.is_none());
        assert!(!asset.asset.png_bytes.is_empty());
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
            None,
        )
        .unwrap();
        assert_eq!(result.assets.len(), 1);
        assert_eq!(result.failures.len(), 1);
    }

    #[test]
    fn resolve_ani_returns_animation_data() {
        use crate::cursor::generate_cur_binary;
        use image::RgbaImage;
        use std::io::Write;

        let img = RgbaImage::from_pixel(16, 16, image::Rgba([200, 100, 50, 255]));
        let cur = generate_cur_binary(&[(img, 2, 3)]).unwrap();

        let mut ani: Vec<u8> = Vec::new();
        ani.extend_from_slice(b"RIFF");
        let bp = ani.len();
        ani.extend_from_slice(&0u32.to_le_bytes());
        ani.extend_from_slice(b"ACON");
        ani.extend_from_slice(b"anih");
        ani.extend_from_slice(&36u32.to_le_bytes());
        let mut h = vec![0u8; 36];
        h[0..4].copy_from_slice(&36u32.to_le_bytes());
        h[4..8].copy_from_slice(&1u32.to_le_bytes());
        h[8..12].copy_from_slice(&1u32.to_le_bytes());
        h[28..32].copy_from_slice(&6u32.to_le_bytes());
        h[32..36].copy_from_slice(&0x01u32.to_le_bytes());
        ani.extend_from_slice(&h);
        ani.extend_from_slice(b"LIST");
        let ls = 4 + 8 + cur.len();
        ani.extend_from_slice(&(ls as u32).to_le_bytes());
        ani.extend_from_slice(b"fram");
        ani.extend_from_slice(b"icon");
        ani.extend_from_slice(&(cur.len() as u32).to_le_bytes());
        ani.extend_from_slice(&cur);
        let body = (ani.len() - 8) as u32;
        ani[bp..bp + 4].copy_from_slice(&body.to_le_bytes());

        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("anim.ani");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(&ani)
            .unwrap();

        let resolved = resolve_one(path.to_str().unwrap()).expect("resolve");
        assert_eq!(resolved.kind, AssetKind::Ani);
        assert_eq!(resolved.asset.width, 16);
        // hotspot は px(2,3) を size=16 で比率化した値と一致するはず
        let expected_hotspot = crate::theme::types::Hotspot::from_px(2, 3, 16);
        assert_eq!(resolved.asset.hotspot, expected_hotspot);
        let ani_data = resolved.ani.expect("ani field");
        assert_eq!(ani_data.frame_pngs.len(), 1);
        assert!(!ani_data.is_legacy_raw_dib);
        assert_eq!(ani_data.per_step_durations_ms, vec![100]);
    }

    #[test]
    fn bulk_resolve_returns_cancelled_when_flag_set() {
        // should_cancel が true を返すと、ループはファイル処理を完了させず
        // BulkImportCancelled で打ち切らねばならない (キャンセルボタンの実機能)。
        let tmp = tempfile::tempdir().unwrap();
        let one_pix = include_bytes!("../../tests/fixtures/1x1.png");
        std::fs::write(tmp.path().join("a.png"), one_pix).unwrap();
        std::fs::write(tmp.path().join("b.png"), one_pix).unwrap();

        let cancel = || true;
        let result = bulk_resolve_inner(
            &[tmp.path().to_string_lossy().to_string()],
            false,
            "cancel-job",
            None,
            Some(&cancel),
        );
        match result {
            Err(AppError::BulkImportCancelled) => {}
            other => panic!("expected BulkImportCancelled, got {:?}", other),
        }
    }

    #[test]
    fn bulk_resolve_completes_when_cancel_flag_false() {
        // should_cancel が常に false なら通常どおり完走する (誤キャンセルしない)。
        let tmp = tempfile::tempdir().unwrap();
        let one_pix = include_bytes!("../../tests/fixtures/1x1.png");
        std::fs::write(tmp.path().join("a.png"), one_pix).unwrap();

        let no_cancel = || false;
        let result = bulk_resolve_inner(
            &[tmp.path().to_string_lossy().to_string()],
            false,
            "live-job",
            None,
            Some(&no_cancel),
        )
        .unwrap();
        assert_eq!(result.assets.len(), 1);
    }

    /// Windows でディレクトリ symlink が作成できない環境 (Developer Mode オフ / 非管理者) では
    /// フィクスチャ作成自体が成立しないので、その場合はテストをスキップする。
    /// CI (GitHub Actions Windows runner) とローカル開発者の双方で再現できるよう、
    /// symlink 生成成否を `Ok(true)` / `Ok(false)` で呼び分け側へ伝える。
    #[cfg(windows)]
    fn try_make_dir_symlink(link: &Path, target: &Path) -> std::io::Result<bool> {
        use std::os::windows::fs::symlink_dir;
        match symlink_dir(target, link) {
            Ok(()) => Ok(true),
            Err(e)
                if e.kind() == std::io::ErrorKind::PermissionDenied
                    || e.kind() == std::io::ErrorKind::Unsupported =>
            {
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// `tracing` 出力を文字列としてキャプチャする簡易 helper。
    /// `commands/theme.rs::tests` の同名 helper とは独立。`tracing::subscriber` の
    /// グローバル副作用を避けるため `with_default` のスコープ内で完結させる。
    fn capture_warns<F: FnOnce()>(f: F) -> String {
        use std::io;
        use std::sync::{Arc, Mutex};

        #[derive(Clone, Default)]
        struct LogCapture(Arc<Mutex<Vec<u8>>>);

        impl LogCapture {
            fn into_string(self) -> String {
                String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
            }
        }

        impl io::Write for LogCapture {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogCapture {
            type Writer = LogCapture;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let capture = LogCapture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(capture.clone())
            .with_max_level(tracing::Level::WARN)
            .with_target(false)
            .without_time()
            .finish();
        tracing::subscriber::with_default(subscriber, f);
        capture.into_string()
    }

    /// `walk_dir` が `read_dir` で失敗したときに silent skip せず、`tracing::warn!` を
    /// 発火することを保証する (Wave 1D G20)。
    ///
    /// フィクスチャ: 実体は regular file のパスを `walk_dir` に渡す。canonicalize は
    /// 成功するが `read_dir` はディレクトリではないため Err を返す。これにより
    /// ファイルシステム状態 (権限剥奪 / TOCTOU) に依存せず確実に read_dir 失敗経路を
    /// 踏める。canonicalize 失敗経路は G20 のスコープ外 (別タスク) で、本テストは触らない。
    ///
    /// PII 検証: 出力ログには raw absolute path の代わりに `logging::redact_path` が
    /// 適用されるべき。tempdir はユーザーホーム配下にあるため `~/...` 形式に短縮され、
    /// ユーザー名 (ホームの file_name) は含まれない。
    #[test]
    fn walk_dir_warns_and_skips_when_read_dir_fails() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // canonicalize が成功する regular file を渡す。read_dir は Err で返る。
        let fake_dir = tmp.path().join("not_a_directory");
        std::fs::write(&fake_dir, b"not a directory").expect("write file");

        let redacted = crate::logging::redact_path(&fake_dir);

        let logs = capture_warns(|| {
            let mut out: Vec<String> = Vec::new();
            walk_dir(&fake_dir, false, &mut out);
            // read_dir が失敗しても panic せず out は空のまま返ること (skip 動作保持)。
            assert!(
                out.is_empty(),
                "read_dir 失敗時は out に何も積まないべき、実際: {:?}",
                out
            );
        });

        // G20: silent skip を WARN で明示する。silent failure 退行検知のための最低限の保証。
        assert!(logs.contains("WARN"), "WARN レベルで出力されるべき: {logs}");
        // canonicalize 失敗経路と区別するため、read_dir 由来の文脈語が含まれていること。
        assert!(
            logs.contains("read_dir") || logs.contains("ディレクトリ走査"),
            "read_dir 由来のコンテキストが含まれるべき: {logs}"
        );
        // PII: redact_path 適用後のパスが含まれるべき (ユーザー名の生出力を避ける)。
        assert!(
            logs.contains(&redacted),
            "redact 後のパスが含まれるべき (got redacted={redacted:?}): {logs}"
        );
        // PII: ホームディレクトリ由来の username が生で漏れていないこと。
        if let Some(home) = dirs::home_dir() {
            if let Some(username) = home.file_name().and_then(|n| n.to_str()) {
                if !username.is_empty() {
                    assert!(
                        !logs.contains(username) || redacted == format!("~/{}", username),
                        "username 生出力がログに漏れていないこと: username={username:?} logs={logs}"
                    );
                }
            }
        }
    }

    /// `walk_dir` がディレクトリ symlink ループを検出して終了できることを保証する。
    ///
    /// フィクスチャ: `tmp/cycle_dir/` の中に `loop` というディレクトリ symlink を張り、
    /// `loop` が `cycle_dir` 自身を指す形にする。`recursive: true` で走査すると、
    /// 修正前コードは無限に再帰 → スタックオーバーフローで panic。修正後は canonicalize
    /// + 訪問済み HashSet により同じ canonical path を 2 度訪れず有限時間で完走する。
    ///
    /// `recursive: false` 既定の挙動は本テストでは直接触らない (別テストで保証)。
    #[cfg(windows)]
    #[test]
    fn walk_dir_does_not_loop_on_directory_symlink_cycle() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cycle_dir = tmp.path().join("cycle_dir");
        std::fs::create_dir(&cycle_dir).expect("create cycle_dir");
        // 中身の実ファイル (PNG 1 個)。canonical 走査で循環しない限り 1 件見つかるはず。
        let one_pix = include_bytes!("../../tests/fixtures/1x1.png");
        std::fs::write(cycle_dir.join("inside.png"), one_pix).expect("write inside.png");
        // cycle_dir/loop -> cycle_dir という symlink を作る。これが canonical 上の cycle。
        let link_path = cycle_dir.join("loop");
        let created = try_make_dir_symlink(&link_path, &cycle_dir).expect("symlink_dir");
        if !created {
            eprintln!(
                "skipping: ディレクトリ symlink が作成できない環境 (Developer Mode / SeCreateSymbolicLinkPrivilege)。 Windows の Developer Mode を有効にしてから再実行すること。",
            );
            return;
        }

        // walk_dir は `collect_target_files` の private helper だが、ここでは
        // recursive=true で呼び出してループを踏む (RED 期待)。
        let mut out = Vec::new();
        walk_dir(&cycle_dir, true, &mut out);

        // canonicalize 後の cycle_dir と「loop」が同じ canonical path を持つので、
        // 訪問済みセットに投入されるべき。修正後は無限再帰せず完了する。
        // 1 件 (inside.png) のみが返る。ループ内の同名が再カウントされない。
        assert_eq!(
            out.len(),
            1,
            "symlink loop 配下の inside.png は 1 度だけ拾われるべき、実際: {:?}",
            out
        );
        assert!(
            out.iter().any(|p| p.ends_with("inside.png")),
            "inside.png が見つからない: {:?}",
            out
        );
    }
}
