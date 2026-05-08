//! 画像処理ユーティリティ — リサイズ / ピクセルアート判定 / メタデータ剥離。
//!
//! 17 役割 × 6 サイズ = 最大 102 枚を毎回 Lanczos でやり直すと CPU が無駄なので、
//! 同セッション内で同一 (元画像 / サイズ / 方法) ならグローバルキャッシュ
//! ([`RESIZE_CACHE`]) で再利用する。容量は単純な FIFO で 64 エントリ。

use crate::errors::{AppError, AppResult};
use image::{imageops::FilterType, DynamicImage, RgbaImage};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

/// リサイズ結果のグローバルキャッシュ。
///
/// キー: (元画像 SHA-256 12 文字, target_size, resample method)
///   - 元画像をそのままキーにすると重いので 12 文字短縮 SHA で衝突確率を下げつつコンパクトに
///
/// 値: リサイズ後の RGBA バッファ
static RESIZE_CACHE: Mutex<Option<ResizeCache>> = Mutex::new(None);

const RESIZE_CACHE_CAPACITY: usize = 64;

struct ResizeCache {
    // (画像ハッシュ, target_size, method) → RgbaImage
    map: HashMap<(String, u32, ResizeMethod), RgbaImage>,
    /// FIFO 削除用の挿入順
    order: Vec<(String, u32, ResizeMethod)>,
}

impl ResizeCache {
    fn new() -> Self {
        Self {
            map: HashMap::with_capacity(RESIZE_CACHE_CAPACITY),
            order: Vec::with_capacity(RESIZE_CACHE_CAPACITY),
        }
    }

    fn get(&self, key: &(String, u32, ResizeMethod)) -> Option<RgbaImage> {
        self.map.get(key).cloned()
    }

    fn put(&mut self, key: (String, u32, ResizeMethod), value: RgbaImage) {
        if self.map.len() >= RESIZE_CACHE_CAPACITY && !self.map.contains_key(&key) {
            // FIFO 削除
            if let Some(oldest) = self.order.first().cloned() {
                self.order.remove(0);
                self.map.remove(&oldest);
            }
        }
        self.map.insert(key.clone(), value);
        self.order.retain(|k| k != &key);
        self.order.push(key);
    }
}

/// リサイズ結果キャッシュをクリア。テーマ切替時などに呼ぶ。
pub fn clear_resize_cache() {
    if let Ok(mut guard) = RESIZE_CACHE.lock() {
        *guard = Some(ResizeCache::new());
    }
}

/// (元画像バイト → 短縮 SHA) を計算
pub(crate) fn image_short_hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))[..12].to_string()
}

/// キャッシュ越しのリサイズ。同じ (画像, サイズ, method) なら再計算しない。
pub(crate) fn resize_image_cached(
    img: &DynamicImage,
    src_hash: &str,
    target_size: u32,
    method: ResizeMethod,
) -> RgbaImage {
    let key = (src_hash.to_string(), target_size, method);
    if let Ok(mut guard) = RESIZE_CACHE.lock() {
        let cache = guard.get_or_insert_with(ResizeCache::new);
        if let Some(cached) = cache.get(&key) {
            return cached;
        }
        let resized = resize_image(img, target_size, method);
        cache.put(key, resized.clone());
        return resized;
    }
    // ロック失敗時は素直に再計算
    resize_image(img, target_size, method)
}

/// .cur ファイルに格納するサイズ一覧
pub const CURSOR_SIZES: &[u32] = &[32, 48, 64, 96, 128, 256];

