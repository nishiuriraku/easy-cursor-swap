//! `.ani` (RIFF/ACON) パーサー — アニメーションカーソル抽出。
//!
//! 構造:
//! ```text
//!   'RIFF' <total_size:u32> 'ACON' <chunks>
//!   chunks:
//!     'anih' <size> <ANIHEADER 36 bytes>
//!     'rate' <size> <u32 array of jiffies per frame> (省略可: あれば anih.jifRate を上書き)
//!     'seq ' <size> <u32 array of frame indices>     (省略可: 再生順 anih.cSteps 個)
//!     'LIST' <size> 'fram' <icon chunks>
//!        'icon' <size> <CUR ファイル全体>
//!     'LIST' <size> 'INFO' <INAM/IART chunks> (タイトル・作者など、無視可)
//! ```
//!
//! 1 jiffy = 1/60 秒。`jifRate` の既定が 0 のときは 1 を採用する。

use super::ico_cur::{parse_ico_cur, ParsedIcoCurEntry};
use crate::errors::{AppError, AppResult};

/// `.ani` anih.flags のビット 0。立っていれば各 icon chunk のデータが完全な CUR ファイル。
/// 0 のときは raw DIB 形式 (旧形式)。
const AF_ICON: u32 = 0x01;

/// `.ani` の各フレームが「完全な CUR ファイル (新形式)」か「raw DIB (旧形式)」かを表す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AniFrameFormat {
    /// 新形式: icon chunk のデータ部分が完全な CUR ファイル
    AfIcon,
    /// 旧形式: icon chunk のデータ部分が裸の BITMAPINFOHEADER + DIB
    RawDib { bit_count: u16 },
}

/// 解析済み 1 フレームのメタ情報 (画像本体は `ParsedIcoCurEntry`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AniFrameInfo {
    /// 元バイト列における icon chunk **データ部** のバイト範囲 (8 バイトヘッダ後)
    pub raw_data_range: std::ops::Range<usize>,
    /// フレームデータの形式 (AF_ICON / raw DIB)。
    pub format: AniFrameFormat,
}

/// 解析済み .ani ファイル全体
#[derive(Debug, Clone)]
pub struct ParsedAni {
    /// 格納フレーム数 (cFrames)
    pub num_frames: u32,
    /// 再生ステップ数 (cSteps、シーケンス使用時は num_frames を超え得る)
    pub num_steps: u32,
    /// 既定のフレーム表示時間 (jiffies = 1/60 sec)
    pub default_rate_jiffies: u32,
    /// 各ステップの表示時間 (jiffies)。`seq` がない場合 num_frames 個、ある場合 num_steps 個。
    /// 'rate' チャンク非搭載時は default_rate_jiffies を均一に展開した配列。
    pub per_step_rate_jiffies: Vec<u32>,
    /// 再生シーケンス (フレームインデックス配列)。'seq ' チャンクがあればその値、
    /// なければ `[0, 1, 2, ..., num_frames-1]` を返す。
    pub sequence: Vec<u32>,
    /// 抽出された各フレーム (CUR をデコードした RgbaImage)。
    /// 同サイズが複数解像度埋め込まれている場合は最大解像度を採用。
    pub frames: Vec<ParsedIcoCurEntry>,
    /// 各フレームに対応するメタ情報。`frames` と同じ順序・同じ長さ。
    pub frame_infos: Vec<AniFrameInfo>,
    /// anih.flags に AF_ICON (0x01) が立っていなかった場合 true (旧形式 raw DIB)。
    pub is_legacy_raw_dib: bool,
}

impl ParsedAni {
    /// 1 ループあたりの総再生時間 (ミリ秒)。
    /// 各フレームでの整数除算による累積誤差を避けるため、まず jiffies を合計してから ms 換算する。
    pub fn total_duration_ms(&self) -> u64 {
        let total_jiffies: u64 = self.per_step_rate_jiffies.iter().map(|&j| j as u64).sum();
        (total_jiffies * 1000) / 60
    }
}

