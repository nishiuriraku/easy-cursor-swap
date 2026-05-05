//! 動作環境検出 (Phase 4-7)
//!
//! 仕様書「動作環境マトリクス」より:
//!  - RDP / Citrix / RemoteApp は独自カーソル描画のため動作対象外
//!  - Windows Server は Server Core で Tauri が動かないため動作対象外
//!
//! このモジュールは起動時に検出して警告ダイアログ用情報を返す。
//! 利用は禁止せず、ユーザーに「動作保証外」を明示するに留める (UAC で詰まると
//! ユーザーがアプリを使えなくなるため、緊急時に Windows 既定リセットだけは
//! 通せる必要がある)。

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EnvironmentReport {
    pub is_remote_session: bool,
    pub is_server_sku: bool,
    pub product_name: Option<String>,
    /// 警告レベル: "ok" / "warn" / "error"
    pub level: String,
    /// 警告メッセージ (UI 表示用)
    pub message: Option<String>,
}

impl EnvironmentReport {
    pub fn detect() -> Self {
        let is_remote = is_remote_session();
        let is_server = is_server_sku();
        let product = product_name();

        let (level, message) = if is_remote {
            (
                "warn".to_string(),
                Some("リモートデスクトップセッション (RDP / Citrix 等) が検出されました。RDP 環境は独自カーソル描画のため、テーマ適用が正しく反映されない可能性があります。".to_string()),
            )
        } else if is_server {
            (
                "warn".to_string(),
                Some("Windows Server エディションが検出されました。Server SKU はサポート対象外です。".to_string()),
            )
        } else {
            ("ok".to_string(), None)
        };

        Self {
            is_remote_session: is_remote,
            is_server_sku: is_server,
            product_name: product,
            level,
            message,
        }
    }
}

/// RDP / Citrix / RemoteApp セッションかどうか。
///
/// `GetSystemMetrics(SM_REMOTESESSION)` は最も確実な判定。Citrix も含む。
#[cfg(windows)]
fn is_remote_session() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTESESSION};
    unsafe { GetSystemMetrics(SM_REMOTESESSION) != 0 }
}

#[cfg(not(windows))]
fn is_remote_session() -> bool {
    false
}

/// Windows Server SKU かどうか。
///
/// `VerifyVersionInfo` で `VER_NT_SERVER` を確認する。
/// 簡易判定として `ProductName` レジストリ値も補助に使う。
#[cfg(windows)]
fn is_server_sku() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = match hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion") {
        Ok(k) => k,
        Err(_) => return false,
    };
    let installation_type: Result<String, _> = key.get_value("InstallationType");
    match installation_type {
        Ok(v) => {
            // "Client" / "Server" / "Server Core"
            let v = v.to_lowercase();
            v.contains("server")
        }
        Err(_) => false,
    }
}

#[cfg(not(windows))]
fn is_server_sku() -> bool {
    false
}

/// `ProductName` (例: "Windows 11 Pro")
#[cfg(windows)]
fn product_name() -> Option<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .ok()?;
    key.get_value::<String, _>("ProductName").ok()
}

#[cfg(not(windows))]
fn product_name() -> Option<String> {
    None
}
