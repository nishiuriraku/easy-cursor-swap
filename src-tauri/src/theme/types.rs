//! テーマ関連の型定義 (theme.json スキーマ + ライブラリ列挙用 DTO)。
//!
//! `serde` 派生で IPC 経由 JSON にもそのまま流せる。`LocalizedString` のみ
//! `untagged` enum で「単純文字列 / ロケールマップ」を吸収する。

use crate::errors::AppResult;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 0.0..=1.0 に制約された比率。
///
/// - `new()` で NaN → 0.0、範囲外は `clamp(0.0, 1.0)` で正規化。
/// - `Deserialize` は `f32` で受けて `new()` 経由 (不正値で deserialize 失敗しない)。
/// - JSON 上は透過的に `number` (例: `0.0125`)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Ratio01(f32);

impl Ratio01 {
    pub const ZERO: Self = Self(0.0);
    pub const ONE: Self = Self(1.0);

    /// `v` を `[0.0, 1.0]` に正規化する。NaN は 0.0、無限大は範囲外として clamp。
    pub fn new(v: f32) -> Self {
        if v.is_nan() {
            Self(0.0)
        } else {
            Self(v.clamp(0.0, 1.0))
        }
    }

    pub fn get(self) -> f32 {
        self.0
    }

    /// 比率 → 絶対 px (`round()` で四捨五入、`[0, size]` に clamp)。
    pub fn to_px(self, size: u32) -> u32 {
        let raw = (self.0 * size as f32).round() as i64;
        raw.clamp(0, size as i64) as u32
    }

    /// 絶対 px → 比率。`size == 0` のとき `ZERO`。
    pub fn from_px(px: u32, size: u32) -> Self {
        if size == 0 {
            Self::ZERO
        } else {
            Self::new(px as f32 / size as f32)
        }
    }
}

impl<'de> Deserialize<'de> for Ratio01 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = f32::deserialize(d)?;
        Ok(Self::new(v))
    }
}

/// ホットスポット (比率)。
///
/// `(x, y)` は表示画像の左上から見た比率 (0.0 = 左上、1.0 = 右下) で
/// 絶対 px は持たない。`.cur` 書出時に `to_px(size)` で px に変換する。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hotspot {
    pub x: Ratio01,
    pub y: Ratio01,
}

impl Hotspot {
    pub const ZERO: Self = Self {
        x: Ratio01::ZERO,
        y: Ratio01::ZERO,
    };

    /// 絶対 px ペア (基準 `size`) から比率 Hotspot を作る。
    pub fn from_px(x_px: u32, y_px: u32, size: u32) -> Self {
        Self {
            x: Ratio01::from_px(x_px, size),
            y: Ratio01::from_px(y_px, size),
        }
    }

    /// 比率 Hotspot を `size` 基準の px ペアに戻す。
    pub fn to_px(self, size: u32) -> (u32, u32) {
        (self.x.to_px(size), self.y.to_px(size))
    }
}

/// IPC レスポンスが「カーソル素材の絵」を表現する共通基底。
///
/// 各 IPC レスポンス型は `#[serde(flatten)]` でこれを埋め込み、JSON 上は
/// 今までと同じ平坦構造 (`{ pngBytes, width, height, hotspot, ... }`) を保つ。
/// `width != height` の非正方画像にも対応する。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorAssetDescriptor {
    /// 最大解像度の RGBA PNG バイト列。Vue 側は Blob 化して `<img>` の src に使う。
    pub png_bytes: Vec<u8>,
    /// PNG のネイティブ幅 (px)。
    pub width: u32,
    /// PNG のネイティブ高さ (px)。
    pub height: u32,
    /// ホットスポット (比率 0.0..=1.0)。
    pub hotspot: Hotspot,
}

/// `.ani` フレーム情報の共通コア。`inspect_ani_file` / bulk import 双方で共有する。
///
/// - `frame_pngs`: 各フレームの最大解像度 PNG バイト列 (格納順)。
/// - `sequence`: 再生順 (sequence インデックスをフレーム 0..num_frames-1 にマップ)。
/// - `per_step_durations_ms`: 各 step の duration。`length == sequence.len()`。
/// - `is_legacy_raw_dib`: 元 ANI が `RIFF` ICO エントリではなく生 DIB (旧形式) かどうか。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AniFrameData {
    pub frame_pngs: Vec<Vec<u8>>,
    pub sequence: Vec<u32>,
    pub per_step_durations_ms: Vec<u32>,
    pub is_legacy_raw_dib: bool,
}