/// `.ani` バイナリを解析する。
pub fn parse_ani(bytes: &[u8]) -> AppResult<ParsedAni> {
    if bytes.len() < 12 {
        return Err(AppError::ImageProcessing(
            "ANI ファイルが短すぎます (RIFF ヘッダー不足)".to_string(),
        ));
    }
    if &bytes[0..4] != b"RIFF" {
        return Err(AppError::ImageProcessing(
            "RIFF ヘッダーが見つかりません".to_string(),
        ));
    }
    let riff_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if &bytes[8..12] != b"ACON" {
        return Err(AppError::ImageProcessing(
            "RIFF タイプが ACON ではありません".to_string(),
        ));
    }
    // riff_size はファイル先頭 8 バイトを除く残りサイズ。総量超過は弾く。
    let body_end = 8usize
        .checked_add(riff_size)
        .ok_or_else(|| AppError::ImageProcessing("RIFF サイズオーバーフロー".to_string()))?;
    let end = body_end.min(bytes.len());

    // ---- 第 1 パス: チャンクを収集 ----
    // anih が LIST より後に来ることは稀だが仕様上は許容されるため、
    // 2 パス構成にして af_icon フラグを確定させてから LIST を処理する。
    struct PendingList {
        offset: usize,
        data: Vec<u8>,
    }
    let mut anih: Option<AniHeader> = None;
    let mut rates: Option<Vec<u32>> = None;
    let mut seq: Option<Vec<u32>> = None;
    let mut pending_lists: Vec<PendingList> = Vec::new();

    let mut pos = 12;
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
            .ok_or_else(|| {
                AppError::ImageProcessing(format!(
                    "チャンク {:?} のサイズがファイル外: pos={} size={}",
                    std::str::from_utf8(id).unwrap_or("???"),
                    pos,
                    size
                ))
            })?;
        let data = &bytes[data_start..data_end];

        match id {
            b"anih" => anih = Some(parse_anih(data)?),
            b"rate" => rates = Some(parse_u32_array(data)),
            b"seq " => seq = Some(parse_u32_array(data)),
            b"LIST" => {
                if data.len() >= 4 && &data[0..4] == b"fram" {
                    pending_lists.push(PendingList {
                        offset: data_start + 4,
                        data: data[4..].to_vec(),
                    });
                }
                // 他の LIST (INFO など) は無視
            }
            _ => {}
        }

        // 次チャンクへ。サイズが奇数ならパディング 1 バイト。
        pos = data_end + (size & 1);
    }

    let header = anih
        .ok_or_else(|| AppError::ImageProcessing("'anih' チャンクが見つかりません".to_string()))?;

    // ---- 第 2 パス: af_icon フラグを使って LIST/fram を処理 ----
    let af_icon = (header.flags & AF_ICON) != 0;
    let mut frames: Vec<ParsedIcoCurEntry> = Vec::new();
    let mut frame_infos: Vec<AniFrameInfo> = Vec::new();
    for pl in &pending_lists {
        parse_frame_list(&pl.data, pl.offset, af_icon, &mut frames, &mut frame_infos)?;
    }
    let num_frames = if header.frames == 0 {
        frames.len() as u32
    } else {
        header.frames
    };
    let num_steps = if header.steps == 0 {
        num_frames
    } else {
        header.steps
    };
    let default_rate = if header.jif_rate == 0 {
        1
    } else {
        header.jif_rate
    };

    let sequence = seq.unwrap_or_else(|| (0..num_frames).collect());
    let per_step_rate_jiffies = match rates {
        Some(r) => r,
        None => vec![default_rate; num_steps as usize],
    };

    Ok(ParsedAni {
        num_frames,
        num_steps,
        default_rate_jiffies: default_rate,
        per_step_rate_jiffies,
        sequence,
        frames,
        frame_infos,
        is_legacy_raw_dib: (header.flags & AF_ICON) == 0,
    })
}

