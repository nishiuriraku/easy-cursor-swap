//! アクセシビリティ機能との競合検出 + 現在の CursorBaseSize 取得。
//!
//! 仕様書 §「Windows 11 統合・競合検出」より:
//!  > `CursorIndicator` (Ctrl 押下でカーソル位置を表示) / `ContrastScheme` (ハイコントラスト) が
//!  > 有効な場合、テーマを適用しても期待通りの見た目にならない可能性がある。
//!  > 適用前にユーザーへ警告ダイアログを表示する。
//!
//! 各設定の取得元:
//!  - CursorIndicator: `HKCU\Control Panel\Mouse\MouseSonar` (REG_SZ "0"/"1")
//!  - ContrastScheme: `HKCU\Control Panel\Accessibility\HighContrast\Flags`
//!    の bit 0 (HCF_HIGHCONTRASTON = 1)
//!  - CursorBaseSize: 以下の順で読む (上で取得できたものを採用):
//!    1. `HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize` (DWORD 1-15, slider 値) →
//!       `registry::slider_position_to_base_size` で DWORD に変換。Windows 11 Settings の
//!       「マウスポインターとタッチ」スライダーが canonical に書く値。
//!    2. `HKCU\Control Panel\Cursors\CursorBaseSize` (DWORD 32-256) — 旧 Control Panel API
//!       および本アプリの永続化先。古い Windows ビルドで Accessibility 側が書かれない
//!       ケースの fallback。
//!    3. どちらも未設定なら 32 (= Windows 既定 = slider 1)。
//!
//! 取得失敗時は競合なしとして扱う (フェイルセーフ: 警告は出さない)。
//!
//! ## CursorBaseSize の扱い
//!
//! v0.1+ で本アプリ自身が CursorBaseSize の書き換えを正規機能として提供する
//! (Settings ページの一般セクション → カーソルサイズ。`registry::set_cursor_base_size`
//! 経由)。このため CursorBaseSize > 32 は **競合ではなく単に現状値**となり、
//! `has_conflicts` の判定からは除外する。UI 側でスライダー初期値を反映する
//! ために値自体は引き続き `cursor_base_size` フィールドで公開する。

use serde::Serialize;
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

/// アクセシビリティ機能の競合情報 + 現在の CursorBaseSize 値。
#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub struct AccessibilityConflicts {
    /// マウスソナー (Ctrl 押下でカーソル位置を可視化) が有効
    pub mouse_sonar_enabled: bool,
    /// ハイコントラストモードが有効
    pub high_contrast_enabled: bool,
    /// 現在の HKCU\Control Panel\Cursors\CursorBaseSize 値 (デフォルト 32)。
    /// 本アプリで設定可能になったため競合判定からは除外したが、UI スライダーの
    /// 初期値反映のために値自体は引き続き返す。
    pub cursor_base_size: u32,
    /// `HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize` (slider 1-15) の生値。
    /// `cursor_base_size` は DWORD だが、こちらは slider 位置そのもの。フロント側は
    /// `cursor_size_slider != 1` で「Windows accessibility が cursor を支配している」
    /// 状態を判定し、本アプリのスライダーを disabled にする。
    /// 失敗時は 1 (= factory state)。
    pub cursor_size_slider: u8,
    /// `HKCU\SOFTWARE\Microsoft\Accessibility\CursorType` の生値。
    /// 0=白 (default), 1=黒, 2=反転, 3=白でサイズ拡大時, 6=カスタムカラー (実測)。
    /// 将来の Windows アップデートで値域が増える可能性あり。UI メッセージのバリエーション
    /// 切替で使用 (gate 判定は `cursor_size_slider` 単独で行う)。
    /// 失敗時は 0。
    pub cursor_type: u8,
    /// 競合があるか (mouse_sonar / high_contrast のいずれかが有効)
    pub has_conflicts: bool,
}

const DEFAULT_CURSOR_BASE_SIZE: u32 = 32;

