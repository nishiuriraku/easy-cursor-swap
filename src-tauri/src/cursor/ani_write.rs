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
        rewrite_raw_dib_to_af_icon(bytes, &parsed.frame_infos, hotspot)
    } else {
        rewrite_af_icon(bytes, &parsed.frame_infos, hotspot)
    }
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

/// raw DIB ケース: 各 DIB を CUR (ICONDIR + 1 ICONDIRENTRY + DIB) でラップし、
/// anih.flags に AF_ICON を立てて再パック。rate / seq / LIST INFO は元バイトをコピー。
fn rewrite_raw_dib_to_af_icon(
    bytes: &[u8],
    frame_infos: &[AniFrameInfo],
    hotspot: (u16, u16),
) -> AppResult<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() + frame_infos.len() * 32);
    out.extend_from_slice(&bytes[0..12]);
    let body_size_pos = 4usize;

    let mut pos = 12usize;
    let end = bytes.len();
    let icon_ranges: Vec<std::ops::Range<usize>> = frame_infos
        .iter()
        .map(|i| i.raw_data_range.clone())
        .collect();

    while pos + 8 <= end {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        let data_start = pos + 8;
        let data_end = data_start
            .checked_add(size)
            .filter(|&e| e <= end)
            .ok_or_else(|| AppError::ImageProcessing("rewrite: チャンクが切り詰め".into()))?;

        if id == b"anih" {
            let mut hdr = bytes[data_start..data_end].to_vec();
            if hdr.len() >= 36 {
                let mut flags = u32::from_le_bytes([hdr[32], hdr[33], hdr[34], hdr[35]]);
                flags |= 0x01;
                hdr[32..36].copy_from_slice(&flags.to_le_bytes());
            }
            out.extend_from_slice(b"anih");
            out.extend_from_slice(&(hdr.len() as u32).to_le_bytes());
            out.extend_from_slice(&hdr);
            if hdr.len() & 1 == 1 {
                out.push(0);
            }
        } else if id == b"LIST" && bytes.get(data_start..data_start + 4) == Some(b"fram".as_ref()) {
            let mut list_body: Vec<u8> = Vec::new();
            list_body.extend_from_slice(b"fram");

            let mut inner_pos = data_start + 4;
            while inner_pos + 8 <= data_end {
                let iid = &bytes[inner_pos..inner_pos + 4];
                let isize_u = u32::from_le_bytes([
                    bytes[inner_pos + 4],
                    bytes[inner_pos + 5],
                    bytes[inner_pos + 6],
                    bytes[inner_pos + 7],
                ]) as usize;
                let istart = inner_pos + 8;
                let iend = istart + isize_u;
                if iid == b"icon"
                    && icon_ranges
                        .iter()
                        .any(|r| r.start == istart && r.end == iend)
                {
                    let wrapped = wrap_dib_as_cur(&bytes[istart..iend], hotspot)?;
                    list_body.extend_from_slice(b"icon");
                    list_body.extend_from_slice(&(wrapped.len() as u32).to_le_bytes());
                    list_body.extend_from_slice(&wrapped);
                    if wrapped.len() & 1 == 1 {
                        list_body.push(0);
                    }
                } else {
                    list_body.extend_from_slice(&bytes[inner_pos..iend + (isize_u & 1)]);
                }
                inner_pos = iend + (isize_u & 1);
            }

            out.extend_from_slice(b"LIST");
            out.extend_from_slice(&(list_body.len() as u32).to_le_bytes());
            out.extend_from_slice(&list_body);
            if list_body.len() & 1 == 1 {
                out.push(0);
            }
        } else {
            let total = (data_end - pos) + (size & 1);
            out.extend_from_slice(&bytes[pos..pos + total]);
        }

        pos = data_end + (size & 1);
    }

    let body = (out.len() - 8) as u32;
    out[body_size_pos..body_size_pos + 4].copy_from_slice(&body.to_le_bytes());

    Ok(out)
}