#[derive(Debug, Clone)]
struct AniHeader {
    frames: u32,
    steps: u32,
    jif_rate: u32,
    /// anih の生フラグ値。AF_ICON / raw DIB の判定に使用する。
    flags: u32,
}

fn parse_anih(data: &[u8]) -> AppResult<AniHeader> {
    if data.len() < 36 {
        return Err(AppError::ImageProcessing(
            "anih チャンクが 36 バイトに満たない".to_string(),
        ));
    }
    let frames = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let steps = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    // cx (12), cy (16), cBitCount (20), cPlanes (24) — 個別フレーム参照のため未使用
    let jif_rate = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);
    let flags = u32::from_le_bytes([data[32], data[33], data[34], data[35]]);
    Ok(AniHeader {
        frames,
        steps,
        jif_rate,
        flags,
    })
}

fn parse_u32_array(data: &[u8]) -> Vec<u32> {
    data.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// `data`: LIST チャンクの 'fram' 識別子直後のバイト列 (icon chunks のみ)。
/// `list_offset_in_file`: ファイル全体における `data` の先頭バイト位置。
/// `af_icon`: anih.flags に AF_ICON が立っていれば true (新形式 CUR)、false なら raw DIB (旧形式)。
/// フレームごとの `raw_data_range` はこのオフセットを基準に計算する。
fn parse_frame_list(
    data: &[u8],
    list_offset_in_file: usize,
    af_icon: bool,
    frames: &mut Vec<ParsedIcoCurEntry>,
    infos: &mut Vec<AniFrameInfo>,
) -> AppResult<()> {
    let mut pos = 0;
    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
            as usize;
        let start = pos + 8;
        let end = start
            .checked_add(size)
            .filter(|&e| e <= data.len())
            .ok_or_else(|| {
                AppError::ImageProcessing("LIST/fram 内のチャンクが切り詰め".to_string())
            })?;
        if id == b"icon" {
            if af_icon {
                // 新形式: icon チャンクは丸ごと CUR/ICO ファイル
                let parsed = parse_ico_cur(&data[start..end])?;
                // 各フレームは複数解像度を持つ可能性があるが、最大解像度のみ採用
                if let Some(largest) = parsed
                    .entries
                    .into_iter()
                    .max_by_key(|e| e.width * e.height)
                {
                    // ファイル全体における icon chunk データ部のバイト範囲を記録する
                    frames.push(largest);
                    infos.push(AniFrameInfo {
                        raw_data_range: (list_offset_in_file + start)..(list_offset_in_file + end),
                        format: AniFrameFormat::AfIcon,
                    });
                }
            } else {
                // 旧形式: icon chunk のデータは裸の BITMAPINFOHEADER + DIB ピクセル
                let (entry, bit_count) = parse_raw_dib_frame(&data[start..end])?;
                frames.push(entry);
                infos.push(AniFrameInfo {
                    raw_data_range: (list_offset_in_file + start)..(list_offset_in_file + end),
                    format: AniFrameFormat::RawDib { bit_count },
                });
            }
        }
        pos = end + (size & 1);
    }
    Ok(())
}

