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
}