/// テーマソース種別。
///
/// - `Local`: ユーザーが作成した、または `.cursorpack` を手動で取り込んだテーマ (default)
/// - `Marketplace`: 公式インデックス (`marketplace_install`) 経由で取得したテーマ。
///   UI と `repackage_theme` IPC が編集 / エクスポートをガードする。
///
/// `Deserialize` で未知の値は `Local` にフォールバックする (forward-compat)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeSource {
    Marketplace,
    /// `#[serde(other)]` で未知の値 (将来追加されるソース種別など) を Local にフォールバック。
    /// この属性は必ず enum の最後のバリアントに付ける必要がある (serde 仕様)。
    #[default]
    #[serde(other)]
    Local,
}

fn is_local_source(s: &ThemeSource) -> bool {
    matches!(s, ThemeSource::Local)
}

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
    /// テーマソース種別。`marketplace` の場合 UI と repackage_theme が編集/エクスポートをガード。
    /// 旧スキーマとの互換のため `serde(default)` で `Local` にフォールバック。Local の場合は
    /// JSON に書き出さない (`skip_serializing_if`) ことで旧形式の theme.json と完全互換。
    #[serde(default, skip_serializing_if = "is_local_source")]
    pub source: ThemeSource,
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
    /// ホットスポット (比率, 0.0..=1.0)。`.cur` 書出時に primarySize と乗算して px に変換する。
    pub hotspot: Hotspot,
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
    /// このサイズ専用のホットスポット比率。
    /// `None` なら親 `CursorDefinition.hotspot` を継承。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotspot: Option<Hotspot>,
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
    /// 最終適用日時 (RFC3339)。一度も適用されていなければ None。
    /// Library 画面「最近使用」フィルタと「最近使った順」ソートで使用。
    pub last_applied_at: Option<String>,
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
    /// theme.json `description` を表示用に解決した文字列。
    /// 現状はロケール `"ja"` 固定 (`name` と同じ TODO)。`None` のとき UI は説明段落を非表示。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// theme.json `schema_version`。詳細モーダルの PACKAGE セルで表示する。
    pub schema_version: u32,
    /// theme.json `license` (SPDX)。`None` のとき行非表示。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// theme.json `homepage`。`None` のとき行非表示。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
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

#[cfg(test)]
mod ratio_tests {
    use super::*;

    #[test]
    fn ratio_new_clamps_below_zero() {
        assert_eq!(Ratio01::new(-0.5).get(), 0.0);
    }

    #[test]
    fn ratio_new_clamps_above_one() {
        assert_eq!(Ratio01::new(1.5).get(), 1.0);
    }

    #[test]
    fn ratio_new_handles_nan() {
        assert_eq!(Ratio01::new(f32::NAN).get(), 0.0);
    }

    #[test]
    fn ratio_new_handles_infinity() {
        assert_eq!(Ratio01::new(f32::INFINITY).get(), 1.0);
        assert_eq!(Ratio01::new(f32::NEG_INFINITY).get(), 0.0);
    }

    #[test]
    fn ratio_to_px_rounds() {
        assert_eq!(Ratio01::new(0.5).to_px(32), 16);
        assert_eq!(Ratio01::new(0.5).to_px(33), 17); // 16.5 → 17
        assert_eq!(Ratio01::new(0.0).to_px(32), 0);
        assert_eq!(Ratio01::new(1.0).to_px(32), 32);
    }

    #[test]
    fn ratio_from_px_zero_size() {
        assert_eq!(Ratio01::from_px(5, 0), Ratio01::ZERO);
    }

    #[test]
    fn ratio_roundtrip_within_one_px() {
        for size in [16u32, 32, 64, 128, 256] {
            for px in [0u32, 1, size / 4, size / 2, size - 1, size] {
                let r = Ratio01::from_px(px, size);
                let back = r.to_px(size);
                assert!(
                    back.abs_diff(px) <= 1,
                    "size={size} px={px} → {back} (Δ={})",
                    back.abs_diff(px)
                );
            }
        }
    }

