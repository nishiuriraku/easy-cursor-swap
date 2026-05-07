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
    let Ok(rd) = std::fs::read_dir(dir) else { return };
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
    let bytes = std::fs::read(p)
        .map_err(|e| AppError::ImageProcessing(format!("読み込み失敗: {}", e)))?;
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
        return Err(AppError::ImageProcessing(
            "PNG マジックバイト不一致".into(),
        ));
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

    #[test]
    fn collect_files_non_recursive_finds_pngs() {
        let dir = fixture_dir();
        let files = collect_target_files(&[dir.to_string_lossy().to_string()], false);
        let pngs: Vec<_> = files.iter().filter(|p| p.ends_with(".png")).collect();
        assert!(pngs.len() >= 17, "expected >=17 PNGs in sample-icon, got {}", pngs.len());
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
}
