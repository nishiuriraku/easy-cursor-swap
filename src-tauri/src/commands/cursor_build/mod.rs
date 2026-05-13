//! `.cursorpack` ビルド・エクスポート系 IPC。
//!
//! クリエイターから渡された PNG / メタ情報を 17 役割 × 6 サイズの `.cur` バイナリへ
//! 変換し、theme.json と一緒に zip に固める。
//!
//! - [`export_cursorpack`] — 役割パス → `.cursorpack` (同期)
//! - [`export_cursorpack_streamed`] — 進捗イベント付きビルド (UI からの主流ルート)
//! - [`cancel_build`] — `export_cursorpack_streamed` を中止
//!
//! キャンセルは `OnceLock<Mutex<HashSet<String>>>` 上の build_id 集合で管理する。

pub mod dto;
pub use dto::*;

pub mod cancel;
pub use cancel::cancel_build;

mod build;
mod sign;
pub mod stream;

use crate::errors::AppError;
use crate::theme::{CursorDefinition, LocalizedString, ThemeManager, ThemeMetadata};

#[tauri::command]
pub fn export_cursorpack(req: ExportCursorpackRequest) -> Result<ExportResult, AppError> {
    use std::collections::HashMap;

    // 1) cursors マップ構築
    let mut cursors_meta: HashMap<String, CursorDefinition> = HashMap::new();
    let mut cursor_bytes: HashMap<String, Vec<u8>> = HashMap::new();
    for (role, path) in &req.cur_paths {
        let path = std::path::PathBuf::from(path);
        let bin = std::fs::read(&path).map_err(|e| {
            AppError::Theme(format!(
                "カーソル {} が読み込めません ({}): {}",
                role,
                path.display(),
                e
            ))
        })?;
        let hot = req
            .hotspots
            .get(role)
            .cloned()
            .unwrap_or(crate::theme::types::Hotspot::ZERO);
        // .cur ファイル自体は既ビルド済み (cur_paths で受領)。theme.json に ratio を記録するのみ (変換不要)
        cursors_meta.insert(
            role.clone(),
            CursorDefinition {
                file: format!("cursors/{}.cur", role),
                hotspot: hot,
                resize_method: "lanczos".to_string(),
                size_overrides: None,
            },
        );
        cursor_bytes.insert(role.clone(), bin);
    }

    // 2) theme.json メタデータ
    let mut name_map = HashMap::new();
    name_map.insert("ja".to_string(), req.name_ja.clone());
    if let Some(en) = req.name_en.clone() {
        name_map.insert("en".to_string(), en);
    }

    let mut metadata = ThemeMetadata {
        schema_version: 1,
        id: uuid::Uuid::new_v4(),
        name: LocalizedString::Localized(name_map),
        version: req.version.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        requires_os_shadow: req.requires_os_shadow,
        cursors: cursors_meta,
        author: req.author.clone(),
        license: None,
        homepage: None,
        // Creator UI の説明欄 (`metaDescription`) 由来。空文字 / 空白のみは
        // None と同じ扱い (= theme.json から description フィールドごと省略)。
        description: req
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| LocalizedString::Simple(s.to_string())),
        min_app_version: None,
        signature: None,
        tags: Vec::new(),
    };

    // 3) 署名 (sign=true の場合)。sign.rs に共通化済み。
    let signed_key_id: Option<String> = if req.sign {
        sign::sign_theme_metadata(&mut metadata)?
    } else {
        None
    };

    // 4) Zip 出力
    let out_path = std::path::PathBuf::from(&req.output_path);
    let size = ThemeManager::export_cursorpack(&mut metadata, &cursor_bytes, &out_path)?;

    Ok(ExportResult {
        theme_id: metadata.id.to_string(),
        size_bytes: size,
        signed: req.sign,
        key_id: signed_key_id,
        applied: false,
        apply_error: None,
    })
}

// ストリーム式 .cursorpack ビルドは stream.rs に分離 (Phase 3b)。
// 17 役割 × 6 サイズ = 最大 102 枚の .cur 生成と進捗イベント配信、署名、
// destination 分岐 (File / Library) を内部で実装。

#[cfg(test)]
mod tests {
    use super::cancel::{clear_cancel, is_cancelled, mark_cancelled};
    use super::SizedOverridePayload;

    #[test]
    fn cancel_flag_lifecycle() {
        let id = "test-build-cancel-lifecycle-xyz";
        // ユニーク ID なので前提状態は false
        assert!(!is_cancelled(id));
        mark_cancelled(id);
        assert!(is_cancelled(id));
        clear_cancel(id);
        assert!(!is_cancelled(id));
    }

    #[test]
    fn cancel_flags_are_independent_per_build_id() {
        let id_a = "test-build-independent-a-xyz";
        let id_b = "test-build-independent-b-xyz";
        mark_cancelled(id_a);
        assert!(is_cancelled(id_a));
        assert!(!is_cancelled(id_b));
        clear_cancel(id_a);
    }

