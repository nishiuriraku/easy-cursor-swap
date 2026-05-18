//! PNG プレビュー生成パス。
//!
//! - ユーザーテーマ (`load_role_previews` / `load_role_previews_with_hotspots`)
//! - Windows スキーム (`render_paths_as_previews` / `render_paths_as_previews_with_hotspots`)
//! - `.cursorpack` ビルド時の previews/ 同梱 (`build_preview_pngs`)
//! - 単体ファイルの PNG 化 (`render_cursor_file_as_png`)
//!
//! `theme/mod.rs` から分割 (2026-05-18, refactor/yellow-items)。

use super::types::{self, CursorDefinition, ThemeMetadata};
use super::ThemeManager;
use crate::errors::AppResult;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

/// 1 ロールあたりのプレビュー詳細。
///
/// `CursorAssetDescriptor` を `#[serde(flatten)]` で埋め込み、JSON 上は
/// `{ pngBytes, width, height, hotspot, frames? }` の平坦構造で返す。
/// (旧フィールド `png` は `pngBytes` にリネーム — Phase 3a での breaking change)
///
/// `frames` は `.ani` ロールのみ `Some` を持つ。テーマ詳細ドロワーがこのフィールドを
/// 見て、存在すれば `<CursorPreview kind="ani">` で全フレーム再生に切り替える。
/// 値は `inspect_ani_file` と同じ `AniFrameData` 形 (camelCase で
/// `{ framePngs, sequence, perStepDurationsMs, isLegacyRawDib }`)。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePreview {
    #[serde(flatten)]
    pub asset: types::CursorAssetDescriptor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frames: Option<types::AniFrameData>,
}

/// `parse_ani` の結果から「全フレームを PNG 化した `AniFrameData`」を構築する内部 helper。
///
/// `load_role_previews_with_hotspots` (ユーザーテーマ) と
/// `render_paths_as_previews_with_hotspots` (Windows スキーム) の両方で
/// 同じフレーム展開ロジックを共有するために抽出。先頭フレームの寸法 / ホットスポット
/// 算出は呼び出し側で行うのは、Windows スキームではホットスポット情報を 0 で返す
/// 既存挙動を維持したいため。
fn build_ani_frame_data(parsed: &crate::cursor::ParsedAni) -> AppResult<types::AniFrameData> {
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
        .map_err(|e| {
            crate::errors::AppError::ImageProcessing(format!("PNG エンコード失敗: {}", e))
        })?;
        frame_pngs.push(png);
    }
    let per_step_durations_ms: Vec<u32> = parsed
        .per_step_rate_jiffies
        .iter()
        .map(|j| ((*j as u64 * 1000) / 60) as u32)
        .collect();
    Ok(types::AniFrameData {
        frame_pngs,
        sequence: parsed.sequence.clone(),
        per_step_durations_ms,
        is_legacy_raw_dib: parsed.is_legacy_raw_dib,
    })
}

