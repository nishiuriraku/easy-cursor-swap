//! `.cursorpack` を Creator 用に解凍してロール毎に PNG プレビューを抽出する。
//!
//! 通常の `import_cursorpack` は `~/.custom_cursors/<UUID>/` に展開してライブラリ入りさせるが、
//! Creator の「既存パックを取り込んで編集」フローではディスク書き込みせず
//! メモリ上で PNG バイトを取り出す必要がある。本モジュールはその専用パイプライン。

use super::{BulkImportProgress, ParseCursorpackRequest, ParsedCursorpack, ParsedRole};
use crate::errors::AppError;
use crate::theme::types::AniFrameData;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use tauri::{AppHandle, Emitter};
use zip::ZipArchive;

use super::CursorpackMetadata;

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
        id: Some(meta.id.to_string()),
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

/// `parse_ani_role` の戻り値。`.ani` ロールから抽出した primary プレビューと
/// フレーム列、(あれば) 展開先絶対パスを束ねたもの。
struct AniRoleParseResult {
    primary_size: u32,
    primary_png: Vec<u8>,
    hotspot: crate::theme::types::Hotspot,
    ani: AniFrameData,
    ani_source_path: Option<String>,
}

/// `.ani` ロールを解析して `AniRoleParseResult` を返す。
/// `ani_extract_dir` が指定されていればロールの元バイトを `<dir>/<role-filename>` に
/// 書き出し、その絶対パスを返す (export 時の rewrite_ani_with_hotspot ソースに使う)。
fn parse_ani_role(
    role_id: &str,
    file_in_zip: &str,
    bytes: &[u8],
    ani_extract_dir: Option<&Path>,
) -> Result<AniRoleParseResult, AppError> {
    let parsed = crate::cursor::parse_ani(bytes)?;
    let frame0 = parsed.frames.first().ok_or_else(|| {
        AppError::ImageProcessing(format!("ロール {} の .ani にフレームがありません", role_id))
    })?;
    let primary_png = {
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        image::ImageEncoder::write_image(
            encoder,
            frame0.image.as_raw(),
            frame0.image.width(),
            frame0.image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| AppError::ImageProcessing(format!("PNG エンコード失敗: {}", e)))?;
        buf
    };

    // 全フレームを PNG 化
    let mut frame_pngs: Vec<Vec<u8>> = Vec::with_capacity(parsed.frames.len());
    for f in &parsed.frames {
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        if image::ImageEncoder::write_image(
            encoder,
            f.image.as_raw(),
            f.image.width(),
            f.image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .is_err()
        {
            continue;
        }
        frame_pngs.push(buf);
    }
    let per_step_durations_ms: Vec<u32> = parsed
        .per_step_rate_jiffies
        .iter()
        .map(|j| ((*j as u64 * 1000) / 60) as u32)
        .collect();

    // export 用にバイトを展開
    let ani_source_path = if let Some(dir) = ani_extract_dir {
        std::fs::create_dir_all(dir)?;
        // file_in_zip にはサブディレクトリ含む可能性があるのでファイル名だけ取り出す
        let fname = Path::new(file_in_zip)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("{}.ani", role_id));
        let out_path = dir.join(&fname);
        std::fs::write(&out_path, bytes)?;
        Some(out_path.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(AniRoleParseResult {
        primary_size: frame0.image.width(),
        primary_png,
        hotspot: crate::theme::types::Hotspot::from_px(
            frame0.hotspot_x,
            frame0.hotspot_y,
            frame0.image.width(),
        ),
        ani: AniFrameData {
            frame_pngs,
            sequence: parsed.sequence,
            per_step_durations_ms,
            is_legacy_raw_dib: parsed.is_legacy_raw_dib,
        },
        ani_source_path,
    })
}

pub fn parse_cursorpack_inner(bytes: &[u8]) -> Result<ParsedCursorpack, AppError> {
    parse_cursorpack_inner_with_extract(bytes, None)
}

/// `parse_cursorpack_inner` の `.ani` 展開先指定版。
/// `ani_extract_dir` を渡すと、`.ani` ロールのバイトをそこに書き出して
/// 各 ParsedRole の `ani_source_path` に絶対パスを格納する。
pub fn parse_cursorpack_inner_with_extract(
    bytes: &[u8],
    ani_extract_dir: Option<&Path>,
) -> Result<ParsedCursorpack, AppError> {
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

        // 拡張子で `.ani` を判定。.cur / .ico は従来通り parse_ico_cur に流す。
        let is_ani = Path::new(&def.file)
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| e.eq_ignore_ascii_case("ani"))
            .unwrap_or(false);

        if is_ani {
            let r = parse_ani_role(role_id, &def.file, &primary_bytes, ani_extract_dir)?;
            // .ani には sized オーバーライドの概念がないので空 HashMap
            roles.insert(
                role_id.clone(),
                ParsedRole {
                    asset: crate::theme::types::CursorAssetDescriptor {
                        png_bytes: r.primary_png,
                        width: r.primary_size,
                        height: r.primary_size,
                        hotspot: r.hotspot,
                    },
                    sized_png_bytes: HashMap::new(),
                    ani: Some(r.ani),
                    ani_source_path: r.ani_source_path,
                },
            );
            continue;
        }

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
                asset: crate::theme::types::CursorAssetDescriptor {
                    png_bytes: primary_png,
                    width: largest.width,
                    height: largest.height,
                    hotspot: def.hotspot,
                },
                sized_png_bytes: sized,
                ani: None,
                ani_source_path: None,
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
        // `.ani` ロールのバイトは export 時に rewrite_ani_with_hotspot で再利用するため、
        // `<cursorpack>.extracted/` に書き出してパスを ParsedRole.ani_source_path に格納する。
        // ディレクトリは cursorpack と同じ寿命 (一時テーマ複製では tempDir() 配下) なので
        // 通常のテーマ保存までは生きている。
        let extract_dir = {
            let p = Path::new(&req.path);
            let suffix = format!(
                "{}.extracted",
                p.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("cursorpack")
            );
            p.parent().map(|d| d.join(suffix))
        };
        let r = parse_cursorpack_inner_with_extract(&bytes, extract_dir.as_deref())?;
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
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../sample-icon");
        p
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
        // creator.vue ?editPath 経路で sourceThemeId にそのまま代入されるため、
        // metadata.id が必ず UUID 文字列で返ることを契約として固定する。
        // 過去にここが欠落していた結果、SaveDestinationModal の「上書き / 複製」
        // セクションが永久に出ないバグになっていた。
        let id = parsed
            .metadata
            .id
            .as_deref()
            .expect("metadata.id must be Some");
        assert!(
            uuid::Uuid::parse_str(id).is_ok(),
            "metadata.id should be a parseable UUID, got {id:?}"
        );
    }
}
