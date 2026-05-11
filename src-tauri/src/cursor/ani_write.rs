//! `.ani` のバイト単位パススルー書き換え。
//!
//! - AF_ICON 形式: 元バイト列をコピーし、各 icon chunk 内の CUR ICONDIRENTRY の
//!   ホットスポット 4 バイト (offset 4..8 within each 16-byte entry: X u16 LE + Y u16 LE)
//!   のみ上書きする。画像データ部 (PNG / BMP DIB) には触れない。
//! - raw DIB 形式: 各 DIB を CUR (ICONDIR + ICONDIRENTRY + DIB) でラップし、
//!   anih.flags に AF_ICON を立てて再パックする。DIB バイトはコピーのみ。
//!   (Task 4 で実装予定)
//!
//! rate / seq / LIST INFO チャンクは常に元バイトのまま。

use super::ani::{parse_ani, AniFrameFormat, AniFrameInfo};
use crate::errors::{AppError, AppResult};
use std::path::Path;

/// `.ani` 書き換え結果の統計。
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteStats {
    pub bytes_written: u64,
    pub was_legacy_normalized: bool,
}

/// 既存 .ani バイト列を読み込み、ホットスポットだけを書き換えた新しいバイト列を返す。
pub fn rewrite_ani_with_hotspot(bytes: &[u8], hotspot: (u16, u16)) -> AppResult<Vec<u8>> {
    let parsed = parse_ani(bytes)?;
    if parsed.is_legacy_raw_dib {
        return Err(AppError::ImageProcessing(
            "raw DIB の正規化は Task 4 で実装予定".to_string(),
        ));
    }
    rewrite_af_icon(bytes, &parsed.frame_infos, hotspot)
}

/// I/O 付きラッパ。
pub fn rewrite_ani_to_path(
    input_path: &Path,
    output_path: &Path,
    hotspot: (u16, u16),
) -> AppResult<RewriteStats> {
    let bytes = std::fs::read(input_path).map_err(|e| {
        AppError::ImageProcessing(format!(
            "ファイル読込失敗 ({}): {}",
            crate::logging::redact_path(input_path),
            e
        ))
    })?;
    let parsed = parse_ani(&bytes)?;
    let was_legacy_normalized = parsed.is_legacy_raw_dib;
    let new_bytes = rewrite_ani_with_hotspot(&bytes, hotspot)?;
    std::fs::write(output_path, &new_bytes).map_err(|e| {
        AppError::ImageProcessing(format!(
            "ファイル書込失敗 ({}): {}",
            crate::logging::redact_path(output_path),
            e
        ))
    })?;
    Ok(RewriteStats {
        bytes_written: new_bytes.len() as u64,
        was_legacy_normalized,
    })
}

/// AF_ICON ケース: 元バイトをコピーして CUR ICONDIRENTRY のホットスポットだけ書き換える。
fn rewrite_af_icon(
    bytes: &[u8],
    frame_infos: &[AniFrameInfo],
    hotspot: (u16, u16),
) -> AppResult<Vec<u8>> {
    let mut out = bytes.to_vec();
    for info in frame_infos {
        if !matches!(info.format, AniFrameFormat::AfIcon) {
            continue;
        }
        patch_cur_hotspot(&mut out[info.raw_data_range.clone()], hotspot)?;
    }
    Ok(out)
}

