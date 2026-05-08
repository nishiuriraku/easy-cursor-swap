//! テーマ関連の型定義 (theme.json スキーマ + ライブラリ列挙用 DTO)。
//!
//! `serde` 派生で IPC 経由 JSON にもそのまま流せる。`LocalizedString` のみ
//! `untagged` enum で「単純文字列 / ロケールマップ」を吸収する。

use crate::errors::AppResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// テーマメタデータ (theme.json のスキーマ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetadata {
    /// スキーマバージョン
    pub schema_version: u32,
    /// テーマ固有ID (UUID)
    pub id: Uuid,
    /// テーマ名 (多言語対応)
    pub name: LocalizedString,
    /// テーマバージョン (SemVer)
    pub version: String,
    /// 作成日時 (ISO8601)
    pub created_at: String,
    /// OS標準の影を必要とするか
    pub requires_os_shadow: bool,
    /// カーソル定義マップ（役割 → カーソル定義）
    pub cursors: HashMap<String, CursorDefinition>,

    // --- 推奨フィールド ---
    /// 作者名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// ライセンス (SPDX識別子)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// ホームページURL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// テーマ説明 (多言語対応)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<LocalizedString>,
    /// 最低動作アプリバージョン
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_app_version: Option<String>,
    /// 署名 (将来の検証用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// テーマタグ (ライブラリ一覧の chip 表示・分類用。例: "animated", "dark", "minimal")
    /// 旧スキーマとの互換のため `serde(default)` で空配列にフォールバック。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// 多言語対応文字列
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocalizedString {
    /// 単純な文字列
    Simple(String),
    /// 多言語マップ
    Localized(HashMap<String, String>),
}

impl LocalizedString {
    /// 指定ロケールに合った文字列を返す
    /// フォールバック: 指定ロケール → "default" → "en" → 最初の値
    pub fn get(&self, locale: &str) -> String {
        match self {
            LocalizedString::Simple(s) => s.clone(),
            LocalizedString::Localized(map) => {
                // まず指定ロケールをチェック
                if let Some(val) = map.get(locale) {
                    return val.clone();
                }
                // "default" キーをチェック
                if let Some(val) = map.get("default") {
                    return val.clone();
                }
                // "en" フォールバック
                if let Some(val) = map.get("en") {
                    return val.clone();
                }
                // どれもなければ最初の値
                map.values().next().cloned().unwrap_or_default()
            }
        }
    }
}

/// 個別カーソルの定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorDefinition {
    /// カーソルファイルのパス（.cursorpack 内の相対パス）
    pub file: String,
    /// ホットスポット X座標（元画像のピクセル値）
    pub hotspot_x: u32,
    /// ホットスポット Y座標（元画像のピクセル値）
    pub hotspot_y: u32,
    /// リサイズアルゴリズム ("lanczos" / "nearest")
    #[serde(default = "default_resize_method")]
    pub resize_method: String,
    /// 解像度別の画像オーバーライド
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_overrides: Option<HashMap<String, SizeOverride>>,
}

/// 解像度別オーバーライド
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeOverride {
    /// このサイズ専用の画像ファイルパス
    pub file: String,
    /// このサイズ専用のホットスポット X（未指定時は基準サイズから比例計算）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotspot_x: Option<u32>,
    /// このサイズ専用のホットスポット Y
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotspot_y: Option<u32>,
}

fn default_resize_method() -> String {
    "lanczos".to_string()
}

/// `.cursorpack` をインポートする前の軽量検査結果。
/// theme.json のみ読み出してメタ情報を返し、既存ライブラリとの衝突を報告する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorpackInspection {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub role_count: u32,
    /// 既存ライブラリに同 ID のテーマがあれば情報を埋める
    pub existing: Option<ExistingTheme>,
}

/// 既存ライブラリ内テーマの参照情報 (バージョン比較用)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistingTheme {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub role_count: u32,
}

/// テーマライブラリ内の1テーマを表すサマリー情報
/// UIに表示するための軽量データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSummary {
    /// テーマID
    pub id: Uuid,
    /// テーマ名
    pub name: String,
    /// 作者名
    pub author: Option<String>,
    /// テーマバージョン
    pub version: String,
    /// 作成日時
    pub created_at: String,
    /// 適用中かどうか
    pub is_active: bool,
    /// お気に入りかどうか
    pub is_favorite: bool,
    /// 適用回数
    pub apply_count: u32,
    /// 含まれるカーソル役割の一覧
    pub included_roles: Vec<String>,
    /// テーマディレクトリのパス
    pub path: String,
    /// テーマタグ (theme.json の `tags` フィールドをそのまま転送)
    pub tags: Vec<String>,
    /// テーマディレクトリ全体の合計サイズ (bytes)。一覧表示で「2.1 MB」のように出す用途。
    pub size_bytes: u64,
    /// 署名済みか (`metadata.signature` が存在するかどうかのみで判定)。
    /// 一覧の「署名」列で Ed25519 / 未署名 のピル色分けに使う。
    /// **検証結果ではない** — 検証は marketplace::verify_signature が別途行う。
    pub signed: bool,
}

/// `LocalizedString` の文字列値全てに同じサフィックスを付与した新しい LocalizedString を返す。
///
/// serde_json::Value 経由で実装することで、LocalizedString が将来増減する
/// ロケールに自動追従する。
pub(super) fn clone_with_suffix(src: &LocalizedString, suffix: &str) -> LocalizedString {
    let mut value: serde_json::Value = match serde_json::to_value(src) {
        Ok(v) => v,
        Err(_) => return src.clone(),
    };
    if let Some(map) = value.as_object_mut() {
        for (_, v) in map.iter_mut() {
            if let Some(s) = v.as_str() {
                *v = serde_json::Value::String(format!("{}{}", s, suffix));
            }
        }
    }
    serde_json::from_value(value).unwrap_or_else(|_| src.clone())
}

/// `from` 配下を `to` に再帰コピーする。シンボリックリンクは追わず通常ファイルとして扱う。
pub(super) fn copy_dir_recursive(from: &std::path::Path, to: &std::path::Path) -> AppResult<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            copy_dir_recursive(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

/// `dir` 配下を `root` からの相対パスで Zip に書き込む。
pub(super) fn zip_dir_recursive<W: std::io::Write + std::io::Seek>(
    dir: &std::path::Path,
    root: &std::path::Path,
    zip: &mut zip::ZipWriter<W>,
    opts: zip::write::SimpleFileOptions,
) -> AppResult<()> {
    use std::io::Write;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if entry.metadata()?.is_dir() {
            zip_dir_recursive(&path, root, zip, opts)?;
        } else {
            let bytes = std::fs::read(&path)?;
            zip.start_file(rel_str, opts)?;
            zip.write_all(&bytes)?;
        }
    }
    Ok(())
}
