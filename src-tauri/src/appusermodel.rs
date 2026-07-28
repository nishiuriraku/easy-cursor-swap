//! AppUserModelID (AUMID) 明示登録 (Phase 7-2 残タスク)
//!
//! Windows のトースト通知 / ジャンプリスト / タスクバーグルーピングで使われる
//! プロセス識別子。`SetCurrentProcessExplicitAppUserModelID` を呼んで明示しておくと、
//! 通知センターで送信元アプリ名が EasyCursorSwap として正しく表示される。
//!
//! 仕様書 §「通知 UX」より:
//!  > AppUserModelID をマニフェストに登録 (MSIX は自動、`.msi` 版はインストーラで設定)
//!  > が、ここでは念のため起動時にも明示する。
//!
//! 参考: https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-setcurrentprocessexplicitappusermodelid

/// AppUserModelID 文字列 (`Vendor.Product.Subproduct.VersionInformation` 形式が推奨)。
/// `tauri.conf.json` の `identifier` (`dev.easycursorswap.app`) と整合させる。
#[cfg(windows)]
pub const APP_USER_MODEL_ID: &str = "dev.easycursorswap.app";

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

#[cfg(test)]
mod tests {
    /// `APP_USER_MODEL_ID` は `tauri.conf.json` の `identifier` と整合させる
    /// 契約がある。`Vendor.Product.Subproduct.VersionInformation` 形式の
    /// ベンダー ID + アプリ ID + Subproduct の少なくとも 3 ドットを含む
    /// canonical 形であることを保証する。
    ///
    /// NOTE: `super::APP_USER_MODEL_ID` は `#[cfg(windows)] pub const` なので、
    /// `cargo check --target x86_64-unknown-linux-gnu` 等の非 Windows ターゲット
    /// でもこのテストモジュールをコンパイルできるよう、本テストにも
    /// `#[cfg(windows)]` を付与する。
    #[cfg(windows)]
    #[test]
    fn app_user_model_id_has_canonical_dot_separated_form() {
        let parts: Vec<&str> = super::APP_USER_MODEL_ID.split('.').collect();
        assert!(
            parts.len() >= 3,
            "AUMID should have at least 3 dot-separated segments: got {:?}",
            super::APP_USER_MODEL_ID
        );
        for (i, seg) in parts.iter().enumerate() {
            assert!(
                !seg.is_empty(),
                "AUMID segment {i} is empty: {:?}",
                super::APP_USER_MODEL_ID
            );
            assert!(
                seg.chars().all(|c| c.is_ascii_alphanumeric()),
                "AUMID segment {i:?} must be ASCII alphanumeric only: {:?}",
                super::APP_USER_MODEL_ID
            );
        }
    }

    /// AUMID 文字列は `dev.easycursorswap.app` 形式 (= `tauri.conf.json` の
    /// identifier と完全一致) を維持する。これを意図せず変更すると Windows
    /// のトースト通知 / ジャンプリスト / タスクバーグルーピングが Tauri 既定
    /// (タスクバーで別アプリ扱い) にフォールバックする。
    ///
    /// NOTE: Windows-only シンボルを参照するため `#[cfg(windows)]` で gate。
    #[cfg(windows)]
    #[test]
    fn app_user_model_id_matches_tauri_identifier() {
        assert_eq!(super::APP_USER_MODEL_ID, "dev.easycursorswap.app");
    }

    /// 非 Windows での `register_aumid` はノーオペ。
    /// panic / error を返さず呼び出しが即座に返ることのみを保証する。
    #[cfg(not(windows))]
    #[test]
    fn register_aumid_is_noop_on_non_windows() {
        // 失敗しないことだけ確認。
        super::register_aumid();
    }
}