/// CUR バイト列 (ICONDIR + ICONDIRENTRY[] + image data) のディレクトリエントリを巡回し、
/// 各エントリの bytes[4..6] (hotspot X u16 LE) と bytes[6..8] (hotspot Y u16 LE) を上書きする。
fn patch_cur_hotspot(cur: &mut [u8], hotspot: (u16, u16)) -> AppResult<()> {
    if cur.len() < 6 {
        return Err(AppError::ImageProcessing(
            "CUR ヘッダ (6 bytes) 未満".to_string(),
        ));
    }
    let kind = u16::from_le_bytes([cur[2], cur[3]]);
    if kind != 2 {
        return Err(AppError::ImageProcessing(format!(
            "CUR (kind=2) ではない: kind={}",
            kind
        )));
    }
    let count = u16::from_le_bytes([cur[4], cur[5]]) as usize;
    let dir_end = 6 + count * 16;
    if cur.len() < dir_end {
        return Err(AppError::ImageProcessing(
            "ICONDIRENTRY 配列が切り詰め".to_string(),
        ));
    }
    let (hx, hy) = hotspot;
    for i in 0..count {
        let off = 6 + i * 16;
        cur[off + 4] = hx as u8;
        cur[off + 5] = (hx >> 8) as u8;
        cur[off + 6] = hy as u8;
        cur[off + 7] = (hy >> 8) as u8;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::ani::parse_ani;
    use super::super::cur_build::generate_cur_binary;
    use super::*;
    use image::RgbaImage;

    fn build_af_icon_two_frame_ani() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let img1 = RgbaImage::from_pixel(32, 32, image::Rgba([255, 0, 0, 255]));
        let img2 = RgbaImage::from_pixel(32, 32, image::Rgba([0, 255, 0, 255]));
        let cur1 = generate_cur_binary(&[(img1, 1, 2)]).unwrap();
        let cur2 = generate_cur_binary(&[(img2, 3, 4)]).unwrap();

        let mut ani = Vec::new();
        ani.extend_from_slice(b"RIFF");
        let bp = ani.len();
        ani.extend_from_slice(&0u32.to_le_bytes());
        ani.extend_from_slice(b"ACON");
        ani.extend_from_slice(b"anih");
        ani.extend_from_slice(&36u32.to_le_bytes());
        let mut header = vec![0u8; 36];
        header[0..4].copy_from_slice(&36u32.to_le_bytes());
        header[4..8].copy_from_slice(&2u32.to_le_bytes());
        header[8..12].copy_from_slice(&2u32.to_le_bytes());
        header[28..32].copy_from_slice(&6u32.to_le_bytes());
        header[32..36].copy_from_slice(&0x01u32.to_le_bytes()); // AF_ICON
        ani.extend_from_slice(&header);
        ani.extend_from_slice(b"LIST");
        let ls = 4 + (8 + cur1.len()) + (8 + cur2.len());
        ani.extend_from_slice(&(ls as u32).to_le_bytes());
        ani.extend_from_slice(b"fram");
        ani.extend_from_slice(b"icon");
        ani.extend_from_slice(&(cur1.len() as u32).to_le_bytes());
        ani.extend_from_slice(&cur1);
        ani.extend_from_slice(b"icon");
        ani.extend_from_slice(&(cur2.len() as u32).to_le_bytes());
        ani.extend_from_slice(&cur2);
        let body = (ani.len() - 8) as u32;
        ani[bp..bp + 4].copy_from_slice(&body.to_le_bytes());
        (ani, cur1, cur2)
    }

    fn cur_image_data_range(cur: &[u8]) -> std::ops::Range<usize> {
        let count = u16::from_le_bytes([cur[4], cur[5]]) as usize;
        let off = 6 + count * 16;
        off..cur.len()
    }

    #[test]
    fn rewrite_updates_hotspot_in_all_entries() {
        let (ani, _, _) = build_af_icon_two_frame_ani();
        let out = rewrite_ani_with_hotspot(&ani, (7, 3)).expect("rewrite");
        let reparsed = parse_ani(&out).expect("reparse");
        for f in &reparsed.frames {
            assert_eq!(f.hotspot_x, 7);
            assert_eq!(f.hotspot_y, 3);
        }
    }

    #[test]
    fn rewrite_preserves_frame_image_bytes() {
        let (ani, _cur1, _cur2) = build_af_icon_two_frame_ani();
        let out = rewrite_ani_with_hotspot(&ani, (9, 9)).expect("rewrite");

        let parsed_in = parse_ani(&ani).unwrap();
        let parsed_out = parse_ani(&out).unwrap();
        assert_eq!(parsed_in.frame_infos.len(), parsed_out.frame_infos.len());
        for (a, b) in parsed_in
            .frame_infos
            .iter()
            .zip(parsed_out.frame_infos.iter())
        {
            let cur_in = &ani[a.raw_data_range.clone()];
            let cur_out = &out[b.raw_data_range.clone()];
            let img_range_in = cur_image_data_range(cur_in);
            let img_range_out = cur_image_data_range(cur_out);
            assert_eq!(&cur_in[img_range_in], &cur_out[img_range_out]);
        }
    }

    #[test]
    fn rewrite_preserves_anih_rate_seq_durations() {
        let (ani, _, _) = build_af_icon_two_frame_ani();
        let out = rewrite_ani_with_hotspot(&ani, (1, 1)).expect("rewrite");
        let a = parse_ani(&ani).unwrap();
        let b = parse_ani(&out).unwrap();
        assert_eq!(a.num_frames, b.num_frames);
        assert_eq!(a.num_steps, b.num_steps);
        assert_eq!(a.default_rate_jiffies, b.default_rate_jiffies);
        assert_eq!(a.per_step_rate_jiffies, b.per_step_rate_jiffies);
        assert_eq!(a.sequence, b.sequence);
        assert_eq!(a.total_duration_ms(), b.total_duration_ms());
    }
}
