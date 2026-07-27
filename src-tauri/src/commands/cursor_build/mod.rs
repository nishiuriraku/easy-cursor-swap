//! `.cursorpack` ビルド・エクスポート系 IPC。
//!
//! クリエイターから渡された PNG / メタ情報を 17 役割 × 6 サイズの `.cur` バイナリへ
//! 変換し、theme.json と一緒に zip に固める。
//!
//! - [`export_cursorpack_streamed`] — 進捗イベント付きビルド (UI からの主流ルート)
//! - [`cancel_build`] — `export_cursorpack_streamed` を中止
//!
//! キャンセルは `crate::cancel_registry::CancelRegistry` (App state) で管理する
//! — bulk_import と同じ仕組みに統一済み。

pub mod dto;
pub use dto::*;

mod build;
mod sign;
pub mod stream;

use crate::cancel_registry::CancelRegistry;

/// 進行中の build を中止する。実際の中止は次のチェックポイントで行われる。
#[tauri::command]
pub fn cancel_build(registry: tauri::State<'_, CancelRegistry>, build_id: String) {
    registry.cancel(&build_id);
    tracing::info!("ビルド中止要求: {}", build_id);
}

// ストリーム式 .cursorpack ビルドは stream.rs に分離 (Phase 3b)。
// 17 役割 × 6 サイズ = 最大 102 枚の .cur 生成と進捗イベント配信、署名、
// destination 分岐 (File / Library) を内部で実装。

#[cfg(test)]
mod tests {
    use super::CancelRegistry;
    use super::SizedOverridePayload;

    #[test]
    fn cancel_flag_lifecycle() {
        let registry = CancelRegistry::default();
        let id = "test-build-cancel-lifecycle-xyz";
        // 新規 instance なので前提状態は false
        assert!(!registry.is_cancelled(id));
        // cancel() は登録済みジョブにのみ作用する (Y15)。ワーカーは register してから走る。
        registry.register(id);
        registry.cancel(id);
        assert!(registry.is_cancelled(id));
        registry.drop_job(id);
        assert!(!registry.is_cancelled(id));
    }