/// 1 枚の DIB バイト列を CUR (Type 2、エントリ 1 個) でラップする。
/// 画像データは DIB バイト列をそのままコピー (再エンコードしない)。
fn wrap_dib_as_cur(dib: &[u8], hotspot: (u16, u16)) -> AppResult<Vec<u8>> {
    if dib.len() < 40 {
        return Err(AppError::ImageProcessing(
            "wrap_dib_as_cur: DIB が短すぎる".into(),
        ));
    }
    let width = i32::from_le_bytes([dib[4], dib[5], dib[6], dib[7]]);
    let height_raw = i32::from_le_bytes([dib[8], dib[9], dib[10], dib[11]]);
    let _bit_count = u16::from_le_bytes([dib[14], dib[15]]);

    // CUR ICONDIRENTRY の幅・高さは u8 (0 = 256 を意味する)。
    // CUR 内 DIB の biHeight は XOR マスク + AND マスクの合計行数 (表示高さの 2 倍) なので
    // ICONDIRENTRY に格納する表示高さは biHeight / 2 とする。
    let w_u: u8 = if width <= 0 || width >= 256 {
        0
    } else {
        width as u8
    };
    let h_abs = if height_raw < 0 {
        -height_raw
    } else {
        height_raw
    };
    let h_disp = h_abs / 2; // biHeight は 2× なので表示高さに戻す
    let h_u: u8 = if h_disp <= 0 || h_disp >= 256 {
        0
    } else {
        h_disp as u8
    };

    let mut cur: Vec<u8> = Vec::with_capacity(6 + 16 + dib.len());
    // ICONDIR
    cur.extend_from_slice(&0u16.to_le_bytes()); // reserved
    cur.extend_from_slice(&2u16.to_le_bytes()); // type = 2 (CUR)
    cur.extend_from_slice(&1u16.to_le_bytes()); // count = 1
                                                // ICONDIRENTRY (16 bytes)
    cur.push(w_u);
    cur.push(h_u);
    cur.push(0); // bColorCount
    cur.push(0); // bReserved
    cur.extend_from_slice(&hotspot.0.to_le_bytes()); // hotspot X
    cur.extend_from_slice(&hotspot.1.to_le_bytes()); // hotspot Y
    cur.extend_from_slice(&(dib.len() as u32).to_le_bytes()); // dwBytesInRes
    cur.extend_from_slice(&((6 + 16) as u32).to_le_bytes()); // dwImageOffset
    cur.extend_from_slice(dib);

    Ok(cur)
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

    fn build_raw_dib_one_frame_ani() -> (Vec<u8>, Vec<u8>) {
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&8i32.to_le_bytes());
        dib.extend_from_slice(&8i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&32u16.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&256u32.to_le_bytes());
        dib.extend_from_slice(&[0u8; 16]);
        for i in 0..64 {
            let v = (i % 256) as u8;
            dib.extend_from_slice(&[v, v, v, 0xFF]);
        }

        let mut ani = Vec::new();
        ani.extend_from_slice(b"RIFF");
        let bp = ani.len();
        ani.extend_from_slice(&0u32.to_le_bytes());
        ani.extend_from_slice(b"ACON");
        ani.extend_from_slice(b"anih");
        ani.extend_from_slice(&36u32.to_le_bytes());
        let mut header = vec![0u8; 36];
        header[0..4].copy_from_slice(&36u32.to_le_bytes());
        header[4..8].copy_from_slice(&1u32.to_le_bytes());
        header[8..12].copy_from_slice(&1u32.to_le_bytes());
        header[12..16].copy_from_slice(&8u32.to_le_bytes());
        header[16..20].copy_from_slice(&8u32.to_le_bytes());
        header[20..24].copy_from_slice(&32u32.to_le_bytes());
        header[28..32].copy_from_slice(&3u32.to_le_bytes());
        header[32..36].copy_from_slice(&0u32.to_le_bytes()); // flags=0 (raw DIB)
        ani.extend_from_slice(&header);
        ani.extend_from_slice(b"LIST");
        let ls = 4 + 8 + dib.len();
        ani.extend_from_slice(&(ls as u32).to_le_bytes());
        ani.extend_from_slice(b"fram");
        ani.extend_from_slice(b"icon");
        ani.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        ani.extend_from_slice(&dib);
        let body = (ani.len() - 8) as u32;
        ani[bp..bp + 4].copy_from_slice(&body.to_le_bytes());

        (ani, dib)
    }

    #[test]
    fn rewrite_normalizes_raw_dib_to_af_icon() {
        let (ani, dib) = build_raw_dib_one_frame_ani();
        let out = rewrite_ani_with_hotspot(&ani, (5, 5)).expect("rewrite legacy");
        let reparsed = parse_ani(&out).expect("reparse");
        assert!(!reparsed.is_legacy_raw_dib);
        assert_eq!(reparsed.frames.len(), 1);
        assert_eq!(reparsed.frames[0].hotspot_x, 5);
        assert_eq!(reparsed.frames[0].hotspot_y, 5);

        assert!(matches!(
            reparsed.frame_infos[0].format,
            super::super::ani::AniFrameFormat::AfIcon
        ));

        let cur = &out[reparsed.frame_infos[0].raw_data_range.clone()];
        let count = u16::from_le_bytes([cur[4], cur[5]]) as usize;
        assert_eq!(count, 1);
        let image_off = 6 + count * 16;
        let dw_offset =
            u32::from_le_bytes([cur[6 + 12], cur[6 + 13], cur[6 + 14], cur[6 + 15]]) as usize;
        assert_eq!(dw_offset, image_off);
        assert_eq!(&cur[image_off..], &dib[..]);
    }

    #[test]
    fn rewrite_preserves_dib_pixel_bytes_after_normalization() {
        let (ani, dib) = build_raw_dib_one_frame_ani();
        let out = rewrite_ani_with_hotspot(&ani, (0, 0)).expect("rewrite");
        let reparsed = parse_ani(&out).unwrap();
        let cur = &out[reparsed.frame_infos[0].raw_data_range.clone()];
        let count = u16::from_le_bytes([cur[4], cur[5]]) as usize;
        let image_off = 6 + count * 16;
        assert_eq!(&cur[image_off..], &dib[..]);
    }
}
