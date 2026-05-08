//! `.ico` / `.cur` インポーター — 既存ファイルから複数解像度を抽出。
//!
//! クリエイターモードで「既存カーソルを取り込む」ユースケース向け。
//!
//! サポート範囲:
//!   - PNG エンコード済みエントリ (256px の Vista 以降のフォーマット)
//!   - 32bpp BMP DIB エントリ (BITMAPINFOHEADER + XOR mask)
//!   - AND mask は不透明度を補強する目的でのみ参照 (32bpp ではアルファを優先)
//!
//! 非対応: 1/4/8/24bpp の旧式パレットエントリ (生成側でも使われていない)

use crate::errors::{AppError, AppResult};
use image::RgbaImage;

/// 解析済みカーソルファイル全体の表現
#[derive(Debug, Clone)]
pub struct ParsedIcoCur {
    /// true = .cur (Type 2) / false = .ico (Type 1)
    pub is_cur: bool,
    /// サイズの異なるエントリ群 (元ファイル順)
    pub entries: Vec<ParsedIcoCurEntry>,
}

/// 解析済み 1 エントリ
#[derive(Debug, Clone)]
pub struct ParsedIcoCurEntry {
    pub width: u32,
    pub height: u32,
    /// CUR 形式時のホットスポット X (ICO のときは 0)
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    pub image: RgbaImage,
}

const PNG_MAGIC: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// `.ico` / `.cur` のバイナリを解析し、含まれる全解像度を抽出する。
///
/// 失敗ケース:
///   - 先頭 6 バイト未満 / Type が 1/2 以外
///   - エントリオフセットが配列外
///   - 個別エントリのデコードに失敗 (個別 Err は配列から除外せず即返却)
pub fn parse_ico_cur(bytes: &[u8]) -> AppResult<ParsedIcoCur> {
    if bytes.len() < 6 {
        return Err(AppError::ImageProcessing(
            "ICO/CUR ヘッダーに必要な 6 バイトがありません".to_string(),
        ));
    }
    let reserved = u16::from_le_bytes([bytes[0], bytes[1]]);
    let kind = u16::from_le_bytes([bytes[2], bytes[3]]);
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    if reserved != 0 {
        return Err(AppError::ImageProcessing(format!(
            "ICO/CUR ヘッダーの reserved 値が 0 ではありません: {}",
            reserved
        )));
    }
    let is_cur = match kind {
        1 => false,
        2 => true,
        other => {
            return Err(AppError::ImageProcessing(format!(
                "未対応のファイル種別: {}",
                other
            )));
        }
    };
    if count == 0 {
        return Err(AppError::ImageProcessing("エントリ数が 0 です".to_string()));
    }
    let dir_size = 6 + count * 16;
    if bytes.len() < dir_size {
        return Err(AppError::ImageProcessing(
            "ディレクトリエントリが切り詰められています".to_string(),
        ));
    }

    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let off = 6 + i * 16;
        let raw_w = bytes[off] as u32;
        let raw_h = bytes[off + 1] as u32;
        let width = if raw_w == 0 { 256 } else { raw_w };
        let height = if raw_h == 0 { 256 } else { raw_h };
        // ICO の planes / bit_count を兼ねるフィールドを CUR ではホットスポットとして読む
        let f0 = u16::from_le_bytes([bytes[off + 4], bytes[off + 5]]);
        let f1 = u16::from_le_bytes([bytes[off + 6], bytes[off + 7]]);
        let data_size = u32::from_le_bytes([
            bytes[off + 8],
            bytes[off + 9],
            bytes[off + 10],
            bytes[off + 11],
        ]) as usize;
        let data_off = u32::from_le_bytes([
            bytes[off + 12],
            bytes[off + 13],
            bytes[off + 14],
            bytes[off + 15],
        ]) as usize;
        if data_off
            .checked_add(data_size)
            .is_none_or(|end| end > bytes.len())
        {
            return Err(AppError::ImageProcessing(format!(
                "エントリ {} のデータ範囲がファイル外です",
                i
            )));
        }
        let payload = &bytes[data_off..data_off + data_size];
        let image = decode_ico_cur_entry(payload, width, height)?;
        entries.push(ParsedIcoCurEntry {
            width,
            height,
            hotspot_x: if is_cur { f0 as u32 } else { 0 },
            hotspot_y: if is_cur { f1 as u32 } else { 0 },
            image,
        });
    }

    Ok(ParsedIcoCur { is_cur, entries })
}

fn decode_ico_cur_entry(data: &[u8], expected_w: u32, expected_h: u32) -> AppResult<RgbaImage> {
    if data.len() >= PNG_MAGIC.len() && &data[..PNG_MAGIC.len()] == PNG_MAGIC {
        let dyn_img = image::load_from_memory_with_format(data, image::ImageFormat::Png)
            .map_err(|e| AppError::ImageProcessing(format!("PNG エントリのデコード失敗: {}", e)))?;
        return Ok(dyn_img.to_rgba8());
    }
    decode_dib_entry(data, expected_w, expected_h)
}

