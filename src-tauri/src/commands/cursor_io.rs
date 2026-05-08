//! `.cur` / `.ico` / `.ani` 単一ファイルの読み込み IPC。
//!
//! クリエイター画面の「既存カーソルを取り込む」用途で使う。
//! バイナリパースは `crate::cursor` 側、ここはそれをラップして PNG + メタ情報を返すだけ。

use crate::cursor::{parse_ani, parse_ico_cur, pick_largest_as_png};
use crate::errors::AppError;
use serde::Serialize;

/// `.cur` / `.ico` を取り込んだ結果のフロント返却型。
///
/// `pngBytes` が最大解像度のラスタ画像 (RGBA PNG)。
/// `availableSizes` は元ファイルに含まれていた全エントリの幅 (= 高さ前提)。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCursorFile {
    pub is_cur: bool,
    pub width: u32,
    pub height: u32,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    pub png_bytes: Vec<u8>,
    pub available_sizes: Vec<u32>,
}

/// `.ico` / `.cur` ファイルを読み込み、最大解像度を PNG 化して返す。
///
/// クリエイターモードで「既存の Windows カーソルを取り込む」用途。
/// 全解像度を返すと IPC ペイロードが膨らむため、最大サイズのみを返却する設計。
#[tauri::command]
pub fn import_cursor_file(path: String) -> Result<ImportedCursorFile, AppError> {
    let bytes = std::fs::read(&path).map_err(|e| {
        AppError::ImageProcessing(format!(
            "ファイル読込失敗 ({}): {}",
            crate::logging::redact_path(std::path::Path::new(&path)),
            e
        ))
    })?;
    let parsed = parse_ico_cur(&bytes)?;
    let available_sizes = parsed.entries.iter().map(|e| e.width).collect();
    let (largest, png_bytes) = pick_largest_as_png(&parsed)?;
    tracing::info!(
        "import_cursor_file: is_cur={} entries={} largest={}x{}",
        parsed.is_cur,
        parsed.entries.len(),
        largest.width,
        largest.height
    );
    Ok(ImportedCursorFile {
        is_cur: parsed.is_cur,
        width: largest.width,
        height: largest.height,
        hotspot_x: largest.hotspot_x,
        hotspot_y: largest.hotspot_y,
        png_bytes,
        available_sizes,
    })
}

/// `.ani` ファイル検査結果。クリエイター UI のプレビュー / 情報表示用。
///
/// `framePngs` は再生順 (= sequence インデックスを展開した順) ではなく、
/// 格納順 (フレーム 0..num_frames-1) に並ぶ。UI 側で `sequence` と
/// `perStepDurationsMs` を見ながら setInterval / requestAnimationFrame で
/// アニメーション再生する。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AniInspection {
    pub num_frames: u32,
    pub num_steps: u32,
    pub default_rate_jiffies: u32,
    pub per_step_durations_ms: Vec<u32>,
    pub sequence: Vec<u32>,
    pub total_duration_ms: u64,
    /// 各フレームの最大解像度 PNG バイト列
    pub frame_pngs: Vec<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
}

/// `.ani` ファイルを読み込み、フレームごとの PNG とアニメーション情報を返す。
#[tauri::command]
pub fn inspect_ani_file(path: String) -> Result<AniInspection, AppError> {
    let bytes = std::fs::read(&path).map_err(|e| {
        AppError::ImageProcessing(format!(
            "ファイル読込失敗 ({}): {}",
            crate::logging::redact_path(std::path::Path::new(&path)),
            e
        ))
    })?;
    let parsed = parse_ani(&bytes)?;

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
        .map_err(|e| AppError::ImageProcessing(format!("PNG エンコード失敗: {}", e)))?;
        frame_pngs.push(png);
    }

    let per_step_durations_ms: Vec<u32> = parsed
        .per_step_rate_jiffies
        .iter()
        .map(|j| ((*j as u64 * 1000) / 60) as u32)
        .collect();
    let total_duration_ms = parsed.total_duration_ms();

    let (width, height, hotspot_x, hotspot_y) = parsed
        .frames
        .first()
        .map(|f| (f.width, f.height, f.hotspot_x, f.hotspot_y))
        .unwrap_or((0, 0, 0, 0));

    tracing::info!(
        "inspect_ani_file: frames={} steps={} total={}ms",
        parsed.num_frames,
        parsed.num_steps,
        total_duration_ms
    );

    Ok(AniInspection {
        num_frames: parsed.num_frames,
        num_steps: parsed.num_steps,
        default_rate_jiffies: parsed.default_rate_jiffies,
        per_step_durations_ms,
        sequence: parsed.sequence,
        total_duration_ms,
        frame_pngs,
        width,
        height,
        hotspot_x,
        hotspot_y,
    })
}
