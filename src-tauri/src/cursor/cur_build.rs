//! `.cur` バイナリ生成 — 1 枚の PNG → 6 解像度 .cur にパッキング。
//!
//! ICO/CUR 共通フォーマット: ICO ヘッダー (6) + ディレクトリエントリ (16 × N) + 画像データ。
//! Type フィールドが 2 のものを CUR と呼び、ホットスポット (X, Y) を planes/bit_count
//! の位置に詰める。本モジュールでは PNG エンコード済みエントリ (Vista 以降の標準) を使用する。

use super::image::{
    detect_pixel_art, image_short_hash, resize_image_cached, scale_hotspot, ResizeMethod,
    CURSOR_SIZES,
};
use crate::errors::{AppError, AppResult};
use image::RgbaImage;

/// PNG バイト列から .cur ファイルバイナリを生成する。
/// 6 サイズ全てに自動リサイズ + ホットスポットのスケーリングを行う。
///
/// 引数:
///  - `png_bytes`: 元画像 (PNG)
///  - `hotspot_x` / `hotspot_y`: 元画像での座標 (px)
///  - `resample`: リサイズアルゴリズム
///  - `sized_overrides`: サイズ別オーバーライド PNG (サイズ px → PNG バイト列)
///  - `per_size_hotspot_px`: サイズ別ホットスポット px (`sized_overrides` の各エントリに
///    独立した hotspot が指定されている場合のみ存在)。`None` / 該当サイズなしの場合は
///    `hotspot_x` / `hotspot_y` を `scale_hotspot` でスケールした値を使用する。
///
/// メタデータ (tEXt/iTXt/zTXt/eXIf) は内部の DynamicImage 経由で自動剥離されるため、
/// 出力 .cur には元 PNG の補助チャンクは残らない。
pub fn build_cur_from_png(
    png_bytes: &[u8],
    hotspot_x: u32,
    hotspot_y: u32,
    resample: ResizeMethod,
    sized_overrides: Option<&std::collections::HashMap<u32, Vec<u8>>>,
    per_size_hotspot_px: Option<&std::collections::HashMap<u32, (u32, u32)>>,
) -> AppResult<Vec<u8>> {
    // PNG マジックバイト検証 (Magic Byte 第一防御線)
    const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if png_bytes.len() < 8 || png_bytes[..8] != PNG_MAGIC {
        return Err(AppError::ImageProcessing(
            "PNG ヘッダーが不正です".to_string(),
        ));
    }

    let img = image::load_from_memory_with_format(png_bytes, image::ImageFormat::Png)
        .map_err(|e| AppError::ImageProcessing(format!("PNG デコード失敗: {}", e)))?;

    let original_width = img.width();
    let original_height = img.height();
    if original_width == 0 || original_height == 0 {
        return Err(AppError::ImageProcessing(
            "画像サイズがゼロです".to_string(),
        ));
    }

    // 自動判定モード: PNG が小さいときはピクセルアートと推定
    let effective = if matches!(resample, ResizeMethod::Lanczos)
        && original_width <= 64
        && detect_pixel_art(&img.to_rgba8())
    {
        ResizeMethod::Nearest
    } else {
        resample
    };

    // ホットスポットは元画像が長辺なら長辺サイズに対する比率
    let original_size = original_width.max(original_height);

    // 元画像のハッシュを 1 度だけ計算してキャッシュキーに使う
    let src_hash = image_short_hash(png_bytes);

    let mut entries: Vec<(RgbaImage, u32, u32)> = Vec::with_capacity(CURSOR_SIZES.len());
    for &target in CURSOR_SIZES {
        let img_at_size = if let Some(overrides) = sized_overrides {
            if let Some(bytes) = overrides.get(&target) {
                // オーバーライドの PNG をデコード。サイズが合わなければリサンプルでフォールバック。
                match image::load_from_memory_with_format(bytes, image::ImageFormat::Png) {
                    Ok(decoded) => {
                        let r = decoded.to_rgba8();
                        if r.width() == target && r.height() == target {
                            Some(r)
                        } else {
                            tracing::warn!(
                                "sized override {}px のサイズが {}x{} で不一致 → リサンプル",
                                target,
                                r.width(),
                                r.height()
                            );
                            None
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "sized override {}px の decode 失敗: {} → リサンプル",
                            target,
                            e
                        );
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        let resized =
            img_at_size.unwrap_or_else(|| resize_image_cached(&img, &src_hash, target, effective));
        // サイズ別ホットスポット override が指定されていればそれを使用し、
        // なければ primary hotspot をターゲットサイズへスケーリングする。
        let (hx, hy) = per_size_hotspot_px
            .and_then(|m| m.get(&target))
            .copied()
            .unwrap_or_else(|| scale_hotspot(hotspot_x, hotspot_y, original_size, target));
        entries.push((resized, hx, hy));
    }

    generate_cur_binary(&entries)
}

/// .cur ファイルのバイナリを生成する
///
/// 複数の解像度画像を1つの .cur ファイルにパッキングする
/// フォーマット: ICO ヘッダー (6 bytes) + ディレクトリエントリ (16 bytes × N) + 画像データ
pub fn generate_cur_binary(
    images: &[(RgbaImage, u32, u32)], // (画像, hotspot_x, hotspot_y)
) -> AppResult<Vec<u8>> {
    if images.is_empty() {
        return Err(AppError::ImageProcessing(
            "画像が1枚も指定されていません".to_string(),
        ));
    }

    let num_images = images.len() as u16;
    let mut buffer = Vec::new();

    // --- ICO ヘッダー (6 bytes) ---
    buffer.extend_from_slice(&0u16.to_le_bytes()); // Reserved (0)
    buffer.extend_from_slice(&2u16.to_le_bytes()); // Type: 2 = CUR
    buffer.extend_from_slice(&num_images.to_le_bytes()); // Number of images

    // --- 各画像のPNGデータを先に生成 ---
    let mut png_data: Vec<Vec<u8>> = Vec::new();
    for (img, _, _) in images {
        let mut png_buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_buf);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| AppError::ImageProcessing(format!("PNG エンコードに失敗: {}", e)))?;
        png_data.push(png_buf);
    }

    // --- ディレクトリエントリ (16 bytes × N) ---
    let header_size = 6 + 16 * num_images as u32;
    let mut data_offset = header_size;

    for (i, (img, hotspot_x, hotspot_y)) in images.iter().enumerate() {
        let width = img.width();
        let height = img.height();
        let png_size = png_data[i].len() as u32;

        // Width (0 = 256px)
        buffer.push(if width >= 256 { 0 } else { width as u8 });
        // Height (0 = 256px)
        buffer.push(if height >= 256 { 0 } else { height as u8 });
        // Color count (0 for 32-bit)
        buffer.push(0);
        // Reserved
        buffer.push(0);
        // Hotspot X (CUR形式でのプレーン数の代わり)
        buffer.extend_from_slice(&(*hotspot_x as u16).to_le_bytes());
        // Hotspot Y (CUR形式でのビット深度の代わり)
        buffer.extend_from_slice(&(*hotspot_y as u16).to_le_bytes());
        // Image data size
        buffer.extend_from_slice(&png_size.to_le_bytes());
        // Data offset
        buffer.extend_from_slice(&data_offset.to_le_bytes());

        data_offset += png_size;
    }

    // --- 画像データ ---
    for data in &png_data {
        buffer.extend_from_slice(data);
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::super::ico_cur::parse_ico_cur;
    use super::*;

    /// build_cur_from_png に sized_overrides を渡したとき、対応サイズはリサンプルではなく
    /// 渡された PNG をそのまま使う。
    #[test]
    fn build_cur_uses_sized_override_when_size_matches() {
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

        // primary は 256x256 の青
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

        let mut overrides = std::collections::HashMap::new();
        overrides.insert(64u32, png64.clone());

        // sized_overrides ありでビルド (per_size_hotspot_px は指定なし)
        let cur_bytes =
            build_cur_from_png(&png256, 0, 0, ResizeMethod::Lanczos, Some(&overrides), None)
                .unwrap();

        // 出力 .cur をパースして 64x64 のエントリの最初のピクセルが赤か確認
        let parsed = parse_ico_cur(&cur_bytes).unwrap();
        let entry_64 = parsed
            .entries
            .iter()
            .find(|e| e.width == 64)
            .expect("64px エントリがあるはず");
        let pixel = entry_64.image.get_pixel(0, 0);
        assert_eq!(pixel[0], 255, "R");
        assert_eq!(pixel[1], 0, "G");
        assert_eq!(pixel[2], 0, "B");
        assert_eq!(pixel[3], 255, "A");
    }
}
