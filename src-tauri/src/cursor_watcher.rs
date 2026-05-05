//! 外部からのカーソル設定変更検知 (Phase 2-2)
//!
//! コントロールパネル / 他ツールが `HKCU\Control Panel\Cursors` を書き換えると、
//! Windows は `WM_SETTINGCHANGE` を `SPI_SETCURSORS` (= 0x0057) 番号でブロードキャストする。
//! それを購読して UI を再読み込みするためのコールバックを呼び出す。
//!
//! `darkmode.rs` と同じく不可視メッセージ専用ウィンドウで実装。
//!
//! 仕様書 §「OS 設定との状態同期」:
//!  > ユーザーがコントロールパネル「マウスのプロパティ」から外部でカーソルを変更した場合、
//!  > アプリ内部状態と実レジストリが乖離する。`WM_SETTINGCHANGE`（`SPI_SETCURSORS` 関連）を
//!  > 購読して外部変更を検知し、UI に反映する。

use crate::errors::AppResult;

/// 外部カーソル変更コールバック (グローバル)
#[cfg(windows)]
static CURSOR_CHANGE_CALLBACK: std::sync::Mutex<Option<Box<dyn Fn() + Send>>> =
    std::sync::Mutex::new(None);

/// `WM_SETTINGCHANGE` を購読してカーソル設定変更を検知する。
///
/// `on_change` は `SPI_SETCURSORS` (uiAction=0x0057) を受信したときに呼ばれる。
/// 内部はバックグラウンドスレッドでメッセージループを回す。
#[cfg(windows)]
pub fn start_cursor_watcher<F>(on_change: F) -> AppResult<()>
where
    F: Fn() + Send + 'static,
{
    use windows::core::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    std::thread::spawn(move || {
        unsafe {
            let class_name = w!("CursorForgeCursorWatcher");
            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(cursor_wnd_proc),
                lpszClassName: class_name,
                ..Default::default()
            };
            RegisterClassW(&wnd_class);

            let hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("CursorForge Cursor Watcher"),
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
                    tracing::error!("カーソル監視ウィンドウ作成失敗: {}", e);
                    return;
                }
            };

            // 不可視ウィンドウなのでフォーカスは奪わない
            CURSOR_CHANGE_CALLBACK.lock().unwrap().replace(Box::new(on_change));
            tracing::info!("カーソル設定変更監視を開始しました");

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, Some(hwnd), 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });

    Ok(())
}

#[cfg(not(windows))]
pub fn start_cursor_watcher<F>(_on_change: F) -> AppResult<()>
where
    F: Fn() + Send + 'static,
{
    tracing::warn!("カーソル設定変更監視は Windows 以外では利用できません");
    Ok(())
}

#[cfg(windows)]
unsafe extern "system" fn cursor_wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;

    if msg == WM_SETTINGCHANGE {
        // SPI_SETCURSORS = 0x0057
        const SPI_SETCURSORS: usize = 0x0057;
        if wparam.0 == SPI_SETCURSORS {
            tracing::debug!("SPI_SETCURSORS 変更を検知");
            if let Ok(cb) = CURSOR_CHANGE_CALLBACK.lock() {
                if let Some(ref callback) = *cb {
                    callback();
                }
            }
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