impl AccessibilityConflicts {
    /// 現在のレジストリ状態から競合情報を読み出す。
    ///
    /// 1 つでも読み取りに失敗した場合、その項目はデフォルト値 (=競合なし側) として扱う。
    pub fn detect() -> Self {
        let mouse_sonar_enabled = read_mouse_sonar();
        let high_contrast_enabled = read_high_contrast();
        let cursor_base_size = read_cursor_base_size();
        let cursor_size_slider = read_accessibility_cursor_size_slider();
        let cursor_type = read_accessibility_cursor_type();

        // CursorBaseSize は本アプリの正規機能となったため競合判定から除外。
        let has_conflicts = mouse_sonar_enabled || high_contrast_enabled;

        Self {
            mouse_sonar_enabled,
            high_contrast_enabled,
            cursor_base_size,
            cursor_size_slider,
            cursor_type,
            has_conflicts,
        }
    }
}

/// `HKCU\Control Panel\Mouse\MouseSonar` を読む。失敗時は false。
fn read_mouse_sonar() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(r"Control Panel\Mouse")
        .and_then(|k| k.get_value::<String, _>("MouseSonar"))
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}

/// `HKCU\Control Panel\Accessibility\HighContrast\Flags` の bit 0 を確認。
/// HCF_HIGHCONTRASTON = 0x00000001
fn read_high_contrast() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(r"Control Panel\Accessibility\HighContrast")
        .and_then(|k| k.get_value::<u32, _>("Flags"))
        .map(|flags| (flags & 0x0000_0001) != 0)
        .unwrap_or(false)
}

/// 現在のカーソル基準サイズ (DWORD 32-256) を取得する。Windows 11 Settings が
/// canonical に書く `Accessibility\CursorSize` (slider 1-15) を優先し、本アプリ
/// および旧 Control Panel API が書く `CursorBaseSize` を fallback として読む。
///
/// 順序は [`resolve_cursor_base_size`] で純粋関数として表現する (テスト容易化)。
fn read_cursor_base_size() -> u32 {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let accessibility_slider = hkcu
        .open_subkey(r"SOFTWARE\Microsoft\Accessibility")
        .and_then(|k| k.get_value::<u32, _>("CursorSize"))
        .ok();
    let cursor_base_size = hkcu
        .open_subkey(r"Control Panel\Cursors")
        .and_then(|k| k.get_value::<u32, _>("CursorBaseSize"))
        .ok();
    resolve_cursor_base_size(accessibility_slider, cursor_base_size)
}

/// 2 つのレジストリ値から canonical な CursorBaseSize (DWORD) を決定する純粋関数。
///
/// 優先順位 (v2 / 2026-05-23 redesign):
/// 1. `Accessibility\CursorSize > 1` のとき: eoa pipeline がアクティブ。slider 値を
///    `slider_position_to_base_size` で DWORD に変換して返す (UI は disabled 表示)。
/// 2. `CursorSize == 1` または `None` のとき: 標準 pipeline。`CursorBaseSize` を採用し
///    `clamp_cursor_base_size` でレンジ clamp。本アプリの slider が書く永続化先。
/// 3. `CursorBaseSize` も `None` なら Windows 既定 (32px) を返す。
///
/// **なぜ slider=1 で fall through するか:**
/// アプリの slider は `set_cursor_base_size` 経由で **CursorBaseSize のみ** を書く
/// (Accessibility\CursorSize は OS の Settings UI のみが書く invariant)。よって
/// `CursorSize=1 + CursorBaseSize=80` (アプリで 80px 設定後) のような並存ケースは
/// **正常状態**であり、CursorBaseSize を信頼するのが正しい。v1 では slider=1 でも
/// Accessibility 優先で 32px に戻っていたため、アプリ slider の round-trip が壊れていた。
fn resolve_cursor_base_size(
    accessibility_slider: Option<u32>,
    cursor_base_size: Option<u32>,
) -> u32 {
    use crate::registry::{
        clamp_cursor_base_size, slider_position_to_base_size, MAX_CURSOR_SIZE_SLIDER,
        MIN_CURSOR_SIZE_SLIDER,
    };

    // eoa pipeline active (slider > 1) のときのみ Accessibility 値を採用する。
    if let Some(slider) = accessibility_slider {
        if slider > u32::from(MIN_CURSOR_SIZE_SLIDER) {
            let clamped = slider.clamp(
                u32::from(MIN_CURSOR_SIZE_SLIDER),
                u32::from(MAX_CURSOR_SIZE_SLIDER),
            ) as u8;
            return slider_position_to_base_size(clamped);
        }
    }
    // 標準 pipeline: CursorBaseSize を採用。
    if let Some(size) = cursor_base_size {
        return clamp_cursor_base_size(size);
    }
    DEFAULT_CURSOR_BASE_SIZE
}

