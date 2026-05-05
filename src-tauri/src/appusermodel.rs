//! AppUserModelID (AUMID) 明示登録 (Phase 7-2 残タスク)
//!
//! Windows のトースト通知 / ジャンプリスト / タスクバーグルーピングで使われる
//! プロセス識別子。`SetCurrentProcessExplicitAppUserModelID` を呼んで明示しておくと、
//! 通知センターで送信元アプリ名が CursorForge として正しく表示される。
//!
//! 仕様書 §「通知 UX」より:
//!  > AppUserModelID をマニフェストに登録 (MSIX は自動、`.msi` 版はインストーラで設定)
//!  > が、ここでは念のため起動時にも明示する。
//!
//! 参考: https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-setcurrentprocessexplicitappusermodelid

/// AppUserModelID 文字列 (`Vendor.Product.Subproduct.VersionInformation` 形式が推奨)。
/// `tauri.conf.json` の `identifier` (`dev.cursorforge.app`) と整合させる。
#[cfg(windows)]
pub const APP_USER_MODEL_ID: &str = "dev.cursorforge.app";

/// プロセスに AppUserModelID を設定する。
/// 失敗してもアプリ動作は継続 (通知元の表示が "Tauri アプリ" 等になるだけ)。
#[cfg(windows)]
pub fn register_aumid() {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let aumid = HSTRING::from(APP_USER_MODEL_ID);
    let result = unsafe { SetCurrentProcessExplicitAppUserModelID(&aumid) };
    match result {
        Ok(()) => {
            tracing::info!("AppUserModelID 設定: {}", APP_USER_MODEL_ID);
        }
        Err(e) => {
            tracing::warn!("AppUserModelID 設定失敗: {}", e);
        }
    }
}

#[cfg(not(windows))]
pub fn register_aumid() {
    // 非 Windows ではノーオペ
}