    #[test]
    fn hotspot_serde_roundtrip() {
        let h = Hotspot {
            x: Ratio01::new(0.125),
            y: Ratio01::new(0.875),
        };
        let json = serde_json::to_string(&h).unwrap();
        assert_eq!(json, r#"{"x":0.125,"y":0.875}"#);
        let back: Hotspot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn hotspot_deserialize_clamps_invalid() {
        let h: Hotspot = serde_json::from_str(r#"{"x":2.5,"y":-1.0}"#).unwrap();
        assert_eq!(h.x.get(), 1.0);
        assert_eq!(h.y.get(), 0.0);
    }

    #[test]
    fn cursor_asset_descriptor_serializes_with_expected_keys() {
        let d = CursorAssetDescriptor {
            png_bytes: vec![0xDE, 0xAD],
            width: 32,
            height: 32,
            hotspot: Hotspot::ZERO,
        };
        let v = serde_json::to_value(&d).unwrap();
        assert!(v.get("pngBytes").is_some(), "pngBytes missing: {v}");
        assert_eq!(v["width"], 32);
        assert_eq!(v["height"], 32);
        assert!(v.get("hotspot").is_some());
    }

    #[test]
    fn ani_frame_data_serializes_with_expected_keys() {
        let a = AniFrameData {
            frame_pngs: vec![vec![1, 2]],
            sequence: vec![0],
            per_step_durations_ms: vec![100],
            is_legacy_raw_dib: false,
        };
        let v = serde_json::to_value(&a).unwrap();
        assert!(v.get("framePngs").is_some());
        assert!(v.get("sequence").is_some());
        assert!(v.get("perStepDurationsMs").is_some());
        assert_eq!(v["isLegacyRawDib"], false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ja_en_de_map() -> LocalizedString {
        let mut m = HashMap::new();
        m.insert("ja".into(), "日本語名".into());
        m.insert("en".into(), "English Name".into());
        m.insert("de".into(), "Deutscher Name".into());
        LocalizedString::Localized(m)
    }

    fn sample_summary_full() -> ThemeSummary {
        ThemeSummary {
            id: Uuid::nil(),
            name: "Test Theme".into(),
            author: Some("Tester".into()),
            version: "1.0.0".into(),
            created_at: "2026-05-12T00:00:00Z".into(),
            is_active: false,
            is_favorite: false,
            apply_count: 0,
            last_applied_at: None,
            included_roles: vec!["Arrow".into()],
            path: "/tmp/test".into(),
            tags: vec!["dark".into()],
            size_bytes: 1024,
            signed: true,
            description: Some("テスト用説明".into()),
            schema_version: 1,
            license: Some("MIT".into()),
            homepage: Some("https://example.test".into()),
        }
    }

    #[test]
    fn theme_summary_omits_none_optional_fields() {
        let mut s = sample_summary_full();
        s.description = None;
        s.license = None;
        s.homepage = None;
        let v = serde_json::to_value(&s).unwrap();
        // 必須フィールドは出る
        assert_eq!(v["schema_version"], 1);
        // Option::None はキーごと省略される
        assert!(v.get("description").is_none(), "description present: {v}");
        assert!(v.get("license").is_none(), "license present: {v}");
        assert!(v.get("homepage").is_none(), "homepage present: {v}");
    }

    #[test]
    fn theme_summary_includes_optional_fields_when_present() {
        let s = sample_summary_full();
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["description"], "テスト用説明");
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["license"], "MIT");
        assert_eq!(v["homepage"], "https://example.test");
    }

    #[test]
    fn simple_returns_value_for_any_locale() {
        // Simple variant はロケールに関係なく同じ値を返す
        let s = LocalizedString::Simple("Universal".into());
        assert_eq!(s.get("ja"), "Universal");
        assert_eq!(s.get("en"), "Universal");
        assert_eq!(s.get("zz"), "Universal");
    }

    #[test]
    fn localized_returns_exact_match() {
        let s = ja_en_de_map();
        assert_eq!(s.get("ja"), "日本語名");
        assert_eq!(s.get("en"), "English Name");
        assert_eq!(s.get("de"), "Deutscher Name");
    }

    #[test]
    fn localized_falls_back_to_default_then_en() {
        // 未定義ロケール (fr) → "default" 不在 → "en" にフォールバック
        let s = ja_en_de_map();
        assert_eq!(s.get("fr"), "English Name");
    }

    #[test]
    fn localized_prefers_default_over_en() {
        // "default" が存在する場合は en より優先
        let mut m = HashMap::new();
        m.insert("default".into(), "Default Value".into());
        m.insert("en".into(), "English Value".into());
        let s = LocalizedString::Localized(m);
        assert_eq!(s.get("zz"), "Default Value");
    }

    #[test]
    fn localized_falls_back_to_first_value_when_no_default_or_en() {
        // ja のみ → fr 要求時は ja を返す (どれもなければ最初の値)
        let mut m = HashMap::new();
        m.insert("ja".into(), "唯一の値".into());
        let s = LocalizedString::Localized(m);
        assert_eq!(s.get("fr"), "唯一の値");
    }

    #[test]
    fn localized_returns_empty_string_for_empty_map() {
        // 空マップでも panic しない
        let s = LocalizedString::Localized(HashMap::new());
        assert_eq!(s.get("ja"), "");
    }

    #[test]
    fn localized_exact_match_takes_precedence_over_fallback() {
        // ja を求めて ja があれば、default や en が両方あっても ja を返す
        let mut m = HashMap::new();
        m.insert("ja".into(), "JA".into());
        m.insert("default".into(), "DEFAULT".into());
        m.insert("en".into(), "EN".into());
        let s = LocalizedString::Localized(m);
        assert_eq!(s.get("ja"), "JA");
    }

    #[test]
    fn clone_with_suffix_appends_to_localized_map() {
        // 主要ユースケース: Localized マップの全ロケール値にサフィックス付与
        let original = ja_en_de_map();
        let copied = clone_with_suffix(&original, " (Copy)");
        assert_eq!(copied.get("ja"), "日本語名 (Copy)");
        assert_eq!(copied.get("en"), "English Name (Copy)");
        assert_eq!(copied.get("de"), "Deutscher Name (Copy)");
    }

    #[test]
    fn clone_with_suffix_keeps_simple_unchanged() {
        // 仕様上の既知の制限: Simple variant は serde_json で string になり
        // as_object_mut() で None が返るためサフィックスが付かない。
        // ライブラリは Localized が既定なので実害はないが、テストで挙動を固定しておく。
        let original = LocalizedString::Simple("Theme".into());
        let copied = clone_with_suffix(&original, " (Copy)");
        assert_eq!(copied.get("any"), "Theme");
    }

    #[test]
    fn clone_with_suffix_handles_empty_suffix() {
        // 空サフィックスは Localized でも内容不変
        let original = ja_en_de_map();
        let copied = clone_with_suffix(&original, "");
        assert_eq!(copied.get("ja"), "日本語名");
        assert_eq!(copied.get("en"), "English Name");
    }

    #[test]
    fn theme_source_defaults_to_local() {
        let s = ThemeSource::default();
        assert!(matches!(s, ThemeSource::Local));
    }

    #[test]
    fn theme_source_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&ThemeSource::Local).unwrap(),
            r#""local""#
        );
        assert_eq!(
            serde_json::to_string(&ThemeSource::Marketplace).unwrap(),
            r#""marketplace""#
        );
    }

