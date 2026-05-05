//! グローバルホットキー登録 (Phase 4-6)
//!
//! Win32 `RegisterHotKey` でシステム全体のホットキーを購読する。
//! デフォルトは `Ctrl+Alt+Shift+R` (パニックリセット)。
//!
//! `cursor_watcher.rs` と同じくバックグラウンドスレッドで不可視ウィンドウを作り、
//! メッセージループで `WM_HOTKEY` を受け取ってコールバックを呼ぶ。
//!
//! プロセスが落ちたら OS がレジスタを解放するため、明示的な停止機構は持たない。

use crate::errors::{AppError, AppResult};

/// パニックホットキー受信時に呼ばれるコールバック (グローバル)
#[cfg(windows)]
static HOTKEY_CALLBACK: std::sync::Mutex<Option<Box<dyn Fn() + Send>>> =
    std::sync::Mutex::new(None);

/// このアプリ内で重複しない任意の ID。`RegisterHotKey` 呼び出しのトークン。
#[cfg(windows)]
const PANIC_HOTKEY_ID: i32 = 0xCF1;

/// ホットキー文字列を `(modifiers, vk)` ペアに分解する。
///
/// 受け付ける形式: `Ctrl+Alt+Shift+R` のように `+` 区切り。大文字小文字区別なし。
/// 修飾子: `Ctrl` `Alt` `Shift` `Win` (`Meta` も `Win` 扱い)
/// 主キー: 単一の英大文字 (A-Z) または F1-F24
///
/// 不正な形式は `None` を返す。
pub fn parse_hotkey(spec: &str) -> Option<(u32, u32)> {
    const MOD_ALT: u32 = 0x0001;
    const MOD_CONTROL: u32 = 0x0002;
    const MOD_SHIFT: u32 = 0x0004;
    const MOD_WIN: u32 = 0x0008;
    const MOD_NOREPEAT: u32 = 0x4000;

    let parts: Vec<&str> = spec.split('+').map(|p| p.trim()).collect();
    if parts.len() < 2 {
        return None;
    }
    let mut modifiers: u32 = MOD_NOREPEAT;
    let mut vk: Option<u32> = None;

    for part in &parts {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "alt" => modifiers |= MOD_ALT,
            "shift" => modifiers |= MOD_SHIFT,
            "win" | "meta" | "super" | "cmd" => modifiers |= MOD_WIN,
            other => {
                // 単一英字
                if other.len() == 1 {
                    let c = other.chars().next().unwrap();
                    if c.is_ascii_alphabetic() {
                        vk = Some(c.to_ascii_uppercase() as u32);
                        continue;
                    }
                    if c.is_ascii_digit() {
                        // 数字キー (VK_0=0x30 .. VK_9=0x39)
                        vk = Some(c as u32);
                        continue;
                    }
                }
                // F1〜F24 (VK_F1=0x70)
                if let Some(rest) = other.strip_prefix('f') {
                    if let Ok(n) = rest.parse::<u32>() {
                        if (1..=24).contains(&n) {
                            vk = Some(0x70 + n - 1);
                            continue;
                        }
                    }
                }
                return None;
            }
        }
    }
    let vk = vk?;
    // 修飾子なしの裸キーは登録不可とする (誤爆防止)
    if modifiers == MOD_NOREPEAT {
        return None;
    }
    Some((modifiers, vk))
}

