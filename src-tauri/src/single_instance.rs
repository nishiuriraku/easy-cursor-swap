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

use crate::errors::{AppError, AppResult};

/// アプリ固有の GUID。複数プロダクトとの衝突を避けるため一意の値を使用。
/// 変更すると既存ユーザーの常駐プロセスとは別系統と判定されるので注意。
const MUTEX_NAME: &str = "Local\\CursorForge.SingleInstance.7c2a4f9a-3b8d-4e6f-8a1c-5d9e0f3b6c7d";

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
                "既に CursorForge が起動しています".to_string(),
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