    #[test]
    fn cancel_flags_are_independent_per_build_id() {
        let registry = CancelRegistry::default();
        let id_a = "test-build-independent-a-xyz";
        let id_b = "test-build-independent-b-xyz";
        registry.register(id_a);
        registry.cancel(id_a);
        assert!(registry.is_cancelled(id_a));
        assert!(!registry.is_cancelled(id_b));
        registry.drop_job(id_a);
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
            source: crate::theme::types::ThemeSource::Local,
            cloned_from_marketplace_id: None,
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

    /// CUR 6 サイズ標準パスの contract: `build_cur_from_png` は CURSOR_SIZES にある
    /// 6 つのサイズ (32, 48, 64, 96, 128, 256) 全てを含む `.cur` を生成する。
    /// 旧実装で 32 固定の単一サイズしか返さない回帰が入っていないかを固定する。
    #[test]
    fn build_cur_from_png_emits_all_six_canonical_sizes() {
        use crate::cursor::build_cur_from_png;
        use crate::cursor::ico_cur::parse_ico_cur;
        use crate::cursor::ResizeMethod;

        // 256x256 の元画像 (十分なサイズなので 6 リサイズ全てが縮小方向)
        let img: image::RgbaImage =
            image::ImageBuffer::from_pixel(256, 256, image::Rgba([100, 200, 50, 255]));
        let mut png = Vec::new();
        image::ImageEncoder::write_image(
            image::codecs::png::PngEncoder::new(&mut png),
            img.as_raw(),
            256,
            256,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();

        let cur_bytes = build_cur_from_png(&png, 10, 20, ResizeMethod::Lanczos, None, None)
            .expect("build_cur_from_png should succeed for valid PNG");
        let parsed = parse_ico_cur(&cur_bytes).expect("output should be parseable .cur");

        // 全 6 サイズ ([32, 48, 64, 96, 128, 256]) が含まれる
        assert_eq!(
            parsed.entries.len(),
            6,
            "expected exactly 6 .cur entries for canonical sizes, got {}",
            parsed.entries.len()
        );
        let widths: Vec<u32> = parsed.entries.iter().map(|e| e.width).collect();
        assert_eq!(widths, vec![32, 48, 64, 96, 128, 256]);

        // ホットスポットが全サイズで primary の (10, 20) を
        // `scale_hotspot(original_size, target)` でスケールした値として記録されている
        // ことを確認する。各エントリの幅は同じでも `scale_hotspot` の返しは
        // ターゲットサイズへのスケールなので、ホットスポット px の算出ロジックを
        // 巻き込んでいないことを確かめる意味でも、各エントリのホットスポットが
        // 妥当な範囲 (target_size 以内) に収まることを検証する。
        for entry in &parsed.entries {
            assert!(
                entry.hotspot_x <= entry.width,
                "hotspot_x {} must be <= width {}",
                entry.hotspot_x,
                entry.width
            );
            assert!(
                entry.hotspot_y <= entry.height,
                "hotspot_y {} must be <= height {}",
                entry.hotspot_y,
                entry.height
            );
        }
    }

    /// `build_cur_from_png` に渡す PNG が不正な Magic Byte だと即座に `Err` を返す
    /// contract。空入力や非 PNG を入れたときにバッファ読み込みで panic しないことを保証する。
    #[test]
    fn build_cur_from_png_rejects_invalid_png_magic() {
        use crate::cursor::build_cur_from_png;
        use crate::cursor::ResizeMethod;

        let bogus: &[u8] = b"NOT A PNG";
        let err = build_cur_from_png(bogus, 0, 0, ResizeMethod::Lanczos, None, None)
            .expect_err("non-PNG should fail");
        match err {
            crate::errors::AppError::ImageProcessing(msg) => {
                assert!(
                    msg.contains("PNG") || msg.contains("ヘッダー"),
                    "error must indicate PNG issue: {msg}"
                );
            }
            other => panic!("expected ImageProcessing, got {other:?}"),
        }

        // 空バイト列も同様
        let empty: &[u8] = &[];
        assert!(build_cur_from_png(empty, 0, 0, ResizeMethod::Lanczos, None, None).is_err());
    }

    /// `generate_cur_binary` は空入力に対し `AppError::ImageProcessing` を返す contract。
    /// 「画像なし」を 0 エントリの .cur として通してしまうと、ICONDIR.num_images=0
    /// の壊れたファイルが出来上がる。
    #[test]
    fn generate_cur_binary_rejects_empty_input() {
        use crate::cursor::generate_cur_binary;
        let entries: Vec<(image::RgbaImage, u32, u32)> = vec![];
        let err = generate_cur_binary(&entries).expect_err("empty should fail");
        match err {
            crate::errors::AppError::ImageProcessing(msg) => {
                assert!(
                    msg.contains("1枚も") || msg.contains("指定"),
                    "error must indicate empty input: {msg}"
                );
            }
            other => panic!("expected ImageProcessing, got {other:?}"),
        }
    }

    /// `generate_cur_binary` は複数サイズを単一 .cur にパッキングし、ICONDIR の
    /// エントリ数と num_images が一致する contract。
    #[test]
    fn generate_cur_binary_packs_multiple_sizes_with_matching_header() {
        use crate::cursor::generate_cur_binary;
        // 3 サイズ (32, 64, 128) を生成
        let make = |w: u32| image::ImageBuffer::from_pixel(w, w, image::Rgba([10, 20, 30, 255]));
        let entries = vec![
            (make(32), 1u32, 2u32),
            (make(64), 3u32, 4u32),
            (make(128), 5u32, 6u32),
        ];
        let cur_bytes = generate_cur_binary(&entries).expect("pack 3 entries");
        let parsed = crate::cursor::parse_ico_cur(&cur_bytes).expect("parse output");

        // ICONDIR.num_images が 3 で、entries も同じ 3 件
        assert_eq!(parsed.entries.len(), 3);
        assert_eq!(parsed.entries[0].width, 32);
        assert_eq!(parsed.entries[1].width, 64);
        assert_eq!(parsed.entries[2].width, 128);
    }

    /// `StreamedExportRequest` の `destination = Library { apply_after: false }` が
    /// 受理される contract。apply 経路がデフォルト無効でもエラーにならない。
    #[test]
    fn streamed_request_accepts_library_destination_without_apply() {
        let json = serde_json::json!({
            "buildId": "id",
            "nameJa": "T",
            "nameEn": null,
            "author": null,
            "version": "1.0.0",
            "requiresOsShadow": false,
            "roles": [],
            "destination": { "kind": "library", "applyAfter": false },
            "existingThemeId": null,
            "sign": false
        });
        let req: super::StreamedExportRequest = serde_json::from_value(json).unwrap();
        match req.destination {
            super::ExportDestination::Library { apply_after } => assert!(!apply_after),
            _ => panic!("expected Library variant"),
        }
    }

    /// `ExportResult` のシリアライズ形を lock する contract。
    /// 現在の実装では `serde(rename_all = ...)` 未指定 (= snake_case) なので、
    /// フロント側は `theme_id` / `size_bytes` をキーに読む。camelCase 化すると
    /// フロントが壊れるので、現状を固定する (= 既存フロントとの契約)。
    #[test]
    fn export_result_serializes_with_snake_case_fields() {
        let result = super::ExportResult {
            theme_id: "abc".to_string(),
            size_bytes: 1234,
            signed: true,
            key_id: Some("kid".to_string()),
            applied: false,
            apply_error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        // 期待する snake_case キーが含まれている
        assert!(json.contains("\"theme_id\""), "missing theme_id: {json}");
        assert!(
            json.contains("\"size_bytes\""),
            "missing size_bytes: {json}"
        );
        assert!(json.contains("\"signed\""), "missing signed: {json}");
        assert!(json.contains("\"key_id\""), "missing key_id: {json}");
        assert!(json.contains("\"applied\""), "missing applied: {json}");
        assert!(
            json.contains("\"apply_error\""),
            "missing apply_error: {json}"
        );
    }
}
