//! 単一ロールの PNG → 6 サイズ .cur (または .ani 再書き出し) のビルド純粋関数。
//!
//! `export_cursorpack_streamed` のロール処理ループから「1 ロール分のビルド」を
//! 切り出した部分。Tauri / IPC / 進捗イベントには依存せず、入出力のみで完結する。

use super::dto::RoleBuildEntry;
use crate::cursor::{build_cur_from_png, ResizeMethod};
use crate::errors::AppError;
use crate::theme::CursorDefinition;
use std::collections::HashMap;

/// 1 ロールのビルド結果。
/// - `bytes`: 完成 .cur / .ani バイナリ (zip に書き込む対象)
/// - `definition`: theme.json の `cursors[<role>]` に書く `CursorDefinition`
///   (zip 内のパスは `definition.file` に格納)
pub(super) struct BuiltRole {
    pub bytes: Vec<u8>,
    pub definition: CursorDefinition,
}

/// `entry` を 1 ロール分の zip 用バイナリ + CursorDefinition に変換する。
///
/// `entry.ani_source_path` が `Some` の場合は `.ani` リライト経路、それ以外は
/// `build_cur_from_png` 経由で 6 サイズ .cur を生成する。
pub(super) fn build_role(entry: &RoleBuildEntry) -> Result<BuiltRole, AppError> {
    // .ani 由来ロール: rewrite_ani_with_hotspot 経由で .ani バイナリを生成
    if let Some(src) = &entry.ani_source_path {
        let bytes = std::fs::read(src).map_err(|e| {
            AppError::ImageProcessing(format!(
                "ani 読込失敗 ({}): {}",
                crate::logging::redact_path(std::path::Path::new(src)),
                e
            ))
        })?;
        // ANI の primary_size はファイル自体を parse_ani してフレーム幅から取得する。
        // サムネイル PNG (png_bytes) は UI 表示用にリサイズされる場合があるため、
        // それを使うと ratio→px 変換がズレる可能性がある。
        let primary_size = crate::cursor::ani::parse_ani(&bytes)
            .ok()
            .and_then(|p| p.frames.first().map(|f| f.width))
            .unwrap_or(32);
        let (hot_x_px, hot_y_px) = entry.hotspot.to_px(primary_size);
        let rewritten =
            crate::cursor::rewrite_ani_with_hotspot(&bytes, (hot_x_px as u16, hot_y_px as u16))?;
        return Ok(BuiltRole {
            bytes: rewritten,
            definition: CursorDefinition {
                file: format!("cursors/{}.ani", entry.role),
                hotspot: entry.hotspot,
                resize_method: entry.resample.clone(),
                size_overrides: None,
            },
        });
    }

    let resample = match entry.resample.as_str() {
        "auto" => ResizeMethod::Lanczos,
        other => ResizeMethod::from_str(other),
    };

    // primary_size を PNG ヘッダから取得して ratio→px 変換する
    let primary_size = image::load_from_memory(&entry.png_bytes)
        .map(|img| img.width())
        .unwrap_or(32);
    let (hot_x_px, hot_y_px) = entry.hotspot.to_px(primary_size);

    // sized_overrides を PNG バイト列マップとサイズ別ホットスポット px マップに分解する。
    // payload.hotspot が Some の場合はそのサイズ専用 hotspot px を計算して渡す。
    // None のサイズは親の (hot_x_px, hot_y_px) を build_cur_from_png 側でスケール適用。
    let (sized_png_map, sized_hotspot_map) = match entry.sized_overrides.as_ref() {
        None => (None, None),
        Some(overrides) => {
            let mut png_map: HashMap<u32, Vec<u8>> = HashMap::new();
            let mut hot_map: HashMap<u32, (u32, u32)> = HashMap::new();
            for (size, payload) in overrides {
                png_map.insert(*size, payload.png_bytes.clone());
                // payload.hotspot が Some ならそのサイズ独自の hotspot を px 変換して記録
                if let Some(override_hot) = payload.hotspot {
                    hot_map.insert(*size, override_hot.to_px(*size));
                }
            }
            let hot_map_opt = if hot_map.is_empty() {
                None
            } else {
                Some(hot_map)
            };
            (Some(png_map), hot_map_opt)
        }
    };

    let bin = build_cur_from_png(
        &entry.png_bytes,
        hot_x_px,
        hot_y_px,
        resample,
        sized_png_map.as_ref(),
        sized_hotspot_map.as_ref(),
    )?;

    Ok(BuiltRole {
        bytes: bin,
        definition: CursorDefinition {
            file: format!("cursors/{}.cur", entry.role),
            hotspot: entry.hotspot,
            resize_method: entry.resample.clone(),
            size_overrides: None,
        },
    })
}
