//! クリエイターモードの一括インポート機能。
//!
//! - `bulk_resolve_assets`: 複数ファイル / フォルダから対応形式を読み取り、
//!   PNG bytes / SVG テキスト / メタデータを正規化して返す。
//! - キャンセル可能 (job_id 単位)。
//! - SVG sanitize は責務分離のため JS 側で実施 (ここでは生テキストを返す)。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
