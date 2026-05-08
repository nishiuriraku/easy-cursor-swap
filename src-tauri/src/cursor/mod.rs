//! EasyCursorSwap 画像処理 / `.cur` バイナリ生成 / `.ico` `.cur` `.ani` パース モジュール群。
//!
//! 4 つのサブモジュールに責務分割している:
//!
//! | モジュール | 担当領域 |
//! |---|---|
//! | [`image`]     | リサイズ / ピクセルアート判定 / ホットスポット計算 / PNG メタデータ剥離 |
//! | [`cur_build`] | PNG → 6 解像度 `.cur` バイナリ生成 (sized override 対応) |
//! | [`ico_cur`]   | `.ico` `.cur` の解析 (PNG エントリ + 32bpp BMP DIB エントリ) |
//! | [`ani`]       | `.ani` (RIFF/ACON) アニメーションカーソルの解析 |
//!
//! 公開 API はすべて本ファイルから `pub use` で再エクスポートしているため、
//! 既存の `use crate::cursor::ResizeMethod;` 等の import は分割後も変更不要。

pub mod ani;
pub mod cur_build;
pub mod ico_cur;
pub mod image;

pub use self::ani::{parse_ani, ParsedAni};
pub use self::cur_build::{build_cur_from_png, generate_cur_binary};
pub use self::ico_cur::{parse_ico_cur, pick_largest_as_png, ParsedIcoCur, ParsedIcoCurEntry};
pub use self::image::{
    clear_resize_cache, detect_pixel_art, resize_image, scale_hotspot, strip_png_metadata,
    ResizeMethod, CURSOR_SIZES,
};
