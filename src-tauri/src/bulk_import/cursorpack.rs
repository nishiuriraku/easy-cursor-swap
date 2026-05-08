//! `.cursorpack` を Creator 用に解凍してロール毎に PNG プレビューを抽出する。
//!
//! 通常の `import_cursorpack` は `~/.custom_cursors/<UUID>/` に展開してライブラリ入りさせるが、
//! Creator の「既存パックを取り込んで編集」フローではディスク書き込みせず
//! メモリ上で PNG バイトを取り出す必要がある。本モジュールはその専用パイプライン。

use super::{BulkImportProgress, ParseCursorpackRequest, ParsedCursorpack, ParsedRole};
use crate::errors::AppError;
use std::collections::HashMap;
use std::io::Read;
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
    }
}
