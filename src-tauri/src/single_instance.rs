//! 多重起動防止 (Phase 4-4)
//!
//! `Local\<UUID>` 形式の Named Mutex を取得することで、
//! 同一ユーザーセッション内での二重起動をブロックする。
//!
//! 仕様書 §5「多重起動防止」:
//!  > 常駐型のため Named Mutex (Local\<AppGuid>) で多重起動をブロックし、
//!  > 既存インスタンスのトレイアイコンへフォーカス (or 通知) を移す。
//!
//! 戻り値:
//!  - `Ok(SingleInstanceLock)` — このプロセスが排他的に保持
//!  - `Err(AppError)` — 既存インスタンスあり、または取得失敗
//!
//! `SingleInstanceLock` を `drop` するとミューテックスが解放される。
//! main 関数の最後まで保持し続けること。
//!
//! 「既存インスタンスへのフォーカス移動」は Named Event (auto-reset) で実装する。
//!  - 第一インスタンスが [`start_show_window_listener`] で待機スレッドを起動
//!  - 第二インスタンスが [`notify_existing_instance`] で `SetEvent` してから終了
//!  - 第一インスタンスのスレッドが起床し、main ウィンドウを show + unminimize + set_focus

use crate::errors::{AppError, AppResult};

/// アプリ固有の GUID。複数プロダクトとの衝突を避けるため一意の値を使用。
/// 変更すると既存ユーザーの常駐プロセスとは別系統と判定されるので注意。
const MUTEX_NAME: &str = "Local\\EasyCursorSwap.SingleInstance.7c2a4f9a-3b8d-4e6f-8a1c-5d9e0f3b6c7d";

/// 「ウィンドウを表示せよ」シグナル用の Named Event 名。
/// MUTEX_NAME と同じ GUID を流用してアプリ単位で対応付ける。
const SHOW_EVENT_NAME: &str =
    "Local\\EasyCursorSwap.ShowWindow.7c2a4f9a-3b8d-4e6f-8a1c-5d9e0f3b6c7d";

#[cfg(windows)]
pub struct SingleInstanceLock {
    handle: windows::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl SingleInstanceLock {
    pub fn acquire() -> AppResult<Self> {
        use windows::core::HSTRING;
        use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
        use windows::Win32::System::Threading::CreateMutexW;

        let name = HSTRING::from(MUTEX_NAME);
        let handle = unsafe {
            CreateMutexW(None, true, &name).map_err(|e| {
                AppError::Config(format!("CreateMutexW 失敗: {}", e))
            })?
        };

        // CreateMutexW は既存ミューテックスがあっても成功する。
        // 既存判定は GetLastError == ERROR_ALREADY_EXISTS で行う。
        let last_err = unsafe { GetLastError() };
        if last_err == ERROR_ALREADY_EXISTS {
            // 取得済みハンドルはここで明示的に閉じる
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(handle);
            }
            return Err(AppError::Config(
                "既に EasyCursorSwap が起動しています".to_string(),
            ));
        }

        Ok(Self { handle })
    }
}

#[cfg(windows)]
impl Drop for SingleInstanceLock {
    fn drop(&mut self) {
        unsafe {
            // ReleaseMutex はミューテックスのオーナーシップを解放
            let _ = windows::Win32::System::Threading::ReleaseMutex(self.handle);
            let _ = windows::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(not(windows))]
pub struct SingleInstanceLock;

#[cfg(not(windows))]
impl SingleInstanceLock {
    pub fn acquire() -> AppResult<Self> {
        Ok(Self)
    }
}

/// 第二インスタンスから既存インスタンスへ「ウィンドウを表示せよ」シグナルを送る。
///
/// Named Event を `OpenEventW(EVENT_MODIFY_STATE)` で開き、`SetEvent` で起床させる。
/// イベントが存在しなければエラーを返す (= 既存インスタンス側がまだリスナーを起動していない)。
#[cfg(windows)]
pub fn notify_existing_instance() -> AppResult<()> {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenEventW, SetEvent, EVENT_MODIFY_STATE};

    let name = HSTRING::from(SHOW_EVENT_NAME);
    let handle = unsafe {
        OpenEventW(EVENT_MODIFY_STATE, false, &name)
            .map_err(|e| AppError::Other(format!("OpenEventW 失敗: {}", e)))?
    };
    let result = unsafe { SetEvent(handle) };
    unsafe {
        let _ = CloseHandle(handle);
    }
    result.map_err(|e| AppError::Other(format!("SetEvent 失敗: {}", e)))
}

/// 第一インスタンスがウィンドウ表示シグナルの待機スレッドを起動する。
///
/// `auto-reset` イベントを作成し、別スレッドで [`WaitForSingleObject`] をループする。
/// シグナルされるたびに `callback` を実行する。プロセス終了時に OS がイベントハンドルを
/// クリーンアップするため、リスナースレッドは明示的な停止機構を持たない (デーモン扱い)。
///
/// `WaitForSingleObject` がエラー値を返した場合はループを抜けてスレッドが終了する。
#[cfg(windows)]
pub fn start_show_window_listener<F>(callback: F) -> AppResult<()>
where
    F: Fn() + Send + 'static,
{
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};

    let name = HSTRING::from(SHOW_EVENT_NAME);
    // manual_reset = false (auto-reset) / initial_state = false
    let handle: HANDLE = unsafe {
        CreateEventW(None, false, false, &name)
            .map_err(|e| AppError::Other(format!("CreateEventW 失敗: {}", e)))?
    };

    // HANDLE は *mut c_void を内包し Send 非実装。
    // usize に剥離してスレッド境界を越えてから HANDLE に戻す。
    let raw = handle.0 as usize;

    std::thread::Builder::new()
        .name("easycursorswap-showwindow-listener".to_string())
        .spawn(move || {
            let h = HANDLE(raw as *mut _);
            loop {
                let r = unsafe { WaitForSingleObject(h, INFINITE) };
                if r == WAIT_OBJECT_0 {
                    callback();
                } else {
                    tracing::warn!(
                        "show-window listener: WaitForSingleObject 異常終了 ({:?})",
                        r
                    );
                    break;
                }
            }
        })
        .map_err(|e| {
            AppError::Other(format!("show-window listener スレッド起動失敗: {}", e))
        })?;
    Ok(())
}

#[cfg(not(windows))]
pub fn notify_existing_instance() -> AppResult<()> {
    Ok(())
}

#[cfg(not(windows))]
pub fn start_show_window_listener<F>(_callback: F) -> AppResult<()>
where
    F: Fn() + Send + 'static,
{
    Ok(())
}
