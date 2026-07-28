//! `cursor::ani_write::build_ani` の RED テスト群 (Wave 3C)。
//!
//! PNG フレーム列から 6 サイズ CUR 連結 + RIFF ACON を一気通貫で構築する
//! 純粋 Rust writer の挙動を固定化する。
//!
//! ## 方針
//! - 既存 `parse_ani` で round-trip し、`num_frames` / `num_steps` / `sequence` /
//!   `default_rate_jiffies` / `per_step_rate_jiffies` が意図通りかを検証。
//! - 6 サイズ CUR は既存 `build_cur_from_png` 経由なのでサイズ検証は
//!   `parse_ico_cur` 側でも検証可能。
//! - RIFF ACON のチャンク整合性 (anih.cFrames / rate.len == cSteps /
//!   seq.len == cFrames / 偶数 padding) はバイト列を直接覗いて検証。
//!
//! ## 上限
//! - frames ≤ 64
//! - sequence ≤ 256
//! - total pixel bytes ≤ 20 MB
//! - max duration ≤ 10 秒 / step

use super::super::ani::parse_ani;
use super::super::cur_build::generate_cur_binary;
use super::super::ico_cur::parse_ico_cur;
use super::*;
use image::RgbaImage;

/// 32x32 の単色 PNG を 1 枚生成 (テスト fixture 用)。
fn make_png(color: [u8; 4]) -> Vec<u8> {
    let img = RgbaImage::from_pixel(32, 32, image::Rgba(color));
    let mut buf = Vec::new();
    image::ImageEncoder::write_image(
        image::codecs::png::PngEncoder::new(&mut buf),
        img.as_raw(),
        32,
        32,
        image::ExtendedColorType::Rgba8,
    )
    .expect("PNG encode");
    buf
}

/// `build_ani` の最小呼び出し: 1 frame / 既定 durations (60 jiffies = 1s) / no theme。
#[test]
fn build_ani_single_frame_round_trip() {
    let png = make_png([255, 0, 0, 255]);
    let frames = vec![AniFrameInput {
        png_bytes: png,
        hotspot_ratio: (0.5, 0.5),
        durations_ms: vec![1000],
        sequence_indices: vec![],
        sizes: None,
    }];
    let bytes = build_ani(frames, AniBuildOptions::default()).expect("build_ani");
    let parsed = parse_ani(&bytes).expect("parse_ani");
    assert_eq!(parsed.num_frames, 1);
    assert_eq!(parsed.num_steps, 1);
    // 1000ms → 60 jiffies
    assert_eq!(parsed.default_rate_jiffies, 60);
    assert_eq!(parsed.per_step_rate_jiffies.len(), 1);
    assert_eq!(parsed.per_step_rate_jiffies[0], 60);
}

/// `build_ani` で複数 frame を構築 → num_frames / num_steps 一致。
#[test]
fn build_ani_two_frames_round_trip() {
    let png1 = make_png([255, 0, 0, 255]);
    let png2 = make_png([0, 255, 0, 255]);
    let frames = vec![
        AniFrameInput {
            png_bytes: png1,
            hotspot_ratio: (0.5, 0.5),
            durations_ms: vec![500],
            sequence_indices: vec![],
            sizes: None,
        },
        AniFrameInput {
            png_bytes: png2,
            hotspot_ratio: (0.25, 0.75),
            durations_ms: vec![500],
            sequence_indices: vec![],
            sizes: None,
        },
    ];
    let bytes = build_ani(frames, AniBuildOptions::default()).expect("build_ani");
    let parsed = parse_ani(&bytes).expect("parse_ani");
    assert_eq!(parsed.num_frames, 2);
    assert_eq!(parsed.num_steps, 2);
    // 500ms = (500*60+500)/1000 = 30 jiffies
    assert_eq!(parsed.per_step_rate_jiffies, vec![30, 30]);
}

/// jiffies 計算: 100ms → 6 (60 jiffies/秒 × 100ms = 6)、16ms → 1 (max(1, ...) で 0 を避ける)。
#[test]
fn build_ani_jiffies_conversion() {
    let png = make_png([255, 0, 0, 255]);
    // 16.67ms → round(16.67 * 60 / 1000) = round(1.002) = 1 → max(1, ...) で 1
    // 16ms → round(16 * 60 / 1000) = round(0.96) = 1 → max(1, ...) で 1
    let frames = vec![AniFrameInput {
        png_bytes: png,
        hotspot_ratio: (0.5, 0.5),
        durations_ms: vec![100, 16],
        sequence_indices: vec![],
        sizes: None,
    }];
    let bytes = build_ani(frames, AniBuildOptions::default()).expect("build_ani");
    let parsed = parse_ani(&bytes).expect("parse_ani");
    // 100ms → 6 jiffies
    assert_eq!(parsed.per_step_rate_jiffies[0], 6);
    // 16ms → round(0.96) = 1 → max(1, 1) = 1
    assert_eq!(parsed.per_step_rate_jiffies[1], 1);
}