/// パニックホットキーを登録し、押下時に `callback` を呼ぶ。
///
/// `spec` は `parse_hotkey` で解釈する。失敗ケース:
///   - 不正な形式 → `AppError::InvalidInput`
///   - 既に他アプリが同じ組合せを取得済み → `AppError::Other` (RegisterHotKey 失敗)
#[cfg(windows)]
pub fn register_panic_hotkey<F>(spec: &str, callback: F) -> AppResult<()>
where
    F: Fn() + Send + 'static,
{
    let (modifiers, vk) = parse_hotkey(spec).ok_or_else(|| {
        AppError::InvalidInput(format!("ホットキー文字列を解釈できません: {}", spec))
    })?;

    let spec_owned = spec.to_string();
    HOTKEY_CALLBACK.lock().unwrap().replace(Box::new(callback));

    std::thread::Builder::new()
        .name("easycursorswap-hotkey".to_string())
        .spawn(move || unsafe {
            use windows::core::*;
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
            };
            use windows::Win32::UI::WindowsAndMessaging::*;

            let class_name = w!("EasyCursorSwapHotkey");
            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(hotkey_wnd_proc),
                lpszClassName: class_name,
                ..Default::default()
            };
            RegisterClassW(&wnd_class);

            let hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("EasyCursorSwap Hotkey"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            ) {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("hotkey ウィンドウ作成失敗: {}", e);
                    return;
                }
            };

            if let Err(e) = RegisterHotKey(
                Some(hwnd),
                PANIC_HOTKEY_ID,
                HOT_KEY_MODIFIERS(modifiers),
                vk,
            ) {
                tracing::error!(
                    "RegisterHotKey 失敗 (おそらく他アプリが {} を保持中): {}",
                    spec_owned,
                    e
                );
                let _ = DestroyWindow(hwnd);
                return;
            }
            tracing::info!("パニックホットキーを登録: {}", spec_owned);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, Some(hwnd), 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // ループ抜け = WM_QUIT (現状到達経路はないが念のため)
            let _ = UnregisterHotKey(Some(hwnd), PANIC_HOTKEY_ID);
        })
        .map_err(|e| AppError::Other(format!("hotkey スレッド起動失敗: {}", e)))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn register_panic_hotkey<F>(_spec: &str, _callback: F) -> AppResult<()>
where
    F: Fn() + Send + 'static,
{
    tracing::warn!("グローバルホットキーは Windows 以外では利用できません");
    Ok(())
}

#[cfg(windows)]
unsafe extern "system" fn hotkey_wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;

    if msg == WM_HOTKEY && wparam.0 as i32 == PANIC_HOTKEY_ID {
        tracing::info!("パニックホットキー押下を検知");
        if let Ok(cb) = HOTKEY_CALLBACK.lock() {
            if let Some(ref callback) = *cb {
                callback();
            }
        }
        return windows::Win32::Foundation::LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOD_ALT: u32 = 0x0001;
    const MOD_CONTROL: u32 = 0x0002;
    const MOD_SHIFT: u32 = 0x0004;
    const MOD_WIN: u32 = 0x0008;
    const MOD_NOREPEAT: u32 = 0x4000;

    #[test]
    fn parse_default_panic_hotkey() {
        let (m, vk) = parse_hotkey("Ctrl+Alt+Shift+R").unwrap();
        assert_eq!(
            m,
            MOD_NOREPEAT | MOD_CONTROL | MOD_ALT | MOD_SHIFT,
            "Ctrl+Alt+Shift の修飾子が必要"
        );
        assert_eq!(vk, 0x52, "VK_R = 0x52");
    }

    #[test]
    fn parse_is_case_insensitive() {
        let (_m, vk) = parse_hotkey("ctrl+alt+shift+r").unwrap();
        assert_eq!(vk, 0x52);
        let (_m2, vk2) = parse_hotkey("CTRL+ALT+SHIFT+R").unwrap();
        assert_eq!(vk2, 0x52);
    }

    #[test]
    fn parse_supports_function_keys() {
        let (m, vk) = parse_hotkey("Win+F12").unwrap();
        assert_eq!(m, MOD_NOREPEAT | MOD_WIN);
        assert_eq!(vk, 0x70 + 11); // VK_F12
    }

    #[test]
    fn parse_supports_digits() {
        let (_m, vk) = parse_hotkey("Ctrl+5").unwrap();
        assert_eq!(vk, 0x35);
    }

    #[test]
    fn parse_rejects_modifier_only() {
        // 修飾子だけはダメ
        assert!(parse_hotkey("Ctrl+Alt").is_none());
    }

    #[test]
    fn parse_rejects_naked_key() {
        // 単キーだけ (誤爆防止) もダメ
        assert!(parse_hotkey("R").is_none());
    }

    #[test]
    fn parse_rejects_unknown_token() {
        assert!(parse_hotkey("Ctrl+Foobar").is_none());
        assert!(parse_hotkey("").is_none());
        assert!(parse_hotkey("F25").is_none()); // F24 まで
    }
}
