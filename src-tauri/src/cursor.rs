//! CursorForge .cur バイナリ生成モジュール
//!
//! RGBA ピクセルバッファから 6 サイズ（32/48/64/96/128/256px）の
//! マルチ解像度 .cur ファイルを生成する。

use crate::errors::{AppError, AppResult};
use image::{imageops::FilterType, DynamicImage, RgbaImage};

/// .cur ファイルに格納するサイズ一覧
pub const CURSOR_SIZES: &[u32] = &[32, 48, 64, 96, 128, 256];

/// リサイズアルゴリズムの選択
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeMethod {
    /// 滑らかな画像向け（Lanczos3）
    Lanczos,
    /// ドット絵向け（Nearest Neighbor）
    Nearest,
}

impl ResizeMethod {
    /// image クレートの FilterType に変換
    fn to_filter_type(self) -> FilterType {
        match self {
            ResizeMethod::Lanczos => FilterType::Lanczos3,
            ResizeMethod::Nearest => FilterType::Nearest,
        }
    }

    /// 文字列からパース
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "nearest" | "pixel" | "dot" => ResizeMethod::Nearest,
            _ => ResizeMethod::Lanczos,
        }
    }
}

/// 画像がドット絵（ピクセルアート）かどうかを自動判定する
///
/// 判定基準:
/// - 使用色数が少ない（≤ 64色）
/// - アンチエイリアスされていない（ピクセル境界が鮮明）
pub fn detect_pixel_art(img: &RgbaImage) -> bool {
    let mut colors = std::collections::HashSet::new();
    let sample_limit = 10000; // パフォーマンスのためサンプリング
    let total_pixels = img.width() * img.height();
    let step = (total_pixels / sample_limit).max(1);

    for (i, pixel) in img.pixels().enumerate() {
        if i as u32 % step != 0 {
            continue;
        }
        // アルファ値を含めた完全な色をカウント
        colors.insert([pixel[0], pixel[1], pixel[2], pixel[3]]);
        // 64色超ならピクセルアートではない
        if colors.len() > 64 {
            return false;
        }
    }

    true
}

/// RGBA バッファから指定サイズにリサイズする
pub fn resize_image(
    img: &DynamicImage,
    target_size: u32,
    method: ResizeMethod,
) -> RgbaImage {
    img.resize_exact(target_size, target_size, method.to_filter_type())
        .to_rgba8()
}

/// ホットスポットを元画像サイズから目標サイズにスケーリングする
pub fn scale_hotspot(
    hotspot_x: u32,
    hotspot_y: u32,
    original_size: u32,
    target_size: u32,
) -> (u32, u32) {
    if original_size == 0 {
        return (0, 0);
    }
    let scale = target_size as f64 / original_size as f64;
    let new_x = (hotspot_x as f64 * scale).round() as u32;
    let new_y = (hotspot_y as f64 * scale).round() as u32;
    // 座標がサイズを超えないようにクランプ
    (new_x.min(target_size - 1), new_y.min(target_size - 1))
}

/// PNG バイト列から .cur ファイルバイナリを生成する。
/// 6 サイズ全てに自動リサイズ + ホットスポットのスケーリングを行う。
///
/// 引数:
///  - `png_bytes`: 元画像 (PNG)
///  - `hotspot_x` / `hotspot_y`: 元画像での座標 (px)
///  - `resample`: リサイズアルゴリズム
pub fn build_cur_from_png(
    png_bytes: &[u8],
    hotspot_x: u32,
    hotspot_y: u32,
    resample: ResizeMethod,
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

    let mut entries: Vec<(RgbaImage, u32, u32)> = Vec::with_capacity(CURSOR_SIZES.len());
    for &target in CURSOR_SIZES {
        let resized = resize_image(&img, target, effective);
        let (hx, hy) = scale_hotspot(hotspot_x, hotspot_y, original_size, target);
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
    use super::*;

    #[test]
    fn test_scale_hotspot() {
        // 128px の (64, 64) を 32px にスケール → (16, 16)
        let (x, y) = scale_hotspot(64, 64, 128, 32);
        assert_eq!(x, 16);
        assert_eq!(y, 16);
    }

    #[test]
    fn test_scale_hotspot_zero() {
        let (x, y) = scale_hotspot(0, 0, 0, 32);
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn test_detect_pixel_art() {
        // 単色画像はピクセルアートとして検出される
        let img = RgbaImage::from_pixel(32, 32, image::Rgba([255, 0, 0, 255]));
        assert!(detect_pixel_art(&img));
    }

    #[test]
    fn test_resize_method_from_str() {
        assert_eq!(ResizeMethod::from_str("nearest"), ResizeMethod::Nearest);
        assert_eq!(ResizeMethod::from_str("pixel"), ResizeMethod::Nearest);
        assert_eq!(ResizeMethod::from_str("lanczos"), ResizeMethod::Lanczos);
        assert_eq!(ResizeMethod::from_str("unknown"), ResizeMethod::Lanczos);
    }
}
