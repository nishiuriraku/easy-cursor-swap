//! `.ico` / `.cur` インポーター — 既存ファイルから複数解像度を抽出。
//!
//! クリエイターモードで「既存カーソルを取り込む」ユースケース、および
//! ライブラリの「現在のスキーム」プレビューで使用。
//!
//! サポート範囲:
//!   - PNG エンコード済みエントリ (256px の Vista 以降のフォーマット)
//!   - 32bpp BMP DIB エントリ (BITMAPINFOHEADER + XOR mask + α は XOR から)
//!   - 24bpp BMP DIB エントリ (RGB + AND mask による透明化)
//!   - 8/4/1bpp パレットエントリ (BITMAPINFOHEADER + 色テーブル + AND mask)
//!
//! 8bpp パレットは Aero などの Windows 標準スキームで現役なので必須サポート。
//! AND mask は 1bpp 透明マスクで、各非 32bpp 形式に対する透明度の出所。

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

/// BITMAPINFOHEADER + XOR/AND マスクを 1/4/8/24/32bpp で RGBA にデコードする。
///
/// Windows 標準のレガシーカーソル (Aero など) は 8bpp パレット形式で配布されている
/// ものが多く、ここで弾くとライブラリの「現在のスキーム」プレビューが全滅する。
/// 1/4/8bpp はパレット参照、24/32bpp は直接ピクセル。透明度は 32bpp のときのみ
/// XOR の α を使用し、それ以外は AND mask (1bpp) を参照する。
fn decode_dib_entry(data: &[u8], expected_w: u32, expected_h: u32) -> AppResult<RgbaImage> {
    if data.len() < 40 {
        return Err(AppError::ImageProcessing(
            "BITMAPINFOHEADER が不足".to_string(),
        ));
    }
    let header_size_raw = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if header_size_raw < 40 {
        return Err(AppError::ImageProcessing(format!(
            "想定外の BITMAPINFOHEADER サイズ: {}",
            header_size_raw
        )));
    }
    let dib_w = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let dib_h = i32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    // ICO/CUR の DIB は通常 height = 実高さ × 2 (XOR + AND)
    let bit_count = u16::from_le_bytes([data[14], data[15]]);

    let palette_entries: usize = match bit_count {
        1 => 2,
        4 => 16,
        8 => 256,
        24 | 32 => 0,
        other => {
            return Err(AppError::ImageProcessing(format!(
                "未対応の bpp ({} bit)。1/4/8/24/32 bit の .ico/.cur のみ対応",
                other
            )));
        }
    };

    let width = dib_w as u32;
    let real_height = dib_h.unsigned_abs() / 2;
    if width != expected_w || real_height != expected_h {
        return Err(AppError::ImageProcessing(format!(
            "ヘッダー寸法の不一致: ディレクトリ={}x{}, DIB={}x{}",
            expected_w, expected_h, width, real_height
        )));
    }

    let header_size = header_size_raw as usize;
    let palette_off = header_size;
    let palette_size = palette_entries * 4;
    let pixel_off = palette_off + palette_size;

    // XOR mask 行サイズ (ストライドは 4-byte 整列)。
    let xor_row_bytes: usize = match bit_count {
        1 => width.div_ceil(32) as usize * 4,
        4 => width.div_ceil(8) as usize * 4,
        8 => width.div_ceil(4) as usize * 4,
        24 => (width * 3).div_ceil(4) as usize * 4,
        32 => (width * 4) as usize,
        _ => unreachable!(),
    };
    let xor_size = xor_row_bytes * real_height as usize;
    if pixel_off + xor_size > data.len() {
        return Err(AppError::ImageProcessing(
            "XOR マスクが切り詰められています".to_string(),
        ));
    }
    let xor = &data[pixel_off..pixel_off + xor_size];

    // パレット (1/4/8bpp 用)。色テーブルは BGRX 4 byte/エントリで、最後の 1 byte は予約。
    let palette: Vec<[u8; 3]> = if palette_entries > 0 {
        if palette_off + palette_size > data.len() {
            return Err(AppError::ImageProcessing(
                "パレットが切り詰められています".to_string(),
            ));
        }
        (0..palette_entries)
            .map(|i| {
                let p = palette_off + i * 4;
                [data[p + 2], data[p + 1], data[p]] // R, G, B
            })
            .collect()
    } else {
        Vec::new()
    };

    // AND mask (1bpp, 4-byte 整列)。32bpp では α を優先するので不要。
    let and_row_bytes = width.div_ceil(32) as usize * 4;
    let and_size = and_row_bytes * real_height as usize;
    let and_mask: Option<&[u8]> = if bit_count != 32 {
        let and_off = pixel_off + xor_size;
        if and_off + and_size <= data.len() {
            Some(&data[and_off..and_off + and_size])
        } else {
            // AND mask が無い場合は完全不透明扱い (致命的エラーにはしない)
            None
        }
    } else {
        None
    };

    // BMP は通常ボトムアップ (dib_h > 0)。トップダウンは dib_h < 0。
    let bottom_up = dib_h > 0;

    let mut img = RgbaImage::new(width, real_height);
    for y in 0..real_height {
        let src_row = if bottom_up { real_height - 1 - y } else { y };
        let xor_row_off = src_row as usize * xor_row_bytes;
        let and_row_off = src_row as usize * and_row_bytes;
        for x in 0..width {
            let xu = x as usize;
            let (r, g, b, mut a) = match bit_count {
                32 => {
                    let off = xor_row_off + xu * 4;
                    (xor[off + 2], xor[off + 1], xor[off], xor[off + 3])
                }
                24 => {
                    let off = xor_row_off + xu * 3;
                    (xor[off + 2], xor[off + 1], xor[off], 0xff)
                }
                8 => {
                    let idx = xor[xor_row_off + xu] as usize;
                    let p = palette[idx];
                    (p[0], p[1], p[2], 0xff)
                }
                4 => {
                    let byte_off = xor_row_off + xu / 2;
                    let nibble = if xu & 1 == 0 {
                        (xor[byte_off] >> 4) & 0x0f
                    } else {
                        xor[byte_off] & 0x0f
                    };
                    let p = palette[nibble as usize];
                    (p[0], p[1], p[2], 0xff)
                }
                1 => {
                    let byte_off = xor_row_off + xu / 8;
                    let bit = 7 - (xu & 7) as u8;
                    let on = (xor[byte_off] >> bit) & 1;
                    let p = palette[on as usize];
                    (p[0], p[1], p[2], 0xff)
                }
                _ => unreachable!(),
            };

            // 非 32bpp は AND mask の bit が 1 なら透明化。
            if let Some(mask) = and_mask {
                let byte_off = and_row_off + xu / 8;
                let bit = 7 - (xu & 7) as u8;
                if (mask[byte_off] >> bit) & 1 == 1 {
                    a = 0;
                }
            }

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

    /// 8bpp パレット形式の DIB が正しく RGBA に展開されることを確認する。
    /// Windows レガシースキーム (Aero など) は 8bpp が現役なのでここを担保しないと
    /// ライブラリの「現在のスキーム」プレビューが全滅する。
    #[test]
    fn parse_ico_cur_decodes_8bpp_dib_with_palette_and_and_mask() {
        // 4x4 8bpp CUR を手組みで構築する。
        // パレット 0 = 透明位置で使う黒, 1 = 赤, 2 = 緑, 3 = 青。
        let palette_size = 256 * 4;
        let header_size: u32 = 40;
        let xor_row_bytes: usize = 4; // 4px × 1byte = 4 (4-byte aligned)
        let xor_size = xor_row_bytes * 4;
        let and_row_bytes: usize = 4; // (4 + 31) / 32 * 4 = 4
        let and_size = and_row_bytes * 4;

        let dib_size = (header_size as usize) + palette_size + xor_size + and_size;

        let mut dib: Vec<u8> = Vec::new();
        // BITMAPINFOHEADER
        dib.extend_from_slice(&header_size.to_le_bytes()); // size = 40
        dib.extend_from_slice(&4i32.to_le_bytes()); // width = 4
        dib.extend_from_slice(&8i32.to_le_bytes()); // height = real_height * 2 = 8
        dib.extend_from_slice(&1u16.to_le_bytes()); // planes = 1
        dib.extend_from_slice(&8u16.to_le_bytes()); // bit_count = 8
        dib.extend_from_slice(&0u32.to_le_bytes()); // compression = BI_RGB
        dib.extend_from_slice(&0u32.to_le_bytes()); // size_image
        dib.extend_from_slice(&0i32.to_le_bytes()); // x_ppm
        dib.extend_from_slice(&0i32.to_le_bytes()); // y_ppm
        dib.extend_from_slice(&0u32.to_le_bytes()); // colors_used
        dib.extend_from_slice(&0u32.to_le_bytes()); // colors_important
                                                    // パレット (BGRX)
        dib.extend_from_slice(&[0, 0, 0, 0]); // 0: 黒
        dib.extend_from_slice(&[0, 0, 255, 0]); // 1: 赤 (R=255 → BGR=0,0,255)
        dib.extend_from_slice(&[0, 255, 0, 0]); // 2: 緑
        dib.extend_from_slice(&[255, 0, 0, 0]); // 3: 青
        for _ in 4..256 {
            dib.extend_from_slice(&[0, 0, 0, 0]);
        }
        // XOR (ボトムアップなので最終ロジカル行が先頭)。
        // 論理的な見た目 (top→bottom):
        //   row 0: 1 1 1 1 (赤、AND mask で透明)
        //   row 1: 0 1 2 3 (黒, 赤, 緑, 青)
        //   row 2: 2 2 2 2 (全部緑)
        //   row 3: 3 3 3 3 (全部青)
        // ボトムアップ書き込み (実ファイル): row3, row2, row1, row0
        dib.extend_from_slice(&[3, 3, 3, 3]); // file row 0 = logical row 3
        dib.extend_from_slice(&[2, 2, 2, 2]); // file row 1 = logical row 2
        dib.extend_from_slice(&[0, 1, 2, 3]); // file row 2 = logical row 1
        dib.extend_from_slice(&[1, 1, 1, 1]); // file row 3 = logical row 0 (透明予定)

        // AND mask (1bpp, 4-byte 整列). ボトムアップ。
        // 論理 row 0 のみ全部透明 (bit=1 = 透明), 他は不透明 (bit=0)。
        // 各 1 行 4 byte。先頭 byte の上位 4 bit が x=0..3 を表す (MSB から)。
        // file row 0..3 = logical row 3..0 の順なので、最後の row (index 3) が透明。
        dib.extend_from_slice(&[0x00, 0, 0, 0]); // logical row 3: 不透明
        dib.extend_from_slice(&[0x00, 0, 0, 0]); // logical row 2: 不透明
        dib.extend_from_slice(&[0x00, 0, 0, 0]); // logical row 1: 不透明
        dib.extend_from_slice(&[0xf0, 0, 0, 0]); // logical row 0: 上位 4 bit (= x=0..3) ON = 透明

        assert_eq!(dib.len(), dib_size);

        // CUR コンテナを組む: ICONDIR (6) + ICONDIRENTRY (16) + DIB
        let dir_off = 6 + 16;
        let mut cur: Vec<u8> = Vec::new();
        cur.extend_from_slice(&0u16.to_le_bytes()); // reserved
        cur.extend_from_slice(&2u16.to_le_bytes()); // type = CUR
        cur.extend_from_slice(&1u16.to_le_bytes()); // count = 1
                                                    // ICONDIRENTRY
        cur.push(4); // width
        cur.push(4); // height
        cur.push(0); // colorCount
        cur.push(0); // reserved
        cur.extend_from_slice(&2u16.to_le_bytes()); // hotspot_x
        cur.extend_from_slice(&3u16.to_le_bytes()); // hotspot_y
        cur.extend_from_slice(&(dib_size as u32).to_le_bytes()); // bytes_in_res
        cur.extend_from_slice(&(dir_off as u32).to_le_bytes()); // image_offset
        cur.extend_from_slice(&dib);

        let parsed = parse_ico_cur(&cur).expect("parse 8bpp CUR");
        assert!(parsed.is_cur);
        assert_eq!(parsed.entries.len(), 1);
        let e = &parsed.entries[0];
        assert_eq!((e.width, e.height), (4, 4));
        assert_eq!((e.hotspot_x, e.hotspot_y), (2, 3));

        // 透明 (logical row 0) — α = 0
        for x in 0..4 {
            assert_eq!(
                e.image.get_pixel(x, 0).0,
                [0xff, 0, 0, 0],
                "row 0 x={x} should be transparent red",
            );
        }
        // 不透明: row 1 各色
        assert_eq!(e.image.get_pixel(0, 1).0, [0, 0, 0, 0xff]); // 黒
        assert_eq!(e.image.get_pixel(1, 1).0, [0xff, 0, 0, 0xff]); // 赤
        assert_eq!(e.image.get_pixel(2, 1).0, [0, 0xff, 0, 0xff]); // 緑
        assert_eq!(e.image.get_pixel(3, 1).0, [0, 0, 0xff, 0xff]); // 青
                                                                   // row 2 全緑 / row 3 全青
        for x in 0..4 {
            assert_eq!(e.image.get_pixel(x, 2).0, [0, 0xff, 0, 0xff]);
            assert_eq!(e.image.get_pixel(x, 3).0, [0, 0, 0xff, 0xff]);
        }
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
