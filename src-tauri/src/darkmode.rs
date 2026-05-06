//! EasyCursorSwap ダークモード監視モジュール
//!
//! Windows のダークモード設定を監視し、テーマの自動切替を行う。
//! WM_SETTINGCHANGE メッセージを購読して、レジストリ変更をリアルタイムに検知する。

use crate::errors::AppResult;

/// 現在のダークモード状態を取得する
///
/// HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize の
/// AppsUseLightTheme (DWORD: 0=ダーク / 1=ライト) を読み取る
pub fn is_dark_mode() -> AppResult<bool> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    match hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize") {
        Ok(key) => {
            let value: u32 = key.get_value("AppsUseLightTheme").unwrap_or(1);
            // 0 = ダークモード, 1 = ライトモード
            Ok(value == 0)
        }
        Err(_) => {
            // キーが存在しない場合はライトモードと見なす
            tracing::warn!(
                "ダークモード設定のレジストリキーが見つかりません。ライトモードを使用します。"
            );
            Ok(false)
        }
    }
}

/// ダークモード監視ループ（WM_SETTINGCHANGE 購読）
///
/// この関数は別スレッドで実行し、ダークモードの変更を検知したら
/// コールバックを呼び出す。
#[cfg(windows)]
pub fn start_dark_mode_watcher<F>(on_change: F) -> AppResult<()>
where
    F: Fn(bool) + Send + 'static,
{
    use windows::core::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    std::thread::spawn(move || {
        unsafe {
            // 不可視ウィンドウクラスを登録
            let class_name = w!("EasyCursorSwapDarkModeWatcher");
            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(dark_mode_wnd_proc),
                lpszClassName: class_name,
                ..Default::default()
            };
            RegisterClassW(&wnd_class);

            // 不可視のメッセージ専用ウィンドウを作成
            let _hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("EasyCursorSwap DarkMode Watcher"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE), // メッセージ専用ウィンドウ
                None,
                None,
                None,
            ) {
                Ok(hwnd) => hwnd,
                Err(e) => {
                    tracing::error!("ダークモード監視ウィンドウの作成に失敗: {}", e);
                    return;
                }
            };

            // コールバックをスタティックに保持
            // (簡易実装: グローバルコールバック)
            DARK_MODE_CALLBACK
                .lock()
                .unwrap()
                .replace(Box::new(on_change));

            tracing::info!("ダークモード監視を開始しました");

            // メッセージループ
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });

    Ok(())
}

#[cfg(not(windows))]
pub fn start_dark_mode_watcher<F>(_on_change: F) -> AppResult<()>
where
    F: Fn(bool) + Send + 'static,
{
    tracing::warn!("ダークモード監視は Windows 以外では利用できません");
    Ok(())
}

/// ダークモード変更コールバック（グローバル）
#[cfg(windows)]
type DarkModeCallback = Box<dyn Fn(bool) + Send>;
#[cfg(windows)]
static DARK_MODE_CALLBACK: std::sync::Mutex<Option<DarkModeCallback>> = std::sync::Mutex::new(None);

/// 不可視ウィンドウのメッセージプロシージャ
#[cfg(windows)]
unsafe extern "system" fn dark_mode_wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;

    // WM_SETTINGCHANGE (0x001A) を監視
    if msg == WM_SETTINGCHANGE {
        // lParam が "ImmersiveColorSet" かチェック
        let lparam_str = lparam.0 as *const u16;
        if !lparam_str.is_null() {
            let wide_str = windows::core::PCWSTR(lparam_str);
            if let Ok(s) = wide_str.to_string() {
                if s == "ImmersiveColorSet" {
                    tracing::debug!("ImmersiveColorSet 変更を検知");
                    // ダークモード状態を再読込
                    if let Ok(is_dark) = is_dark_mode() {
                        if let Ok(cb) = DARK_MODE_CALLBACK.lock() {
                            if let Some(ref callback) = *cb {
                                callback(is_dark);
                            }
                        }
                    }
                }
            }
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
