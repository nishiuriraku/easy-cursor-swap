//! EasyCursorSwap .cur バイナリ生成モジュール
//!
//! RGBA ピクセルバッファから 6 サイズ（32/48/64/96/128/256px）の
//! マルチ解像度 .cur ファイルを生成する。

use crate::errors::{AppError, AppResult};
use image::{imageops::FilterType, DynamicImage, RgbaImage};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

/// リサイズ結果のグローバルキャッシュ。
///
/// キー: (元画像 SHA-256 12 文字, target_size, resample method)
///   - 元画像をそのままキーにすると重いので 12 文字短縮 SHA で衝突確率を下げつつコンパクトに
/// 値: リサイズ後の RGBA バッファ
///
/// 17 役割 × 6 サイズ = 102 枚を毎回 Lanczos でやり直すと CPU が無駄なので、
/// 同セッション内で同一 (元画像 / サイズ / 方法) なら再利用する。
/// 容量上限は単純な LRU/FIFO で 64 エントリ。
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
fn image_short_hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))[..12].to_string()
}

/// キャッシュ越しのリサイズ。同じ (画像, サイズ, method) なら再計算しない。
fn resize_image_cached(
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

/// PNG バイト列から .cur ファイルバイナリを生成する。
/// 6 サイズ全てに自動リサイズ + ホットスポットのスケーリングを行う。
///
/// 引数:
///  - `png_bytes`: 元画像 (PNG)
///  - `hotspot_x` / `hotspot_y`: 元画像での座標 (px)
///  - `resample`: リサイズアルゴリズム
///
/// メタデータ (tEXt/iTXt/zTXt/eXIf) は内部の DynamicImage 経由で自動剥離されるため、
/// 出力 .cur には元 PNG の補助チャンクは残らない。
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

    // 元画像のハッシュを 1 度だけ計算してキャッシュキーに使う
    let src_hash = image_short_hash(png_bytes);

    let mut entries: Vec<(RgbaImage, u32, u32)> = Vec::with_capacity(CURSOR_SIZES.len());
    for &target in CURSOR_SIZES {
        let resized = resize_image_cached(&img, &src_hash, target, effective);
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

// ===========================================================================
// ICO / CUR インポーター (Phase 3-4)
// ---------------------------------------------------------------------------
// 既存の .ico / .cur ファイルから複数解像度を抽出して RGBA 画像のリストとして返す。
// クリエイターモードで「既存カーソルを取り込む」ユースケース向け。
//
// サポート範囲:
//   - PNG エンコード済みエントリ (256px の Vista 以降のフォーマット)
//   - 32bpp BMP DIB エントリ (BITMAPINFOHEADER + XOR mask)
//   - AND mask は不透明度を補強する目的でのみ参照 (32bpp ではアルファを優先)
// 非対応: 1/4/8/24bpp の旧式パレットエントリ (生成側でも使われていない)
// ===========================================================================

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
        return Err(AppError::ImageProcessing(
            "エントリ数が 0 です".to_string(),
        ));
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
            .map_err(|e| {
                AppError::ImageProcessing(format!("PNG エントリのデコード失敗: {}", e))
            })?;
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
    let real_height = (dib_h.unsigned_abs() / 2) as u32;
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
        assert!(
            find_chunk(&dirty_png, b"tEXt").is_some(),
            "tEXt 注入が失敗"
        );

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
            let len = u32::from_be_bytes([
                png[pos],
                png[pos + 1],
                png[pos + 2],
                png[pos + 3],
            ]) as usize;
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
        for n in 0..256 {
            let mut c = n as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xedb88320 ^ (c >> 1) } else { c >> 1 };
            }
            table[n] = c;
        }
        let mut crc = 0xffffffffu32;
        for &b in data {
            crc = table[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
        }
        crc ^ 0xffffffff
    }

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