/// BITMAPINFOHEADER + XOR/AND マスクを 32bpp 想定で RGBA にデコードする。
/// 非 32bpp は明示エラー。
fn decode_dib_entry(data: &[u8], expected_w: u32, expected_h: u32) -> AppResult<RgbaImage> {
    if data.len() < 40 {
        return Err(AppError::ImageProcessing(
            "BITMAPINFOHEADER が不足".to_string(),
        ));
    }
    let header_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if header_size < 40 {
        return Err(AppError::ImageProcessing(format!(
            "想定外の BITMAPINFOHEADER サイズ: {}",
            header_size
        )));
    }
    let dib_w = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let dib_h = i32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    // ICO/CUR の DIB は通常 height = 実高さ × 2 (XOR + AND)
    let bit_count = u16::from_le_bytes([data[14], data[15]]);
    if bit_count != 32 {
        return Err(AppError::ImageProcessing(format!(
            "未対応の bpp ({} bit)。32bpp の .ico/.cur のみ対応",
            bit_count
        )));
    }
    let width = dib_w as u32;
    let real_height = dib_h.unsigned_abs() / 2;
    if width != expected_w || real_height != expected_h {
        return Err(AppError::ImageProcessing(format!(
            "ヘッダー寸法の不一致: ディレクトリ={}x{}, DIB={}x{}",
            expected_w, expected_h, width, real_height
        )));
    }

    let header_size = header_size as usize;
    let pixel_off = header_size; // パレットなし (32bpp)
    let row_bytes = (width * 4) as usize;
    let xor_size = row_bytes * real_height as usize;
    if pixel_off + xor_size > data.len() {
        return Err(AppError::ImageProcessing(
            "XOR マスクが切り詰められています".to_string(),
        ));
    }
    let xor = &data[pixel_off..pixel_off + xor_size];

    // BMP は通常ボトムアップ (dib_h > 0)。トップダウンは dib_h < 0。
    let bottom_up = dib_h > 0;

    let mut img = RgbaImage::new(width, real_height);
    for y in 0..real_height {
        let src_row = if bottom_up { real_height - 1 - y } else { y };
        let row_off = src_row as usize * row_bytes;
        for x in 0..width {
            let off = row_off + x as usize * 4;
            let b = xor[off];
            let g = xor[off + 1];
            let r = xor[off + 2];
            let a = xor[off + 3];
            img.put_pixel(x, y, image::Rgba([r, g, b, a]));
        }
    }
    Ok(img)
}

/// 解析済みエントリから「最も大きいサイズ」を選んで PNG 化して返すヘルパー。
/// クリエイター UI が「インポート画像」として扱える単一画像を提供する用途。
pub fn pick_largest_as_png(parsed: &ParsedIcoCur) -> AppResult<(ParsedIcoCurEntry, Vec<u8>)> {
    let largest = parsed
        .entries
        .iter()
        .max_by_key(|e| e.width * e.height)
        .ok_or_else(|| AppError::ImageProcessing("エントリがありません".to_string()))?
        .clone();
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);
    image::ImageEncoder::write_image(
        encoder,
        largest.image.as_raw(),
        largest.image.width(),
        largest.image.height(),
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| AppError::ImageProcessing(format!("PNG エンコード失敗: {}", e)))?;
    Ok((largest, png))
}

#[cfg(test)]
mod tests {
    use super::super::cur_build::generate_cur_binary;
    use super::*;

    /// generate_cur_binary で書き出した CUR を parse_ico_cur が読み戻せることを確認する。
    #[test]
    fn parse_ico_cur_roundtrip_png_entry() {
        let img32 = RgbaImage::from_pixel(32, 32, image::Rgba([10, 20, 30, 255]));
        let img64 = RgbaImage::from_pixel(64, 64, image::Rgba([200, 100, 50, 200]));
        let cur = generate_cur_binary(&[(img32.clone(), 4, 5), (img64.clone(), 8, 10)]).unwrap();

        let parsed = parse_ico_cur(&cur).expect("parse");
        assert!(parsed.is_cur);
        assert_eq!(parsed.entries.len(), 2);

        let e0 = &parsed.entries[0];
        assert_eq!((e0.width, e0.height), (32, 32));
        assert_eq!((e0.hotspot_x, e0.hotspot_y), (4, 5));
        assert_eq!(e0.image.get_pixel(0, 0), &image::Rgba([10, 20, 30, 255]));

        let e1 = &parsed.entries[1];
        assert_eq!((e1.width, e1.height), (64, 64));
        assert_eq!((e1.hotspot_x, e1.hotspot_y), (8, 10));
    }

    #[test]
    fn parse_ico_cur_rejects_too_short_header() {
        let err = parse_ico_cur(&[0u8; 4]).unwrap_err();
        assert!(matches!(err, AppError::ImageProcessing(_)));
    }

    #[test]
    fn parse_ico_cur_rejects_unknown_kind() {
        // type=99 = 不明
        let mut bytes = vec![0u8, 0, 99, 0, 1, 0];
        // 1 個のエントリ用ダミー (実際には到達しない)
        bytes.extend_from_slice(&[0u8; 16]);
        let err = parse_ico_cur(&bytes).unwrap_err();
        assert!(matches!(err, AppError::ImageProcessing(_)));
    }

    /// pick_largest_as_png は最大解像度を返すべき
    #[test]
    fn pick_largest_returns_biggest_entry() {
        let i32 = RgbaImage::from_pixel(32, 32, image::Rgba([0, 0, 0, 255]));
        let i256 = RgbaImage::from_pixel(256, 256, image::Rgba([0, 0, 0, 255]));
        let cur = generate_cur_binary(&[(i32, 0, 0), (i256, 100, 110)]).unwrap();

        let parsed = parse_ico_cur(&cur).unwrap();
        let (largest, png) = pick_largest_as_png(&parsed).unwrap();
        assert_eq!(largest.width, 256);
        assert_eq!((largest.hotspot_x, largest.hotspot_y), (100, 110));
        assert!(png.starts_with(PNG_MAGIC));
    }
}
