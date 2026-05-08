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

    let mut anih: Option<AniHeader> = None;
    let mut rates: Option<Vec<u32>> = None;
    let mut seq: Option<Vec<u32>> = None;
    let mut frames: Vec<ParsedIcoCurEntry> = Vec::new();

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
            b"anih" => {
                anih = Some(parse_anih(data)?);
            }
            b"rate" => {
                rates = Some(parse_u32_array(data));
            }
            b"seq " => {
                seq = Some(parse_u32_array(data));
            }
            b"LIST" => {
                if data.len() >= 4 && &data[0..4] == b"fram" {
                    parse_frame_list(&data[4..], &mut frames)?;
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
    })
}

#[derive(Debug, Clone)]
struct AniHeader {
    frames: u32,
    steps: u32,
    jif_rate: u32,
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
    // AF_ICON (bit 0) が立っていなければ raw DIB エントリの可能性あり (旧形式)
    if flags & 0x01 == 0 {
        tracing::warn!("ANI フラグに AF_ICON が立っていません — raw DIB フレームは未対応");
    }
    Ok(AniHeader {
        frames,
        steps,
        jif_rate,
    })
}

fn parse_u32_array(data: &[u8]) -> Vec<u32> {
    data.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn parse_frame_list(data: &[u8], frames: &mut Vec<ParsedIcoCurEntry>) -> AppResult<()> {
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
            // icon チャンクは丸ごと CUR/ICO ファイル
            let parsed = parse_ico_cur(&data[start..end])?;
            // 各フレームは複数解像度を持つ可能性があるが、最大解像度のみ採用
            if let Some(largest) = parsed
                .entries
                .into_iter()
                .max_by_key(|e| e.width * e.height)
            {
                frames.push(largest);
            }
        }
        pos = end + (size & 1);
    }
    Ok(())
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
    }
}