impl ThemeManager {
    /// CURSOR_ROLES 正規順の先頭 6 ロール固定（インデックスリポジトリと約束）。
    pub(crate) const PREVIEW_ROLES: [&'static str; 6] =
        ["Arrow", "Help", "AppStarting", "Wait", "Crosshair", "IBeam"];
    pub(crate) const PREVIEW_SIZE: u32 = 64;

    /// 指定テーマ ID のロール毎 PNG プレビューを返す。
    ///
    /// UI のテーマカード/ApplyModal で「実物の絵」を表示するために使う。
    /// `~/.custom_cursors/<UUID>/<role-file>` を読み、`.cur` / `.ico` は最大解像度を
    /// PNG 化し、`.png` 拡張子はそのままバイト列を返す。
    ///
    /// `roles_filter` が `Some` の場合は指定ロールのみ返す (カード用に Arrow だけ等)。
    pub fn load_role_previews(
        id: Uuid,
        roles_filter: Option<&[String]>,
    ) -> AppResult<HashMap<String, Vec<u8>>> {
        use crate::config::ConfigManager;

        let cursors_dir = ConfigManager::cursors_dir()?;
        let theme_dir = cursors_dir.join(id.to_string());
        let theme_json_path = theme_dir.join("theme.json");
        if !theme_json_path.is_file() {
            return Err(crate::errors::AppError::Theme(format!(
                "テーマ {} が見つかりません",
                id
            )));
        }
        let content = std::fs::read_to_string(&theme_json_path)?;
        let metadata: ThemeMetadata = serde_json::from_str(&content)?;

        let mut out: HashMap<String, Vec<u8>> = HashMap::new();
        for (role, def) in &metadata.cursors {
            if let Some(filter) = roles_filter {
                if !filter.iter().any(|r| r == role) {
                    continue;
                }
            }
            let abs = theme_dir.join(&def.file);
            if !abs.is_file() {
                continue;
            }
            // 拡張子別の分岐 (.png / .ani / .cur / .ico) は render_cursor_file_as_png に集約。
            // ここで個別に parse_ico_cur を呼んでしまうと .ani が "RIFF" 先頭バイトを
            // ICO の reserved (18770 = 0x4952) として解釈されて落ちる。
            match Self::render_cursor_file_as_png(&abs) {
                Ok(png) => {
                    out.insert(role.clone(), png);
                }
                Err(e) => {
                    tracing::warn!("{} の PNG 化に失敗: {}", role, e);
                }
            }
        }
        Ok(out)
    }

    /// [`load_role_previews`] のリッチ版。各ロールに `RolePreview` (PNG + 寸法 + ホットスポット) を返す。
    ///
    /// テーマ詳細ドロワーで「ホットスポットの位置」を視覚化する用途のみ使用する。
    /// PNG だけでよいプレビュー (テーマカードの 17 ロールサムネ等) は従来の
    /// [`load_role_previews`] を使い続けて IPC ペイロードの肥大化を避ける。
    pub fn load_role_previews_with_hotspots(
        id: Uuid,
        roles_filter: Option<&[String]>,
    ) -> AppResult<HashMap<String, RolePreview>> {
        use crate::config::ConfigManager;
        use crate::cursor::{parse_ani, parse_ico_cur, pick_largest_as_png};

        let cursors_dir = ConfigManager::cursors_dir()?;
        let theme_dir = cursors_dir.join(id.to_string());
        let theme_json_path = theme_dir.join("theme.json");
        if !theme_json_path.is_file() {
            return Err(crate::errors::AppError::Theme(format!(
                "テーマ {} が見つかりません",
                id
            )));
        }
        let content = std::fs::read_to_string(&theme_json_path)?;
        let metadata: ThemeMetadata = serde_json::from_str(&content)?;

        let mut out: HashMap<String, RolePreview> = HashMap::new();
        for (role, def) in &metadata.cursors {
            if let Some(filter) = roles_filter {
                if !filter.iter().any(|r| r == role) {
                    continue;
                }
            }
            let abs = theme_dir.join(&def.file);
            if !abs.is_file() {
                continue;
            }
            let bytes = match std::fs::read(&abs) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("プレビュー読込失敗 ({}): {}", role, e);
                    continue;
                }
            };
            let ext = abs
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            // PNG: ファイル寸法 = theme.json の hotspot 座標系。そのまま採用してよい。
            // .cur / .ico: ファイル内に複数解像度が入っており、`pick_largest_as_png` は
            //              通常 256x256 など primary より大きいエントリを返す。
            //              theme.json の hotspot は primary サイズの座標系で書かれて
            //              いるため、`entry.hotspot_x/y` (build 時に各エントリ向けに
            //              scale 済み) を使わないと寸法と座標系がずれてしまい、
            //              ホットスポットドットが画像の左上にずれて表示されるバグ
            //              の原因になる。エントリヘッダの値を採用するのが正しい。
            let preview = if ext == "png" {
                match image::load_from_memory_with_format(&bytes, image::ImageFormat::Png) {
                    Ok(img) => RolePreview {
                        asset: types::CursorAssetDescriptor {
                            png_bytes: bytes,
                            width: img.width(),
                            height: img.height(),
                            hotspot: def.hotspot,
                        },
                        frames: None,
                    },
                    Err(e) => {
                        tracing::warn!("{} の PNG デコード失敗: {}", role, e);
                        continue;
                    }
                }
            } else if ext == "ani" {
                // .ani: 全フレームを PNG 化して frames に詰める。テーマ詳細ドロワーが
                // この frames を見て `<CursorPreview kind="ani">` でアニメ再生する。
                // 静止プレビュー (asset.png_bytes) は先頭フレームを使い、ホットスポットは
                // フレーム内蔵の px 値をフレーム幅で比率化して採用する (静止表示時の
                // hot ドット位置に使う)。
                match parse_ani(&bytes).and_then(|parsed| {
                    let first = parsed.frames.first().ok_or_else(|| {
                        crate::errors::AppError::ImageProcessing(
                            ".ani にフレームがありません".to_string(),
                        )
                    })?;
                    let w = first.image.width();
                    let h = first.image.height();
                    let hotspot = types::Hotspot::from_px(first.hotspot_x, first.hotspot_y, w);
                    let frames = build_ani_frame_data(&parsed)?;
                    let first_png = frames.frame_pngs.first().cloned().unwrap_or_default();
                    Ok(RolePreview {
                        asset: types::CursorAssetDescriptor {
                            png_bytes: first_png,
                            width: w,
                            height: h,
                            hotspot,
                        },
                        frames: Some(frames),
                    })
                }) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!("{} の .ani PNG 化に失敗: {}", role, e);
                        continue;
                    }
                }
            } else {
                match parse_ico_cur(&bytes).and_then(|p| pick_largest_as_png(&p)) {
                    Ok((entry, png)) => RolePreview {
                        asset: types::CursorAssetDescriptor {
                            png_bytes: png,
                            width: entry.width,
                            height: entry.height,
                            hotspot: types::Hotspot::from_px(
                                entry.hotspot_x,
                                entry.hotspot_y,
                                entry.width,
                            ),
                        },
                        frames: None,
                    },
                    Err(e) => {
                        tracing::warn!("{} の PNG 化に失敗: {}", role, e);
                        continue;
                    }
                }
            };
            out.insert(role.clone(), preview);
        }
        Ok(out)
    }

    /// 任意の `.cur` / `.ico` / `.ani` ファイル 1 件を最大解像度の PNG にレンダリングする。
    ///
    /// ライブラリ画面で Windows のシステムスキーム (HKCU\Cursors\Schemes) を表示する際、
    /// `%SystemRoot%\cursors\*.cur` 等を直接サムネイル化するために使う。
    ///
    /// 拡張子で判別:
    ///  - `.png` ならバイト列をそのまま返す
    ///  - `.ani` なら `parse_ani` で先頭フレームを取り出して PNG 化
    ///  - それ以外 (`.cur` / `.ico`) は `parse_ico_cur` + `pick_largest_as_png`
    pub fn render_cursor_file_as_png(path: &std::path::Path) -> AppResult<Vec<u8>> {
        use crate::cursor::{parse_ani, parse_ico_cur, pick_largest_as_png};
        use crate::errors::AppError;
        let bytes = std::fs::read(path).map_err(|e| {
            AppError::ImageProcessing(format!(
                "ファイル読込失敗 ({}): {}",
                crate::logging::redact_path(path),
                e
            ))
        })?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "png" {
            return Ok(bytes);
        }
        if ext == "ani" {
            let parsed = parse_ani(&bytes)?;
            let frame = parsed.frames.first().ok_or_else(|| {
                AppError::ImageProcessing(".ani にフレームがありません".to_string())
            })?;
            let mut png = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut png);
            image::ImageEncoder::write_image(
                encoder,
                frame.image.as_raw(),
                frame.image.width(),
                frame.image.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| AppError::ImageProcessing(format!("PNG エンコード失敗: {}", e)))?;
            return Ok(png);
        }
        // .cur / .ico
        let parsed = parse_ico_cur(&bytes)?;
        let (_, png) = pick_largest_as_png(&parsed)?;
        Ok(png)
    }

    /// 役割名 → 絶対パス のマップから、各ロールのサムネイル PNG を生成する。
    ///
    /// 失敗するロールはスキップしてログに残す (1 つの壊れたファイルで全体表示を
    /// 諦めない設計)。空文字列パスもスキップ。
    pub fn render_paths_as_previews(
        cursor_paths: &HashMap<String, String>,
    ) -> HashMap<String, Vec<u8>> {
        let mut out: HashMap<String, Vec<u8>> = HashMap::new();
        for (role, raw) in cursor_paths {
            if raw.is_empty() {
                continue;
            }
            let path = std::path::PathBuf::from(raw);
            if !path.is_file() {
                tracing::debug!("scheme preview skip ({}): ファイル不在", role);
                continue;
            }
            match Self::render_cursor_file_as_png(&path) {
                Ok(bytes) => {
                    out.insert(role.clone(), bytes);
                }
                Err(e) => {
                    tracing::warn!("scheme preview {} のレンダリング失敗: {}", role, e);
                }
            }
        }
        out
    }

    /// [`render_paths_as_previews`] のリッチ版。各ロールに [`RolePreview`] を返す。
    ///
    /// Windows システムスキーム (HKCU\Cursors\Schemes) のテーマ詳細ドロワーで
    /// ホットスポット位置を視覚化する用途のみ使用。`.ani` 形式は先頭フレームの
    /// 寸法のみ取り出し、ホットスポット情報は持たないので (0, 0) として返す。
    pub fn render_paths_as_previews_with_hotspots(
        cursor_paths: &HashMap<String, String>,
    ) -> HashMap<String, RolePreview> {
        use crate::cursor::{parse_ani, parse_ico_cur, pick_largest_as_png};

        let mut out: HashMap<String, RolePreview> = HashMap::new();
        for (role, raw) in cursor_paths {
            if raw.is_empty() {
                continue;
            }
            let path = std::path::PathBuf::from(raw);
            if !path.is_file() {
                continue;
            }
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("scheme preview 読込失敗 ({}): {}", role, e);
                    continue;
                }
            };
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let preview = if ext == "png" {
                match image::load_from_memory_with_format(&bytes, image::ImageFormat::Png) {
                    Ok(img) => RolePreview {
                        asset: types::CursorAssetDescriptor {
                            png_bytes: bytes,
                            width: img.width(),
                            height: img.height(),
                            hotspot: types::Hotspot::ZERO,
                        },
                        frames: None,
                    },
                    Err(e) => {
                        tracing::warn!("scheme preview {} の PNG デコード失敗: {}", role, e);
                        continue;
                    }
                }
            } else if ext == "ani" {
                // .ani: 全フレームを PNG 化して frames に詰める (テーマ詳細ドロワーで
                // システムスキームの ANI もアニメーション再生させるため)。
                // 静止プレビュー (asset.png_bytes) は先頭フレーム。ホットスポットは
                // 既存挙動と互換性を保つため Hotspot::ZERO とする (スキームのカーソル
                // パスをホットスポット情報無しで返してきた経緯がある)。
                match parse_ani(&bytes).and_then(|parsed| {
                    let first = parsed.frames.first().ok_or_else(|| {
                        crate::errors::AppError::ImageProcessing(
                            ".ani にフレームがありません".to_string(),
                        )
                    })?;
                    let w = first.image.width();
                    let h = first.image.height();
                    let frames = build_ani_frame_data(&parsed)?;
                    let first_png = frames.frame_pngs.first().cloned().unwrap_or_default();
                    Ok(RolePreview {
                        asset: types::CursorAssetDescriptor {
                            png_bytes: first_png,
                            width: w,
                            height: h,
                            hotspot: types::Hotspot::ZERO,
                        },
                        frames: Some(frames),
                    })
                }) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!("scheme preview {} の .ani 変換失敗: {}", role, e);
                        continue;
                    }
                }
            } else {
                match parse_ico_cur(&bytes).and_then(|p| pick_largest_as_png(&p)) {
                    Ok((entry, png)) => RolePreview {
                        asset: types::CursorAssetDescriptor {
                            png_bytes: png,
                            width: entry.width,
                            height: entry.height,
                            hotspot: types::Hotspot::from_px(
                                entry.hotspot_x,
                                entry.hotspot_y,
                                entry.width,
                            ),
                        },
                        frames: None,
                    },
                    Err(e) => {
                        tracing::warn!("scheme preview {} の .cur/.ico 変換失敗: {}", role, e);
                        continue;
                    }
                }
            };
            out.insert(role.clone(), preview);
        }
        out
    }

    /// `.cursorpack` ビルド時、最終 ZIP の `previews/<role>.png` に書き込むバイト列を生成する。
    ///
    /// - `cursors` は role → `.cur` / `.ani` 等のビルド済みバイト列
    /// - 結果は PREVIEW_ROLES 順、テーマに含まれるロールのみ
    /// - 各画像は最大解像度エントリ / ANI 先頭フレームを 64×64 にリサイズ
    pub fn build_preview_pngs(
        cursors: &HashMap<String, Vec<u8>>,
        cursor_metas: &HashMap<String, CursorDefinition>,
    ) -> HashMap<String, Vec<u8>> {
        use crate::cursor::{parse_ani, parse_ico_cur, pick_largest_as_png};
        use image::ImageEncoder;

        let mut out: HashMap<String, Vec<u8>> = HashMap::new();
        for role in Self::PREVIEW_ROLES.iter() {
            let role_s = role.to_string();
            let Some(bytes) = cursors.get(&role_s) else {
                continue;
            };
            let def = cursor_metas.get(&role_s);
            let ext = def
                .and_then(|d| std::path::Path::new(&d.file).extension())
                .and_then(|e| e.to_str())
                .unwrap_or("cur")
                .to_ascii_lowercase();

            // RGBA8 を取り出す
            let rgba: Option<image::RgbaImage> = match ext.as_str() {
                "png" => image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
                    .ok()
                    .map(|img| img.to_rgba8()),
                "ani" => parse_ani(bytes)
                    .ok()
                    .and_then(|p| p.frames.into_iter().next().map(|f| f.image)),
                _ => parse_ico_cur(bytes)
                    .ok()
                    .and_then(|p| pick_largest_as_png(&p).ok().map(|(entry, _)| entry.image)),
            };

            let Some(img) = rgba else { continue };

            // 64×64 にリサイズ
            let resized = image::imageops::resize(
                &img,
                Self::PREVIEW_SIZE,
                Self::PREVIEW_SIZE,
                image::imageops::FilterType::Lanczos3,
            );

            let mut png = Vec::new();
            if image::codecs::png::PngEncoder::new(&mut png)
                .write_image(
                    resized.as_raw(),
                    Self::PREVIEW_SIZE,
                    Self::PREVIEW_SIZE,
                    image::ExtendedColorType::Rgba8,
                )
                .is_ok()
            {
                out.insert(role_s, png);
            }
        }
        out
    }
}