/// 旧形式 .ani の icon chunk (= 裸の BITMAPINFOHEADER + ピクセル) をデコードする。
/// 戻り値の `ParsedIcoCurEntry` は hotspot=(0,0) で埋める (raw DIB は持たない)。
/// 現在は 32bpp のみ対応。他の bit_count は明示的にエラーを返す。
fn parse_raw_dib_frame(data: &[u8]) -> AppResult<(ParsedIcoCurEntry, u16)> {
    if data.len() < 40 {
        return Err(AppError::ImageProcessing(
            "raw DIB が BITMAPINFOHEADER 40 バイトを満たさない".to_string(),
        ));
    }
    let bi_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if bi_size < 40 {
        return Err(AppError::ImageProcessing(format!(
            "biSize が小さすぎる: {}",
            bi_size
        )));
    }
    let width = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let height_raw = i32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let bit_count = u16::from_le_bytes([data[14], data[15]]);

    if width <= 0 || width > 1024 {
        return Err(AppError::ImageProcessing(format!(
            "raw DIB の幅が異常: {}",
            width
        )));
    }
    // bottom-up DIB: height_raw > 0 の場合は行順が下から上。
    // AND マスク付き形式では height_raw == width * 2 になることがある (カーソル標準)。
    let height_abs = if height_raw < 0 {
        (-height_raw) as u32
    } else {
        let h = height_raw as u32;
        let w = width as u32;
        // AND マスク付きの場合 h == w*2 → 実画像高さは w
        if h == w * 2 {
            w
        } else {
            h
        }
    };

    let width_u = width as u32;
    let height = height_abs;

    if bit_count != 32 {
        return Err(AppError::ImageProcessing(format!(
            "raw DIB の bit_count={} は未対応 (32bpp のみ対応)",
            bit_count
        )));
    }

    let row_bytes = (width_u * 4) as usize;
    let pixel_start = bi_size as usize;
    let needed = pixel_start + row_bytes * height as usize;
    if data.len() < needed {
        return Err(AppError::ImageProcessing(
            "raw DIB のピクセル領域が切り詰め".to_string(),
        ));
    }

    let mut img = image::RgbaImage::new(width_u, height);
    for y in 0..height {
        // bottom-up: ファイル先頭の行が画像の最下行
        let src_y = height - 1 - y;
        let row_start = pixel_start + (src_y as usize) * row_bytes;
        for x in 0..width_u {
            let i = row_start + (x as usize) * 4;
            let b = data[i];
            let g = data[i + 1];
            let r = data[i + 2];
            let a = data[i + 3];
            img.put_pixel(x, y, image::Rgba([r, g, b, a]));
        }
    }

    Ok((
        ParsedIcoCurEntry {
            width: width_u,
            height,
            hotspot_x: 0,
            hotspot_y: 0,
            image: img,
        },
        bit_count,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::cur_build::generate_cur_binary;
    use super::*;
    use image::RgbaImage;

    /// 合成 ANI を組み立てて parse_ani で読み戻せることを確認する。
    #[test]
    fn parse_ani_extracts_frames_with_default_rate() {
        // 2 フレーム、各フレームは 32x32 単色 CUR
        let img1 = RgbaImage::from_pixel(32, 32, image::Rgba([255, 0, 0, 255]));
        let img2 = RgbaImage::from_pixel(32, 32, image::Rgba([0, 255, 0, 255]));
        let cur1 = generate_cur_binary(&[(img1, 1, 2)]).unwrap();
        let cur2 = generate_cur_binary(&[(img2, 3, 4)]).unwrap();

        let mut ani = Vec::new();
        ani.extend_from_slice(b"RIFF");
        let body_size_placeholder_pos = ani.len();
        ani.extend_from_slice(&0u32.to_le_bytes()); // 後で書き戻す
        ani.extend_from_slice(b"ACON");

        // anih チャンク (36 byte data)
        ani.extend_from_slice(b"anih");
        ani.extend_from_slice(&36u32.to_le_bytes());
        let mut header = vec![0u8; 36];
        header[0..4].copy_from_slice(&36u32.to_le_bytes()); // cbSizeof
        header[4..8].copy_from_slice(&2u32.to_le_bytes()); // cFrames
        header[8..12].copy_from_slice(&2u32.to_le_bytes()); // cSteps
                                                            // cx, cy, cBitCount, cPlanes は省略可
        header[28..32].copy_from_slice(&6u32.to_le_bytes()); // jifRate = 6 (= 100ms)
        header[32..36].copy_from_slice(&0x01u32.to_le_bytes()); // flags = AF_ICON
        ani.extend_from_slice(&header);

        // LIST 'fram' { icon, icon }
        ani.extend_from_slice(b"LIST");
        let list_size = 4 + (8 + cur1.len()) + (8 + cur2.len());
        ani.extend_from_slice(&(list_size as u32).to_le_bytes());
        ani.extend_from_slice(b"fram");
        ani.extend_from_slice(b"icon");
        ani.extend_from_slice(&(cur1.len() as u32).to_le_bytes());
        ani.extend_from_slice(&cur1);
        ani.extend_from_slice(b"icon");
        ani.extend_from_slice(&(cur2.len() as u32).to_le_bytes());
        ani.extend_from_slice(&cur2);

        // RIFF サイズ (先頭 8 バイトを除く残りバイト数) を書き戻す
        let body_size = (ani.len() - 8) as u32;
        ani[body_size_placeholder_pos..body_size_placeholder_pos + 4]
            .copy_from_slice(&body_size.to_le_bytes());

        let parsed = parse_ani(&ani).expect("parse");
        assert_eq!(parsed.num_frames, 2);
        assert_eq!(parsed.num_steps, 2);
        assert_eq!(parsed.default_rate_jiffies, 6);
        assert_eq!(parsed.frames.len(), 2);
        // 'rate' チャンクなしなので default_rate_jiffies で num_steps 個埋めるはず
        assert_eq!(parsed.per_step_rate_jiffies, vec![6, 6]);
        // 'seq ' なしなので 0..num_frames を返す
        assert_eq!(parsed.sequence, vec![0, 1]);
        // 1 jiffy = 1/60s ≒ 16.66ms。6 jiffies × 2 = 200ms
        assert_eq!(parsed.total_duration_ms(), 200);
        // フレーム内容
        assert_eq!(
            parsed.frames[0].image.get_pixel(0, 0),
            &image::Rgba([255, 0, 0, 255])
        );
        assert_eq!(
            parsed.frames[1].image.get_pixel(0, 0),
            &image::Rgba([0, 255, 0, 255])
        );
        // frame_infos の検証
        assert_eq!(parsed.frame_infos.len(), 2);
        assert_eq!(parsed.frame_infos[0].format, AniFrameFormat::AfIcon);
        assert!(!parsed.is_legacy_raw_dib);
        // raw_data_range が元バイト列の対応する icon chunk データ部を正確に指していることを検証
        let icon1_start = 76;
        let icon1_end = icon1_start + cur1.len();
        assert_eq!(parsed.frame_infos[0].raw_data_range, icon1_start..icon1_end);
        // 1 フレーム目の範囲を切り出すと cur1 のバイト列と完全一致する
        assert_eq!(
            &ani[parsed.frame_infos[0].raw_data_range.clone()],
            cur1.as_slice()
        );
        // 2 フレーム目の範囲も cur2 と一致
        assert_eq!(
            &ani[parsed.frame_infos[1].raw_data_range.clone()],
            cur2.as_slice()
        );
    }

    #[test]
    fn parse_ani_rejects_non_riff_input() {
        let err = parse_ani(b"not a riff file").unwrap_err();
        assert!(matches!(err, AppError::ImageProcessing(_)));
    }

    #[test]
    fn parse_ani_rejects_riff_without_acon() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVE"); // ACON ではない
        let err = parse_ani(&bytes).unwrap_err();
        assert!(matches!(err, AppError::ImageProcessing(_)));
    }

    /// 旧形式 (raw DIB, flags=0) の .ani を組み立てて parse_ani が成功することを確認する。
    /// DIB は 8x8 の RGBA 4bpp 風 (ここでは 32bpp 単色で簡易化) で、anih.cx/cy を持つ。
    #[test]
    fn parse_legacy_raw_dib_frames() {
        // 32bpp BMP DIB: BITMAPINFOHEADER (40 bytes) + 8*8*4 = 256 bytes pixel data
        let mut dib = Vec::new();
        // BITMAPINFOHEADER
        dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
        dib.extend_from_slice(&8i32.to_le_bytes()); // biWidth = 8
        dib.extend_from_slice(&8i32.to_le_bytes()); // biHeight = 8 (bottom-up)
        dib.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
        dib.extend_from_slice(&32u16.to_le_bytes()); // biBitCount = 32
        dib.extend_from_slice(&0u32.to_le_bytes()); // biCompression = BI_RGB
        dib.extend_from_slice(&256u32.to_le_bytes()); // biSizeImage
        dib.extend_from_slice(&0i32.to_le_bytes()); // biXPelsPerMeter
        dib.extend_from_slice(&0i32.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        // 8x8 = 64 BGRA pixels (B, G, R, A)
        for _ in 0..64 {
            dib.extend_from_slice(&[0x00, 0xFF, 0x00, 0xFF]); // green opaque
        }

        let mut ani = Vec::new();
        ani.extend_from_slice(b"RIFF");
        let body_pos = ani.len();
        ani.extend_from_slice(&0u32.to_le_bytes());
        ani.extend_from_slice(b"ACON");
        ani.extend_from_slice(b"anih");
        ani.extend_from_slice(&36u32.to_le_bytes());
        let mut header = vec![0u8; 36];
        header[0..4].copy_from_slice(&36u32.to_le_bytes());
        header[4..8].copy_from_slice(&1u32.to_le_bytes()); // cFrames = 1
        header[8..12].copy_from_slice(&1u32.to_le_bytes()); // cSteps = 1
        header[12..16].copy_from_slice(&8u32.to_le_bytes()); // cx = 8
        header[16..20].copy_from_slice(&8u32.to_le_bytes()); // cy = 8
        header[20..24].copy_from_slice(&32u32.to_le_bytes()); // cBitCount = 32
        header[28..32].copy_from_slice(&3u32.to_le_bytes()); // jifRate
        header[32..36].copy_from_slice(&0u32.to_le_bytes()); // flags = 0 (NO AF_ICON)
        ani.extend_from_slice(&header);

        ani.extend_from_slice(b"LIST");
        let list_size = 4 + 8 + dib.len();
        ani.extend_from_slice(&(list_size as u32).to_le_bytes());
        ani.extend_from_slice(b"fram");
        ani.extend_from_slice(b"icon");
        ani.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        ani.extend_from_slice(&dib);

        let body = (ani.len() - 8) as u32;
        ani[body_pos..body_pos + 4].copy_from_slice(&body.to_le_bytes());

        let parsed = parse_ani(&ani).expect("parse legacy ani");
        assert!(parsed.is_legacy_raw_dib);
        assert_eq!(parsed.frames.len(), 1);
        let f = &parsed.frames[0];
        assert_eq!(f.width, 8);
        assert_eq!(f.height, 8);
        // 中央付近のピクセルが green
        let px = f.image.get_pixel(4, 4);
        assert_eq!(px.0, [0x00, 0xFF, 0x00, 0xFF]);
        assert_eq!(parsed.frame_infos.len(), 1);
        match parsed.frame_infos[0].format {
            AniFrameFormat::RawDib { bit_count } => assert_eq!(bit_count, 32),
            _ => panic!("expected RawDib"),
        }
    }

    /// bottom-up DIB が縦方向に正しく反転されることを確認する。
    /// 最上段 (y=0) を赤、最下段 (y=h-1) を青にした場合、デコード後も同じになるはず。
    /// raw DIB はピクセルが「ファイル先頭 = 最下行」のため、ファイルの bytes 順とは逆になる。
    #[test]
    fn parse_raw_dib_bottom_up_orientation() {
        let w = 4u32;
        let h = 4u32;
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&(w as i32).to_le_bytes());
        dib.extend_from_slice(&(h as i32).to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&32u16.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&(w * h * 4).to_le_bytes());
        dib.extend_from_slice(&[0; 16]);
        // 最下行 (ファイル順では最初) を青 (BGRA: B=FF, G=00, R=00, A=FF)、
        // それ以外を赤 (BGRA: B=00, G=00, R=FF, A=FF)
        for y in 0..h {
            let row_color = if y == 0 {
                [0xFF, 0x00, 0x00, 0xFF] // B,G,R,A => Blue
            } else {
                [0x00, 0x00, 0xFF, 0xFF] // B,G,R,A => Red
            };
            for _ in 0..w {
                dib.extend_from_slice(&row_color);
            }
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
        header[28..32].copy_from_slice(&1u32.to_le_bytes());
        // flags = 0 (NO AF_ICON) → raw DIB
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

        let parsed = parse_ani(&ani).expect("parse");
        let img = &parsed.frames[0].image;
        // y=h-1 (最下行 in image coordinates) は元の DIB y=0 (青) のはず
        assert_eq!(img.get_pixel(0, h - 1).0, [0x00, 0x00, 0xFF, 0xFF]);
        // y=0 (最上行) は元の DIB y=h-1 (赤) のはず
        assert_eq!(img.get_pixel(0, 0).0, [0xFF, 0x00, 0x00, 0xFF]);
    }

    #[test]
    fn parse_ani_uses_rate_chunk_when_present() {
        let img = RgbaImage::from_pixel(16, 16, image::Rgba([0, 0, 255, 255]));
        let cur = generate_cur_binary(&[(img, 0, 0)]).unwrap();

        let mut ani = Vec::new();
        ani.extend_from_slice(b"RIFF");
        let body_size_pos = ani.len();
        ani.extend_from_slice(&0u32.to_le_bytes());
        ani.extend_from_slice(b"ACON");

        // anih: 1 frame, 3 steps (シーケンスで使い回す)
        ani.extend_from_slice(b"anih");
        ani.extend_from_slice(&36u32.to_le_bytes());
        let mut header = vec![0u8; 36];
        header[0..4].copy_from_slice(&36u32.to_le_bytes());
        header[4..8].copy_from_slice(&1u32.to_le_bytes());
        header[8..12].copy_from_slice(&3u32.to_le_bytes());
        header[28..32].copy_from_slice(&3u32.to_le_bytes());
        header[32..36].copy_from_slice(&0x01u32.to_le_bytes());
        ani.extend_from_slice(&header);

        // rate チャンク: 3 ステップ分 [10, 20, 30]
        ani.extend_from_slice(b"rate");
        ani.extend_from_slice(&12u32.to_le_bytes());
        for v in [10u32, 20, 30] {
            ani.extend_from_slice(&v.to_le_bytes());
        }

        // LIST 'fram' { 1 icon }
        ani.extend_from_slice(b"LIST");
        let list_size = 4 + 8 + cur.len();
        ani.extend_from_slice(&(list_size as u32).to_le_bytes());
        ani.extend_from_slice(b"fram");
        ani.extend_from_slice(b"icon");
        ani.extend_from_slice(&(cur.len() as u32).to_le_bytes());
        ani.extend_from_slice(&cur);

        let body = (ani.len() - 8) as u32;
        ani[body_size_pos..body_size_pos + 4].copy_from_slice(&body.to_le_bytes());

        let parsed = parse_ani(&ani).expect("parse");
        assert_eq!(parsed.per_step_rate_jiffies, vec![10, 20, 30]);
        // (10 + 20 + 30) jiffies = 60 jiffies = 1 sec = 1000 ms
        assert_eq!(parsed.total_duration_ms(), 1000);
        // frame_infos の検証
        assert_eq!(parsed.frame_infos.len(), 1);
        assert!(!parsed.is_legacy_raw_dib);
    }
}