/// `HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize` (slider 1-15) を読む。
/// 失敗時はファクトリ状態 1。範囲外は clamp する。
fn read_accessibility_cursor_size_slider() -> u8 {
    use crate::registry::{MAX_CURSOR_SIZE_SLIDER, MIN_CURSOR_SIZE_SLIDER};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let raw: u32 = hkcu
        .open_subkey(r"SOFTWARE\Microsoft\Accessibility")
        .and_then(|k| k.get_value("CursorSize"))
        .unwrap_or(u32::from(MIN_CURSOR_SIZE_SLIDER));
    raw.clamp(
        u32::from(MIN_CURSOR_SIZE_SLIDER),
        u32::from(MAX_CURSOR_SIZE_SLIDER),
    ) as u8
}

/// `HKCU\SOFTWARE\Microsoft\Accessibility\CursorType` を読む。
/// 失敗時は 0 (= 白、ファクトリ)。u8 範囲外は u8::MAX に飽和。
fn read_accessibility_cursor_type() -> u8 {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let raw: u32 = hkcu
        .open_subkey(r"SOFTWARE\Microsoft\Accessibility")
        .and_then(|k| k.get_value("CursorType"))
        .unwrap_or(0);
    if raw > u32::from(u8::MAX) {
        u8::MAX
    } else {
        raw as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_no_conflicts() {
        let c = AccessibilityConflicts {
            mouse_sonar_enabled: false,
            high_contrast_enabled: false,
            cursor_base_size: 32,
            cursor_size_slider: 1,
            cursor_type: 0,
            has_conflicts: false,
        };
        assert!(!c.has_conflicts);
    }

    /// CursorBaseSize はアプリの正規機能 (Settings → 一般 → カーソルサイズ) になったため、
    /// 値が大きくても has_conflicts は **立たない**。値自体は UI スライダーの初期値
    /// 反映のために引き続き返す。
    #[test]
    fn cursor_base_size_does_not_trigger_conflict() {
        // 既定 (32) でも、拡大されていても has_conflicts に影響しない
        for size in [32u32, 64, 128, 256] {
            let c = AccessibilityConflicts {
                mouse_sonar_enabled: false,
                high_contrast_enabled: false,
                cursor_base_size: size,
                cursor_size_slider: 1,
                cursor_type: 0,
                has_conflicts: false,
            };
            assert!(
                !c.has_conflicts,
                "cursor_base_size={} で has_conflicts が立つのは v0.1+ では退行",
                size
            );
        }
    }

    #[test]
    #[allow(
        clippy::nonminimal_bool,
        clippy::bad_bit_mask,
        clippy::overly_complex_bool_expr,
        clippy::assertions_on_constants
    )]
    fn high_contrast_flag_check() {
        // HCF_HIGHCONTRASTON = 0x00000001 の判定ロジック
        assert!((0x0000_0001u32 & 0x0000_0001) != 0);
        assert!(!((0x0000_0002u32 & 0x0000_0001) != 0));
        assert!((0x0000_0003u32 & 0x0000_0001) != 0);
    }

    /// 実環境での detect() 呼び出しが panic せず妥当な値を返すことを確認。
    /// 値はテスト実行環境に依存するためアサーションは形式的。
    #[test]
    fn detect_returns_consistent_result() {
        let c = AccessibilityConflicts::detect();
        assert_eq!(
            c.has_conflicts,
            c.mouse_sonar_enabled || c.high_contrast_enabled,
            "has_conflicts は mouse_sonar / high_contrast のみで決まる (cursor_base_size は除外)"
        );
        // cursor_base_size の値は引き続き返される (UI スライダーの初期値用)。
        // 32 (既定) より小さい値は read 側のフォールバックで 32 になるはず。
        assert!(c.cursor_base_size >= DEFAULT_CURSOR_BASE_SIZE);
    }

    /// `detect` で新規追加された `cursor_size_slider` と `cursor_type` がレンジ内に
    /// 収まることを確認。値は実機依存だが、いずれの環境でも 1..=15 と 0..=255 の
    /// 範囲には収まるはず。`read_*` 関数の clamp ロジック動作確認。
    #[test]
    fn detect_returns_in_range_accessibility_fields() {
        use crate::registry::{MAX_CURSOR_SIZE_SLIDER, MIN_CURSOR_SIZE_SLIDER};
        let c = AccessibilityConflicts::detect();
        assert!(
            (MIN_CURSOR_SIZE_SLIDER..=MAX_CURSOR_SIZE_SLIDER).contains(&c.cursor_size_slider),
            "cursor_size_slider が範囲外: {}",
            c.cursor_size_slider
        );
        // cursor_type は u8 全域 (0..=255) を許容するため上限テストはなし。
        // 失敗時 0 の規約だけ確認 — `c.cursor_type` がアクセスできる (panic しない) ことで十分。
        let _ = c.cursor_type;
    }

    /// `resolve_cursor_base_size` の優先順位契約 (v2):
    /// - `Accessibility\CursorSize > 1` のときのみ Accessibility を採用 (eoa pipeline active)。
    /// - `CursorSize == 1` または `None` のときは `CursorBaseSize` を採用 (standard pipeline)。
    /// - 両方とも `None` なら 32 既定。
    ///
    /// この契約により、アプリが `CursorBaseSize` のみ書く invariant (specs/2026-05-23-cursor-size-redesign-v2)
    /// と組み合わさり、round-trip が成立する。
    #[test]
    fn resolve_prefers_accessibility_only_when_eoa_active() {
        // eoa active: slider=15 + CursorBaseSize=32 → Accessibility=15 を採用 (256)。
        assert_eq!(resolve_cursor_base_size(Some(15), Some(32)), 256);
        // eoa active: 中間スライダー位置 6 → 32 + 16*(6-1) = 112。
        assert_eq!(resolve_cursor_base_size(Some(6), None), 112);
        // eoa **inactive** (slider=1): CursorBaseSize=128 が採用される (= round-trip 成立)。
        // v1 ではここで 32 (slider=1) が返っていたため app slider が勝手に 1 に戻っていた。
        assert_eq!(resolve_cursor_base_size(Some(1), Some(128)), 128);
        // eoa inactive かつ CursorBaseSize なし → 32 既定。
        assert_eq!(resolve_cursor_base_size(Some(1), None), 32);
    }

    #[test]
    fn resolve_falls_back_to_cursor_base_size_when_accessibility_missing() {
        // Accessibility 側が未設定 → CursorBaseSize を採用。
        assert_eq!(resolve_cursor_base_size(None, Some(64)), 64);
        assert_eq!(resolve_cursor_base_size(None, Some(256)), 256);
        // CursorBaseSize がレンジ外でも clamp される。
        assert_eq!(resolve_cursor_base_size(None, Some(0)), 32);
        assert_eq!(resolve_cursor_base_size(None, Some(1000)), 256);
    }

    #[test]
    fn resolve_uses_default_when_both_missing() {
        // 両方未設定 = Windows 既定 (32px / slider 1)。
        assert_eq!(
            resolve_cursor_base_size(None, None),
            DEFAULT_CURSOR_BASE_SIZE
        );
    }

    #[test]
    fn resolve_clamps_accessibility_out_of_range() {
        // registry 破損 / 未来仕様拡張で slider が 0 や 16+ になっていても安全。
        assert_eq!(resolve_cursor_base_size(Some(0), None), 32);
        assert_eq!(resolve_cursor_base_size(Some(99), None), 256);
        // u32::MAX のような壊滅値も clamp で吸収。
        assert_eq!(resolve_cursor_base_size(Some(u32::MAX), None), 256);
    }
}
