//! クリエイターモードの一括インポート機能。
//!
//! - `bulk_resolve_assets`: 複数ファイル / フォルダから対応形式を読み取り、
//!   PNG bytes / SVG テキスト / メタデータを正規化して返す。
//! - キャンセル可能 (job_id 単位)。
//! - SVG sanitize は責務分離のため JS 側で実施 (ここでは生テキストを返す)。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

/// 一括解決対象の最大ファイルサイズ。これを超えるファイルは failures 行きにする。
pub const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// バッチ全体の合計サイズ上限 (メモリ保護)。
pub const MAX_TOTAL_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkResolveRequest {
    pub paths: Vec<String>,
    #[serde(default)]
    pub recursive: bool,
    pub job_id: String,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Png,
    Svg,
    Cur,
    Ico,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAsset {
    pub source_file: String,
    pub source_path: String,
    pub kind: AssetKind,
    pub png_bytes: Vec<u8>,
    pub svg_text: Option<String>,
    pub native_size: u32,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    pub available_sizes: Vec<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveFailure {
    pub source_path: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkResolveResult {
    pub assets: Vec<ResolvedAsset>,
    pub failures: Vec<ResolveFailure>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkImportProgress {
    pub job_id: String,
    pub stage: &'static str,
    pub current: u32,
    pub total: u32,
    pub message: Option<String>,
}

/// 進行中の job_id 集合。キャンセル時は false に下げる。
#[derive(Default)]
pub struct CancelRegistry {
    inner: Mutex<HashMap<String, bool>>,
}

impl CancelRegistry {
    pub fn register(&self, job_id: &str) {
        self.inner.lock().unwrap().insert(job_id.to_string(), true);
    }
    pub fn cancel(&self, job_id: &str) {
        if let Some(v) = self.inner.lock().unwrap().get_mut(job_id) {
            *v = false;
        }
    }
    /// 指定 job_id が登録済みかつキャンセルされていなければ true。未登録の job_id は false を返す（不明 = 非アクティブ扱い）。
    /// ワーカーが poll する前に必ず `register` を先行させること。
    pub fn is_active(&self, job_id: &str) -> bool {
        *self.inner.lock().unwrap().get(job_id).unwrap_or(&false)
    }
    pub fn drop_job(&self, job_id: &str) {
        self.inner.lock().unwrap().remove(job_id);
    }
}

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

use crate::errors::AppError;

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

use tauri::{AppHandle, Emitter, Manager, State};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseCursorpackRequest {
    pub path: String,
    pub job_id: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CursorpackMetadata {
    pub name_ja: Option<String>,
    pub name_en: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRole {
    pub primary_size: u32,
    pub primary_png_bytes: Vec<u8>,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    pub sized_png_bytes: HashMap<u32, Vec<u8>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedCursorpack {
    pub metadata: CursorpackMetadata,
    pub roles: HashMap<String, ParsedRole>,
}

use std::io::Read;
use zip::ZipArchive;

/// `.cursorpack` の theme.json から CursorpackMetadata を組み立てる。
fn metadata_from_theme(meta: &crate::theme::ThemeMetadata) -> CursorpackMetadata {
    use crate::theme::LocalizedString;
    let name_ja = match &meta.name {
        LocalizedString::Simple(s) => Some(s.clone()),
        LocalizedString::Localized(m) => m.get("ja").or_else(|| m.get("default")).cloned(),
    };
    let name_en = match &meta.name {
        LocalizedString::Simple(_) => None,
        LocalizedString::Localized(m) => m.get("en").cloned(),
    };
    let description = meta.description.as_ref().and_then(|d| match d {
        LocalizedString::Simple(s) => Some(s.clone()),
        LocalizedString::Localized(m) => m
            .get("ja")
            .or_else(|| m.get("en"))
            .or_else(|| m.get("default"))
            .cloned(),
    });
    CursorpackMetadata {
        name_ja,
        name_en,
        author: meta.author.clone(),
        version: Some(meta.version.clone()),
        description,
    }
}

/// `ParsedIcoCurEntry.image` を PNG バイト列にエンコード。
fn encode_entry_to_png(entry: &crate::cursor::ParsedIcoCurEntry) -> Result<Vec<u8>, AppError> {
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        entry.image.as_raw(),
        entry.image.width(),
        entry.image.height(),
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| AppError::ImageProcessing(format!("PNG エンコード失敗: {}", e)))?;
    Ok(buf)
}

pub fn parse_cursorpack_inner(bytes: &[u8]) -> Result<ParsedCursorpack, AppError> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| AppError::InvalidCursorpack {
        reason: format!("ZIP オープン失敗: {}", e),
    })?;

    // theme.json を読む
    let theme: crate::theme::ThemeMetadata = {
        let mut entry = archive
            .by_name("theme.json")
            .map_err(|_| AppError::InvalidCursorpack {
                reason: "theme.json が見つかりません".to_string(),
            })?;
        let mut buf = String::new();
        entry
            .read_to_string(&mut buf)
            .map_err(|e| AppError::InvalidCursorpack {
                reason: format!("theme.json 読み込み失敗: {}", e),
            })?;
        serde_json::from_str(&buf).map_err(|e| AppError::InvalidCursorpack {
            reason: format!("theme.json 解析失敗: {}", e),
        })?
    };

    let metadata = metadata_from_theme(&theme);

    // 各ロールを抽出
    let mut roles: HashMap<String, ParsedRole> = HashMap::new();
    for (role_id, def) in &theme.cursors {
        // primary ファイルを読む
        let primary_bytes = {
            let mut entry =
                archive
                    .by_name(&def.file)
                    .map_err(|_| AppError::InvalidCursorpack {
                        reason: format!(
                            "ロール {} のファイル {} が ZIP 内にありません",
                            role_id, def.file
                        ),
                    })?;
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| AppError::InvalidCursorpack {
                    reason: format!("ロール {} のファイル読み込み失敗: {}", role_id, e),
                })?;
            buf
        };

        let parsed = crate::cursor::parse_ico_cur(&primary_bytes)?;
        let (largest, primary_png) = crate::cursor::pick_largest_as_png(&parsed)?;

        // primary 内の各解像度を sized_png_bytes に詰める
        let mut sized: HashMap<u32, Vec<u8>> = HashMap::new();
        for entry in &parsed.entries {
            if let Ok(png) = encode_entry_to_png(entry) {
                sized.insert(entry.width, png);
            }
        }

        // size_overrides の各解像度も追加で読む
        if let Some(overrides) = &def.size_overrides {
            for (size_str, ov) in overrides {
                if let Ok(size) = size_str.parse::<u32>() {
                    let mut entry = match archive.by_name(&ov.file) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    let mut buf = Vec::new();
                    if entry.read_to_end(&mut buf).is_err() {
                        continue;
                    }
                    if let Ok(parsed_ov) = crate::cursor::parse_ico_cur(&buf) {
                        if let Some(matching) = parsed_ov.entries.iter().find(|e| e.width == size) {
                            if let Ok(png) = encode_entry_to_png(matching) {
                                sized.insert(size, png);
                            }
                        }
                    }
                }
            }
        }

        roles.insert(
            role_id.clone(),
            ParsedRole {
                primary_size: largest.width,
                primary_png_bytes: primary_png,
                hotspot_x: def.hotspot_x,
                hotspot_y: def.hotspot_y,
                sized_png_bytes: sized,
            },
        );
    }

    Ok(ParsedCursorpack { metadata, roles })
}

#[tauri::command]
pub async fn parse_cursorpack_for_creator(
    app: AppHandle,
    req: ParseCursorpackRequest,
) -> Result<ParsedCursorpack, AppError> {
    let job_id = req.job_id.clone();
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _ = app_clone.emit(
            "bulk-import-progress",
            BulkImportProgress {
                job_id: job_id.clone(),
                stage: "extract",
                current: 0,
                total: 1,
                message: None,
            },
        );
        let bytes = std::fs::read(&req.path).map_err(|e| AppError::InvalidCursorpack {
            reason: format!("読み込み失敗: {}", e),
        })?;
        let r = parse_cursorpack_inner(&bytes)?;
        let _ = app_clone.emit(
            "bulk-import-progress",
            BulkImportProgress {
                job_id,
                stage: "done",
                current: 1,
                total: 1,
                message: None,
            },
        );
        Ok(r)
    })
    .await
    .map_err(|e| AppError::InvalidCursorpack {
        reason: format!("join 失敗: {}", e),
    })?
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
    fn parse_cursorpack_basic_returns_roles() {
        let mut p = fixture_dir();
        p.push("easy-cursor-swap-mint.cursorpack");
        if !p.is_file() {
            eprintln!("skipping: cursorpack fixture not present");
            return;
        }
        let bytes = std::fs::read(&p).expect("fixture must exist");
        let parsed = parse_cursorpack_inner(&bytes).expect("parse should succeed");
        assert!(!parsed.roles.is_empty(), "should extract at least 1 role");
        assert!(
            parsed.roles.contains_key("Arrow"),
            "Arrow role should be present"
        );
    }

    #[test]
    fn bulk_resolve_with_oversize_collects_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("big.png");
        std::fs::write(&path, vec![0u8; (MAX_FILE_BYTES + 1) as usize]).unwrap();
        let small = tmp.path().join("ok.png");
        // 1x1 PNG (8byte signature + IHDR + IDAT + IEND の最小限)
        let one_pix = include_bytes!("../tests/fixtures/1x1.png");
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