    #[test]
    fn export_destination_file_round_trip() {
        let json = serde_json::json!({
            "kind": "file",
            "path": "/tmp/out.cursorpack"
        });
        let dest: super::ExportDestination = serde_json::from_value(json).unwrap();
        match dest {
            super::ExportDestination::File { path } => assert_eq!(path, "/tmp/out.cursorpack"),
            _ => panic!("expected File variant"),
        }
    }

    #[test]
    fn export_destination_library_round_trip() {
        let json = serde_json::json!({
            "kind": "library",
            "applyAfter": true
        });
        let dest: super::ExportDestination = serde_json::from_value(json).unwrap();
        match dest {
            super::ExportDestination::Library { apply_after } => assert!(apply_after),
            _ => panic!("expected Library variant"),
        }
    }

    /// `description` フィールドは Some(String) でも欠落でも受け取れる必要がある。
    /// (古いフロントとの後方互換 + 説明欄が空のときの省略)
    #[test]
    fn streamed_request_accepts_description_present_and_missing() {
        // (1) description フィールドが存在 + 非空
        let with_desc = serde_json::json!({
            "buildId": "id1",
            "nameJa": "T",
            "nameEn": null,
            "author": null,
            "version": "1.0.0",
            "description": "今回のテーマは……",
            "requiresOsShadow": false,
            "roles": [],
            "destination": { "kind": "file", "path": "/tmp/x" },
            "existingThemeId": null,
            "sign": false
        });
        let req: super::StreamedExportRequest = serde_json::from_value(with_desc).unwrap();
        assert_eq!(req.description.as_deref(), Some("今回のテーマは……"));

        // (2) description フィールド欠落 → None
        let no_desc = serde_json::json!({
            "buildId": "id2",
            "nameJa": "T",
            "nameEn": null,
            "author": null,
            "version": "1.0.0",
            "requiresOsShadow": false,
            "roles": [],
            "destination": { "kind": "file", "path": "/tmp/x" },
            "existingThemeId": null,
            "sign": false
        });
        let req: super::StreamedExportRequest = serde_json::from_value(no_desc).unwrap();
        assert!(req.description.is_none());

        // (3) description フィールド null → None
        let null_desc = serde_json::json!({
            "buildId": "id3",
            "nameJa": "T",
            "nameEn": null,
            "author": null,
            "version": "1.0.0",
            "description": null,
            "requiresOsShadow": false,
            "roles": [],
            "destination": { "kind": "file", "path": "/tmp/x" },
            "existingThemeId": null,
            "sign": false
        });
        let req: super::StreamedExportRequest = serde_json::from_value(null_desc).unwrap();
        assert!(req.description.is_none());
    }

    #[test]
    fn streamed_request_deserializes_with_existing_theme_id_null() {
        let json = serde_json::json!({
            "buildId": "test-id",
            "nameJa": "T",
            "nameEn": null,
            "author": null,
            "version": "1.0.0",
            "requiresOsShadow": false,
            "roles": [],
            "destination": { "kind": "file", "path": "/tmp/x" },
            "existingThemeId": null,
            "sign": false
        });
        let req: super::StreamedExportRequest = serde_json::from_value(json).unwrap();
        assert!(req.existing_theme_id.is_none());
        assert!(matches!(
            req.destination,
            super::ExportDestination::File { .. }
        ));
    }

    #[test]
    fn metadata_id_inherits_existing_theme_id_when_provided() {
        // Helper を直接テストする (フル export_cursorpack_streamed は AppHandle 必要のため)
        let existing = uuid::Uuid::new_v4();
        let resolved = super::stream::resolve_metadata_id(Some(existing));
        assert_eq!(resolved, existing);
    }

    #[test]
    fn metadata_id_generates_new_when_existing_is_none() {
        let a = super::stream::resolve_metadata_id(None);
        let b = super::stream::resolve_metadata_id(None);
        assert_ne!(a, b, "別の Uuid::new_v4() が生成されるはず");
    }

    #[test]
    fn library_destination_zip_round_trips_through_import() {
        // 注: import_cursorpack_bytes は ConfigManager::cursors_dir() を呼ぶため、
        // 一時的なホームディレクトリ環境で実行する必要がある。
        // ここでは write_cursorpack_to_buffer の出力が
        // inspect_cursorpack_bytes でメタを取り出せることだけ確認する
        // (フル展開は手動 E2E に委ねる)。
        use std::collections::HashMap;
        let mut metadata = crate::theme::types::ThemeMetadata {
            schema_version: 1,
            id: uuid::Uuid::new_v4(),
            name: crate::theme::types::LocalizedString::Simple("Lib Test".to_string()),
            version: "1.2.3".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            requires_os_shadow: false,
            cursors: HashMap::new(),
            author: None,
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
            tags: Vec::new(),
        };
        let target_id = metadata.id;
        let cursors: HashMap<String, Vec<u8>> = HashMap::new();
        let bytes = crate::theme::ThemeManager::write_cursorpack_to_buffer(&mut metadata, &cursors)
            .unwrap();
        let inspected = crate::theme::ThemeManager::inspect_cursorpack_bytes(&bytes).unwrap();
        assert_eq!(inspected.id, target_id, "ID が引き継がれているはず");
        assert_eq!(inspected.version, "1.2.3");
    }