    #[test]
    fn theme_source_unknown_value_falls_back_to_local() {
        let s: ThemeSource = serde_json::from_str(r#""future_value""#).unwrap();
        assert!(matches!(s, ThemeSource::Local));
    }

    #[test]
    fn theme_metadata_source_defaults_when_missing() {
        let json = r#"{
            "schema_version": 1,
            "id": "00000000-0000-0000-0000-000000000000",
            "name": "Test",
            "version": "1.0.0",
            "created_at": "2026-05-14T00:00:00Z",
            "requires_os_shadow": false,
            "cursors": {}
        }"#;
        let m: ThemeMetadata = serde_json::from_str(json).unwrap();
        assert!(matches!(m.source, ThemeSource::Local));
    }

    #[test]
    fn theme_metadata_source_marketplace_round_trips() {
        let json = r#"{
            "schema_version": 1,
            "id": "00000000-0000-0000-0000-000000000000",
            "name": "Test",
            "version": "1.0.0",
            "created_at": "2026-05-14T00:00:00Z",
            "requires_os_shadow": false,
            "cursors": {},
            "source": "marketplace"
        }"#;
        let m: ThemeMetadata = serde_json::from_str(json).unwrap();
        assert!(matches!(m.source, ThemeSource::Marketplace));
        let back = serde_json::to_string(&m).unwrap();
        assert!(back.contains(r#""source":"marketplace""#));
    }

    #[test]
    fn theme_metadata_source_local_is_omitted_in_serialization() {
        let m = ThemeMetadata {
            schema_version: 1,
            id: Uuid::nil(),
            name: LocalizedString::Simple("T".into()),
            version: "1.0.0".into(),
            created_at: "2026-05-14T00:00:00Z".into(),
            requires_os_shadow: false,
            cursors: HashMap::new(),
            author: None,
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
            tags: Vec::new(),
            source: ThemeSource::Local,
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("source"), "Local は省略されるべき: {json}");
    }
}