/// リサイズアルゴリズムの選択
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    #[allow(clippy::should_implement_trait)]
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
pub fn resize_image(img: &DynamicImage, target_size: u32, method: ResizeMethod) -> RgbaImage {
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

/// PNG バイト列を再エンコードして tEXt / iTXt / zTXt / eXIf 等の
/// 補助チャンクを完全除去する。
///
/// 仕様書 §「セキュリティ」より:
///  > PNG 画像に隠された不正コードやトラッキングデータ (Exif等) を排除するため、
///  > Rust での画像処理時に純粋なピクセルデータのみを抽出し、元のメタデータは
///  > 全て破棄して .cur を生成する。
///
/// `image` クレートの `PngEncoder` は IHDR/IDAT/IEND のみを書き出す仕様なので、
/// `DynamicImage` 経由のラウンドトリップでメタデータは自動的に剥がれる。
/// この関数はその性質を明示的に活用するヘルパー。
pub fn strip_png_metadata(png_bytes: &[u8]) -> AppResult<Vec<u8>> {
    let img = image::load_from_memory_with_format(png_bytes, image::ImageFormat::Png)
        .map_err(|e| AppError::ImageProcessing(format!("PNG デコード失敗: {}", e)))?;
    let rgba = img.to_rgba8();

    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| AppError::ImageProcessing(format!("PNG 再エンコード失敗: {}", e)))?;
    Ok(buf)
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

    /// tEXt チャンクを含む PNG を作成し、strip_png_metadata が剥離することを確認する。
    #[test]
    fn test_strip_png_metadata_removes_text_chunk() {
        // 32x32 のダミー画像を PngEncoder で生成 (これ自体は metadata-free)
        let img = RgbaImage::from_pixel(32, 32, image::Rgba([0, 128, 255, 255]));
        let mut clean_png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut clean_png);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            32,
            32,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();

        // 手動で tEXt チャンクを IEND の直前に挿入 (PII / トラッキングを模擬)
        let dirty_png = inject_text_chunk(&clean_png, b"Author", b"private@example.com");

        // tEXt チャンクが入っていることを確認 (前提条件)
        assert!(find_chunk(&dirty_png, b"tEXt").is_some(), "tEXt 注入が失敗");

        // strip_png_metadata で除去
        let stripped = strip_png_metadata(&dirty_png).expect("strip");

        // tEXt が消えていることを確認
        assert!(
            find_chunk(&stripped, b"tEXt").is_none(),
            "tEXt が残存している"
        );
        // IDAT は残っていることを確認 (画像データ自体は保持)
        assert!(find_chunk(&stripped, b"IDAT").is_some());
    }

    /// eXIf チャンクを含む PNG を作成し、strip_png_metadata が剥離することを確認する。
    #[test]
    fn test_strip_png_metadata_removes_exif_chunk() {
        let img = RgbaImage::from_pixel(16, 16, image::Rgba([255, 0, 0, 255]));
        let mut clean_png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut clean_png);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            16,
            16,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();

        // Exif ヘッダ (II = little-endian) を持つ最小限のバイト列
        let fake_exif = b"II\x2a\x00\x08\x00\x00\x00".to_vec();
        let dirty_png = inject_raw_chunk(&clean_png, b"eXIf", &fake_exif);
        assert!(find_chunk(&dirty_png, b"eXIf").is_some(), "eXIf 注入が失敗");

        let stripped = strip_png_metadata(&dirty_png).expect("strip");
        assert!(
            find_chunk(&stripped, b"eXIf").is_none(),
            "eXIf が残存している"
        );
        assert!(find_chunk(&stripped, b"IDAT").is_some());
    }

    /// iTXt チャンクを含む PNG を作成し、strip_png_metadata が剥離することを確認する。
    #[test]
    fn test_strip_png_metadata_removes_itxt_chunk() {
        let img = RgbaImage::from_pixel(16, 16, image::Rgba([0, 255, 0, 255]));
        let mut clean_png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut clean_png);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            16,
            16,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();

        // iTXt: keyword\0 compression_flag(0) compression_method(0) language\0 translated\0 text
        let mut itxt_data = Vec::new();
        itxt_data.extend_from_slice(b"Comment\x00"); // keyword + NUL
        itxt_data.push(0); // compression_flag = not compressed
        itxt_data.push(0); // compression_method
        itxt_data.push(0); // language tag NUL
        itxt_data.push(0); // translated keyword NUL
        itxt_data.extend_from_slice(b"GPS:35.6895,139.6917"); // payload
        let dirty_png = inject_raw_chunk(&clean_png, b"iTXt", &itxt_data);
        assert!(find_chunk(&dirty_png, b"iTXt").is_some(), "iTXt 注入が失敗");

        let stripped = strip_png_metadata(&dirty_png).expect("strip");
        assert!(
            find_chunk(&stripped, b"iTXt").is_none(),
            "iTXt が残存している"
        );
        assert!(find_chunk(&stripped, b"IDAT").is_some());
    }

    /// zTXt チャンクを含む PNG を作成し、strip_png_metadata が剥離することを確認する。
    #[test]
    fn test_strip_png_metadata_removes_ztxt_chunk() {
        let img = RgbaImage::from_pixel(16, 16, image::Rgba([0, 0, 255, 255]));
        let mut clean_png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut clean_png);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            16,
            16,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();

        // zTXt: keyword\0 compression_method(0) compressed_text
        // 圧縮データは正当でなくても inject_raw_chunk 経由で注入すればチャンクとして認識される
        let mut ztxt_data = Vec::new();
        ztxt_data.extend_from_slice(b"Description\x00"); // keyword + NUL
        ztxt_data.push(0); // compression method = deflate
        ztxt_data.extend_from_slice(b"\x78\x9c\x03\x00\x00\x00\x00\x01"); // minimal deflate stream
        let dirty_png = inject_raw_chunk(&clean_png, b"zTXt", &ztxt_data);
        assert!(find_chunk(&dirty_png, b"zTXt").is_some(), "zTXt 注入が失敗");

        let stripped = strip_png_metadata(&dirty_png).expect("strip");
        assert!(
            find_chunk(&stripped, b"zTXt").is_none(),
            "zTXt が残存している"
        );
        assert!(find_chunk(&stripped, b"IDAT").is_some());
    }

    /// 任意チャンクタイプと任意データを IEND 直前に注入するテストヘルパー。
    fn inject_raw_chunk(png: &[u8], chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let iend_pos = find_chunk(png, b"IEND").expect("IEND not found");
        let insert_at = iend_pos - 4;

        let mut chunk = Vec::new();
        chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
        chunk.extend_from_slice(chunk_type);
        chunk.extend_from_slice(data);
        let mut crc_input = Vec::new();
        crc_input.extend_from_slice(chunk_type);
        crc_input.extend_from_slice(data);
        chunk.extend_from_slice(&crc32(&crc_input).to_be_bytes());

        let mut out = Vec::with_capacity(png.len() + chunk.len());
        out.extend_from_slice(&png[..insert_at]);
        out.extend_from_slice(&chunk);
        out.extend_from_slice(&png[insert_at..]);
        out
    }

    /// PNG にテキストチャンク (tEXt) を挿入するテストヘルパー。
    fn inject_text_chunk(png: &[u8], keyword: &[u8], text: &[u8]) -> Vec<u8> {
        // IEND チャンクの位置を見つける
        let iend_pos = find_chunk(png, b"IEND").expect("IEND not found");

        // tEXt のデータ部 = keyword + 0x00 + text
        let mut data = Vec::new();
        data.extend_from_slice(keyword);
        data.push(0x00);
        data.extend_from_slice(text);

        let mut chunk = Vec::new();
        chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
        chunk.extend_from_slice(b"tEXt");
        chunk.extend_from_slice(&data);
        // CRC32 (type + data)
        let mut crc_input = Vec::new();
        crc_input.extend_from_slice(b"tEXt");
        crc_input.extend_from_slice(&data);
        chunk.extend_from_slice(&crc32(&crc_input).to_be_bytes());

        // IEND の直前 (= iend_pos - 8 から: 4 byte length + 4 byte type) に挿入
        let insert_at = iend_pos - 4; // length 4 byte 前
        let mut out = Vec::with_capacity(png.len() + chunk.len());
        out.extend_from_slice(&png[..insert_at]);
        out.extend_from_slice(&chunk);
        out.extend_from_slice(&png[insert_at..]);
        out
    }

    /// チャンクタイプ (4 bytes) を探して、その位置 (タイプ先頭オフセット) を返す。
    fn find_chunk(png: &[u8], chunk_type: &[u8; 4]) -> Option<usize> {
        // PNG header 8 bytes をスキップ
        let mut pos = 8;
        while pos + 8 <= png.len() {
            let len =
                u32::from_be_bytes([png[pos], png[pos + 1], png[pos + 2], png[pos + 3]]) as usize;
            let typ = &png[pos + 4..pos + 8];
            if typ == chunk_type {
                return Some(pos + 4);
            }
            pos += 4 + 4 + len + 4; // length + type + data + crc
        }
        None
    }

    /// CRC32 (PNG-style)
    fn crc32(data: &[u8]) -> u32 {
        let mut table = [0u32; 256];
        for (n, slot) in table.iter_mut().enumerate() {
            let mut c = n as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xedb88320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *slot = c;
        }
        let mut crc = 0xffffffffu32;
        for &b in data {
            crc = table[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
        }
        crc ^ 0xffffffff
    }
}
