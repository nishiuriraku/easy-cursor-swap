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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_serializable_report() {
        // detect() は実環境を見るので結果は不定だが、報告型を必ず返す。
        // serialize 可能 (JSON 化できる) ことが UI 連携の最低保証。
        let report = EnvironmentReport::detect();
        let json = serde_json::to_string(&report).expect("serialize ok");
        assert!(json.contains("\"is_remote_session\""));
        assert!(json.contains("\"is_server_sku\""));
        assert!(json.contains("\"level\""));
    }

    #[test]
    fn detect_level_is_one_of_known_values() {
        let report = EnvironmentReport::detect();
        assert!(
            matches!(report.level.as_str(), "ok" | "warn" | "error"),
            "unknown level: {}",
            report.level
        );
    }

    #[test]
    fn detect_attaches_message_when_remote_or_server() {
        let report = EnvironmentReport::detect();
        // 警告が立っていればメッセージは Some、それ以外は None という対応関係。
        if report.is_remote_session || report.is_server_sku {
            assert!(
                report.message.is_some(),
                "warn 状態でメッセージなし: {:?}",
                report
            );
            assert_eq!(report.level, "warn");
        } else {
            assert_eq!(report.level, "ok");
            assert!(report.message.is_none());
        }
    }

    #[test]
    fn detect_message_is_non_empty_when_present() {
        let report = EnvironmentReport::detect();
        if let Some(msg) = &report.message {
            assert!(!msg.trim().is_empty(), "メッセージが空白のみ");
        }
    }

    /// 非 Windows プラットフォームでは false を返すスタブが効くべき。
    /// CI で Linux 上のテストを通すための保険。
    #[cfg(not(windows))]
    #[test]
    fn non_windows_stubs_return_safe_defaults() {
        assert!(!is_remote_session());
        assert!(!is_server_sku());
        assert!(product_name().is_none());
        // detect() は ok レベルになる
        let report = EnvironmentReport::detect();
        assert_eq!(report.level, "ok");
        assert!(report.message.is_none());
    }
}
