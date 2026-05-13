//! クリエイターモードの一括インポート機能。
//!
//! - `bulk_resolve_assets`: 複数ファイル / フォルダから対応形式を読み取り、
//!   PNG bytes / SVG テキスト / メタデータを正規化して返す。
//! - キャンセル可能 (job_id 単位)。
//! - SVG sanitize は責務分離のため JS 側で実施 (ここでは生テキストを返す)。
//!
//! 構成:
//!
//! | サブモジュール | 役割 |
//! |---|---|
//! | [`assets`] | ファイル/フォルダ走査 + 単一ファイル → `ResolvedAsset` 変換 + `bulk_resolve_assets` IPC |
//! | [`cursorpack`] | `.cursorpack` を Creator 用に解凍して各ロールの PNG / sized 情報を取り出す |
//!
//! 共有型 (DTO + `CancelRegistry`) はこのファイルに集約し、サブモジュールから `pub use` する。

pub mod assets;
pub mod cursorpack;

pub use assets::{bulk_resolve_assets, cancel_bulk_import};
pub use cursorpack::parse_cursorpack_for_creator;

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
    Ani,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAsset {
    pub source_file: String,
    pub source_path: String,
    pub kind: AssetKind,
    #[serde(flatten)]
    pub asset: crate::theme::types::CursorAssetDescriptor,
    pub svg_text: Option<String>,
    pub available_sizes: Vec<u32>,
    pub ani: Option<crate::theme::types::AniFrameData>,
}

// AniAssetData は theme::types::AniFrameData に統合済み (Phase 3a)。
// 旧名で参照されないよう、ここでは re-export しない。
// 利用側は `use crate::theme::types::AniFrameData;` に切り替えること。

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseCursorpackRequest {
    pub path: String,
    pub job_id: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CursorpackMetadata {
    /// `theme.json` の id (UUID)。`?editPath` 経由で Creator に流したとき、
    /// SaveDestinationModal の「上書き保存 / 複製」セクションを出し分けるトリガに使う。
    /// 文字列で返してフロント側の `sourceThemeId` ref にそのまま渡せる形に揃える。
    pub id: Option<String>,
    pub name_ja: Option<String>,
    pub name_en: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRole {
    #[serde(flatten)]
    pub asset: crate::theme::types::CursorAssetDescriptor,
    pub sized_png_bytes: HashMap<u32, Vec<u8>>,
    /// `.ani` ロールのフレームデータ。`.cur` / `.ico` の場合は `None`。
    /// フロントエンドはこれがあれば「動くサムネ」を出し、`aniFrames` として
    /// RoleAsset に格納する (= ResolvedAsset の `ani` と同じ役割)。
    pub ani: Option<crate::theme::types::AniFrameData>,
    /// `.ani` ロールの元バイトを展開した絶対パス。export 時に `rewrite_ani_with_hotspot`
    /// のソースとして使う。`.cur` / `.ico` ロールでは `None`。
    /// `.cursorpack` の隣 `<cursorpack>.extracted/<role-filename>` に書き出される。
    pub ani_source_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedCursorpack {
    pub metadata: CursorpackMetadata,
    pub roles: HashMap<String, ParsedRole>,
}

#[cfg(test)]
mod ipc_shape_tests {
    use super::*;
    use crate::theme::types::{AniFrameData, CursorAssetDescriptor, Hotspot};

    #[test]
    fn resolved_asset_serializes_flat() {
        let r = ResolvedAsset {
            source_file: "a.png".into(),
            source_path: "/a.png".into(),
            kind: AssetKind::Png,
            asset: CursorAssetDescriptor {
                png_bytes: vec![0],
                width: 64,
                height: 64,
                hotspot: Hotspot::ZERO,
            },
            svg_text: None,
            available_sizes: vec![64],
            ani: None,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert!(v.get("asset").is_none(), "asset must be flattened");
        assert!(v.get("pngBytes").is_some());
        assert_eq!(v["width"], 64);
        assert_eq!(v["height"], 64);
        assert_eq!(v["sourceFile"], "a.png");
    }

    #[test]
    fn parsed_role_serializes_flat() {
        let p = ParsedRole {
            asset: CursorAssetDescriptor {
                png_bytes: vec![0],
                width: 32,
                height: 32,
                hotspot: Hotspot::ZERO,
            },
            sized_png_bytes: HashMap::new(),
            ani: Some(AniFrameData {
                frame_pngs: vec![vec![1]],
                sequence: vec![0],
                per_step_durations_ms: vec![100],
                is_legacy_raw_dib: false,
            }),
            ani_source_path: None,
        };
        let v = serde_json::to_value(&p).unwrap();
        assert!(v.get("asset").is_none());
        assert!(v.get("pngBytes").is_some());
        assert_eq!(v["width"], 32);
        // ani は flatten せず Option として残るので key として存在
        assert!(v.get("ani").is_some());
        assert!(v["ani"]["framePngs"].is_array());
    }
}
