//! `.ani` を新ホットスポットで書き出すコマンド。
//! 一括取り込みフロー (Creator) からも、`.ani として書き出し` UI からも同じコマンドを呼ぶ。

use crate::cursor::{rewrite_ani_to_path, RewriteStats};
use crate::errors::AppError;
use std::path::PathBuf;

#[tauri::command]
pub fn export_ani_with_hotspot(
    input_path: String,
    output_path: String,
    hotspot_x: u16,
    hotspot_y: u16,
) -> Result<RewriteStats, AppError> {
    let input = PathBuf::from(&input_path);
    let output = PathBuf::from(&output_path);
    if !input.is_file() {
        return Err(AppError::ImageProcessing(format!(
            "入力ファイルが存在しません: {}",
            crate::logging::redact_path(&input)
        )));
    }
    if let Some(parent) = output.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::ImageProcessing(format!(
                    "出力ディレクトリ作成失敗 ({}): {}",
                    crate::logging::redact_path(parent),
                    e
                ))
            })?;
        }
    }
    let stats = rewrite_ani_to_path(&input, &output, (hotspot_x, hotspot_y))?;
    tracing::info!(
        "export_ani_with_hotspot: bytes_written={} legacy_normalized={}",
        stats.bytes_written,
        stats.was_legacy_normalized
    );
    Ok(stats)
}