#[cfg(test)]
mod role_preview_tests {
    use super::*;
    use crate::theme::types::{AniFrameData, CursorAssetDescriptor, Hotspot};

    #[test]
    fn role_preview_serializes_with_flat_keys() {
        let p = RolePreview {
            asset: CursorAssetDescriptor {
                png_bytes: vec![1, 2, 3],
                width: 32,
                height: 32,
                hotspot: Hotspot::ZERO,
            },
            frames: None,
        };
        let v = serde_json::to_value(&p).unwrap();
        // flatten 後は asset サブオブジェクトが出ず、top-level に展開される
        assert!(v.get("asset").is_none(), "asset should be flattened away");
        assert!(v.get("pngBytes").is_some());
        assert_eq!(v["width"], 32);
        assert_eq!(v["height"], 32);
        assert!(v.get("hotspot").is_some());
        // frames: None のときは JSON にキーが出てはいけない (後方互換)
        assert!(
            v.get("frames").is_none(),
            "frames は None のとき省略されるべき: {v}"
        );
    }

    #[test]
    fn role_preview_serializes_frames_when_some() {
        // .ani ロールのケース。frames は AniFrameData 形 (camelCase) でネストする。
        let p = RolePreview {
            asset: CursorAssetDescriptor {
                png_bytes: vec![1, 2, 3],
                width: 32,
                height: 32,
                hotspot: Hotspot::ZERO,
            },
            frames: Some(AniFrameData {
                frame_pngs: vec![vec![10, 20], vec![30, 40]],
                sequence: vec![0, 1],
                per_step_durations_ms: vec![100, 100],
                is_legacy_raw_dib: false,
            }),
        };
        let v = serde_json::to_value(&p).unwrap();
        // frames は flatten せずにネスト構造で出す (asset 平坦化と区別)
        let frames = v.get("frames").expect("frames must exist");
        assert!(frames.get("framePngs").is_some());
        assert_eq!(frames["sequence"], serde_json::json!([0, 1]));
        assert_eq!(frames["perStepDurationsMs"], serde_json::json!([100, 100]));
        assert_eq!(frames["isLegacyRawDib"], false);
    }
}
