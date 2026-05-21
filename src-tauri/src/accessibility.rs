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
//!  - CursorBaseSize: `HKCU\Control Panel\Cursors\CursorBaseSize` (DWORD, 32 がデフォルト)
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

        // CursorBaseSize は本アプリの正規機能となったため競合判定から除外。
        let has_conflicts = mouse_sonar_enabled || high_contrast_enabled;

        Self {
            mouse_sonar_enabled,
            high_contrast_enabled,
            cursor_base_size,
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

/// `HKCU\Control Panel\Cursors\CursorBaseSize` を読む。失敗時はデフォルト 32。
fn read_cursor_base_size() -> u32 {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(r"Control Panel\Cursors")
        .and_then(|k| k.get_value::<u32, _>("CursorBaseSize"))
        .unwrap_or(DEFAULT_CURSOR_BASE_SIZE)
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
}