    #[test]
    fn ratio_hotspot_converts_to_px_at_each_size() {
        use crate::theme::types::{Hotspot, Ratio01};
        let h = Hotspot {
            x: Ratio01::new(0.5),
            y: Ratio01::new(0.5),
        };
        assert_eq!(h.to_px(32), (16, 16));
        assert_eq!(h.to_px(64), (32, 32));
        assert_eq!(h.to_px(128), (64, 64));
        assert_eq!(h.to_px(256), (128, 128));
    }

    #[test]
    fn sized_override_hotspot_overrides_primary() {
        use crate::theme::types::{Hotspot, Ratio01};
        let primary = Hotspot {
            x: Ratio01::new(0.0),
            y: Ratio01::new(0.0),
        };
        let override_h = Hotspot {
            x: Ratio01::new(0.5),
            y: Ratio01::new(0.5),
        };
        let payload = SizedOverridePayload {
            png_bytes: vec![],
            hotspot: Some(override_h),
        };
        let effective = payload.hotspot.unwrap_or(primary);
        assert_eq!(effective, override_h);
    }

    /// SizedOverridePayload.hotspot (比率) が build_cur_from_png の出力 .cur バイナリに
    /// 正しいホットスポット px として記録されることを検証する。
    ///
    /// primary hotspot = (0.0, 0.0) → px=(0,0) で、
    /// 64px オーバーライドの hotspot = (0.5, 0.5) → px=(32,32) を指定した場合、
    /// 出力 .cur の 64px エントリは hotspot=(32,32) になるはず。
    #[test]
    fn sized_override_hotspot_reaches_cur_build_output() {
        use crate::cursor::ico_cur::parse_ico_cur;
        use crate::cursor::{build_cur_from_png, ResizeMethod};
        use crate::theme::types::{Hotspot, Ratio01};

        // 64x64 の赤 PNG (オーバーライド)
        let img64: image::RgbaImage =
            image::ImageBuffer::from_pixel(64, 64, image::Rgba([255, 0, 0, 255]));
        let mut png64 = Vec::new();
        image::ImageEncoder::write_image(
            image::codecs::png::PngEncoder::new(&mut png64),
            img64.as_raw(),
            64,
            64,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();

        // primary は 256x256 の青、hotspot = (0, 0)
        let img256: image::RgbaImage =
            image::ImageBuffer::from_pixel(256, 256, image::Rgba([0, 0, 255, 255]));
        let mut png256 = Vec::new();
        image::ImageEncoder::write_image(
            image::codecs::png::PngEncoder::new(&mut png256),
            img256.as_raw(),
            256,
            256,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();

        // 64px オーバーライドに hotspot (0.5, 0.5) → to_px(64) = (32, 32) を設定
        let override_hotspot = Hotspot {
            x: Ratio01::new(0.5),
            y: Ratio01::new(0.5),
        };
        let (ov_hx, ov_hy) = override_hotspot.to_px(64);
        assert_eq!((ov_hx, ov_hy), (32, 32));

        let mut sized_png_map = std::collections::HashMap::new();
        sized_png_map.insert(64u32, png64.clone());

        let mut sized_hotspot_map = std::collections::HashMap::new();
        sized_hotspot_map.insert(64u32, (ov_hx, ov_hy));

        // primary hotspot = (0, 0) で build_cur_from_png に per_size_hotspot_px を渡す
        let cur_bytes = build_cur_from_png(
            &png256,
            0,
            0,
            ResizeMethod::Lanczos,
            Some(&sized_png_map),
            Some(&sized_hotspot_map),
        )
        .unwrap();

        let parsed = parse_ico_cur(&cur_bytes).unwrap();

        // 64px エントリのホットスポットがオーバーライドの (32, 32) になっていることを確認
        let entry_64 = parsed
            .entries
            .iter()
            .find(|e| e.width == 64)
            .expect("64px エントリがあるはず");
        assert_eq!(
            (entry_64.hotspot_x, entry_64.hotspot_y),
            (32, 32),
            "64px エントリのホットスポットはオーバーライドの (32,32) であるべき"
        );

        // 32px エントリ (オーバーライドなし) のホットスポットは primary (0,0) からスケールされた (0,0)
        let entry_32 = parsed
            .entries
            .iter()
            .find(|e| e.width == 32)
            .expect("32px エントリがあるはず");
        assert_eq!(
            (entry_32.hotspot_x, entry_32.hotspot_y),
            (0, 0),
            "32px エントリのホットスポットは primary (0,0) のスケール値 (0,0) であるべき"
        );
    }
}