/// 各 frame の CUR が 6 サイズ (32/48/64/96/128/256) を含むことを確認。
#[test]
fn build_ani_each_frame_has_six_cur_sizes() {
    let png = make_png([255, 0, 0, 255]);
    let frames = vec![AniFrameInput {
        png_bytes: png,
        hotspot_ratio: (0.5, 0.5),
        durations_ms: vec![1000],
        sequence_indices: vec![],
        sizes: None,
    }];
    let bytes = build_ani(frames, AniBuildOptions::default()).expect("build_ani");
    let parsed = parse_ani(&bytes).expect("parse_ani");
    assert_eq!(parsed.frames.len(), 1);
    let raw = &bytes[parsed.frame_infos[0].raw_data_range.clone()];
    let cur = parse_ico_cur(raw).expect("parse CUR");
    assert_eq!(cur.entries.len(), 6, "6 サイズ CUR であるべき");
    // サイズは 32/48/64/96/128/256 (256 は 0 で表現)
    let mut widths: Vec<u32> = cur.entries.iter().map(|e| e.width).collect();
    widths.sort();
    assert_eq!(widths, vec![32, 48, 64, 96, 128, 256]);
}

/// sizes override は v1 では未実装 (YAGNI: per-size PNG バイトが必要なため
/// AniFrameInput interface 拡張が必要。将来 hook)。
#[test]
fn build_ani_six_default_sizes_unchanged() {
    // sizes = None の既定挙動が 6 サイズ CUR になることを再確認 (build_ani_each_frame_has_six_cur_sizes の variant)。
    // sizes override 対応は Wave 3C 後の将来 hook。
    let png = make_png([255, 0, 0, 255]);
    let frames = vec![AniFrameInput {
        png_bytes: png,
        hotspot_ratio: (0.5, 0.5),
        durations_ms: vec![1000],
        sequence_indices: vec![],
        sizes: Some(vec![16, 64]), // override 指定しても現状は未対応 = 6 サイズのまま (interface 拡張は将来)
    }];
    let bytes = build_ani(frames, AniBuildOptions::default()).expect("build_ani");
    let parsed = parse_ani(&bytes).expect("parse_ani");
    let raw = &bytes[parsed.frame_infos[0].raw_data_range.clone()];
    let cur = parse_ico_cur(raw).expect("parse CUR");
    assert_eq!(cur.entries.len(), 6, "sizes override 未対応 = 6 既定サイズ");
}

/// RIFF ACON チャンク整合性: RIFF size = bytes.len() - 8、ACON marker 確認。
#[test]
fn build_ani_riff_size_and_acon_marker() {
    let png = make_png([255, 0, 0, 255]);
    let frames = vec![AniFrameInput {
        png_bytes: png,
        hotspot_ratio: (0.5, 0.5),
        durations_ms: vec![1000],
        sequence_indices: vec![],
        sizes: None,
    }];
    let bytes = build_ani(frames, AniBuildOptions::default()).expect("build_ani");
    let riff_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    assert_eq!(riff_size, bytes.len() - 8, "RIFF size = total - 8");
    assert_eq!(&bytes[8..12], b"ACON", "ACON marker");
}

/// frame 0 件 → エラー。
#[test]
fn build_ani_rejects_empty_input() {
    let frames: Vec<AniFrameInput> = vec![];
    let result = build_ani(frames, AniBuildOptions::default());
    assert!(result.is_err(), "frame 0 件はエラー");
}

/// frame 上限 (>64) → エラー。
#[test]
fn build_ani_rejects_too_many_frames() {
    let png = make_png([255, 0, 0, 255]);
    let frames: Vec<AniFrameInput> = (0..65)
        .map(|_| AniFrameInput {
            png_bytes: png.clone(),
            hotspot_ratio: (0.5, 0.5),
            durations_ms: vec![1000],
            sequence_indices: vec![],
            sizes: None,
        })
        .collect();
    let result = build_ani(frames, AniBuildOptions::default());
    assert!(result.is_err(), "65 frames は上限超過エラー");
}

/// INAM / IART メタデータが正しく書き込まれる。
#[test]
fn build_ani_includes_inam_and_iart() {
    let png = make_png([255, 0, 0, 255]);
    let frames = vec![AniFrameInput {
        png_bytes: png,
        hotspot_ratio: (0.5, 0.5),
        durations_ms: vec![1000],
        sequence_indices: vec![],
        sizes: None,
    }];
    let opts = AniBuildOptions {
        theme_name: Some("Test Theme".to_string()),
        author: Some("alice".to_string()),
        default_jiffies: 60,
    };
    let bytes = build_ani(frames, opts).expect("build_ani");
    let needle_inam = "Test Theme".as_bytes();
    let needle_iart = "alice".as_bytes();
    assert!(
        find_subsequence(&bytes, needle_inam).is_some(),
        "INAM (theme_name) が見つからない"
    );
    assert!(
        find_subsequence(&bytes, needle_iart).is_some(),
        "IART (author) が見つからない"
    );
}

/// bytes 中で needle の最初の出現位置を返す (RIFF 走査用)。
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// 既存の helper がそのまま使えることを保証 (test の妥当性)。
#[test]
fn smoke_generate_cur_binary() {
    let img = RgbaImage::from_pixel(32, 32, image::Rgba([0, 255, 0, 255]));
    let cur = generate_cur_binary(&[(img, 16, 16)]).expect("generate_cur_binary");
    let parsed = parse_ico_cur(&cur).expect("parse_ico_cur");
    assert_eq!(parsed.entries.len(), 1);
}
