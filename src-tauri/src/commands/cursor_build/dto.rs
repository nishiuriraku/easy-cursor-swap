//! `cursor_build` の IPC 入出力 DTO 定義。
//!
//! `#[derive(Serialize/Deserialize)]` 派生のみのファイルで、ビルドロジックは持たない。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 出力先。`File` はディスクへの保存、`Library` はライブラリ展開 (+ オプションで apply)。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExportDestination {
    #[serde(rename_all = "camelCase")]
    File { path: String },
    #[serde(rename_all = "camelCase")]
    Library {
        #[serde(default)]
        apply_after: bool,
    },
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub theme_id: String,
    pub size_bytes: u64,
    pub signed: bool,
    pub key_id: Option<String>,
    /// `apply_after=true` で `apply_theme` まで成功した場合のみ true。
    pub applied: bool,
    /// `Library { apply_after: true }` で Library 登録は成功したが apply が失敗した場合のメッセージ。
    pub apply_error: Option<String>,
}

/// 1 役割分の入力 (PNG バイト列 + ホットスポット比率 + リサンプル指定)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleBuildEntry {
    pub role: String,
    pub png_bytes: Vec<u8>,
    /// ホットスポット (比率, 0.0..=1.0)。`.cur` 書出直前に `to_px(size)` で px 変換する。
    pub hotspot: crate::theme::types::Hotspot,
    /// "lanczos" / "nearest" (未知の値は ResizeMethod::from_str により Lanczos にフォールバック)
    pub resample: String,
    /// サイズ別オーバーライド (px → PNG bytes + optional 独立 hotspot)。
    /// Some の場合、対応サイズはリサンプルせずそのまま使用。
    /// None / 空なら従来どおり png_bytes をリサンプル。
    #[serde(default)]
    pub sized_overrides: Option<HashMap<u32, SizedOverridePayload>>,
    /// `.ani` 由来ロールのソースファイル絶対パス。
    /// セットされている場合、PNG → CUR ビルダではなく rewrite_ani_with_hotspot を経由して
    /// cursors/<role>.ani として書き出す。
    #[serde(default)]
    pub ani_source_path: Option<String>,
}

/// サイズ別オーバーライド (PNG + optional 独立 hotspot)。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SizedOverridePayload {
    pub png_bytes: Vec<u8>,
    #[serde(default)]
    pub hotspot: Option<crate::theme::types::Hotspot>,
}

/// ストリーム式 .cursorpack ビルドリクエスト
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamedExportRequest {
    /// フロント側が生成した一意 ID。`build-progress` イベントの相関キー兼キャンセル ID。
    pub build_id: String,
    pub name_ja: String,
    pub name_en: Option<String>,
    pub author: Option<String>,
    pub version: String,
    /// `theme.json` の `description` フィールド。`None` または空文字なら省略。
    /// 現状 UI は単一テキスト欄しか持たないので `LocalizedString::Simple` 相当の単一文字列で渡す。
    #[serde(default)]
    pub description: Option<String>,
    pub requires_os_shadow: bool,
    pub roles: Vec<RoleBuildEntry>,
    /// File / Library{apply_after} で出力先を切替える。
    pub destination: ExportDestination,
    /// `Some(uuid)` のとき新 UUID 発行ではなく既存テーマ ID を引き継ぐ (= 上書き保存)。
    pub existing_theme_id: Option<uuid::Uuid>,
    pub sign: bool,
}

/// 進捗イベントペイロード
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuildProgress {
    pub build_id: String,
    /// "role" / "package" / "sign" / "done" / "cancelled" / "error"
    pub stage: String,
    pub current: u32,
    pub total: u32,
    pub message: Option<String>,
}
