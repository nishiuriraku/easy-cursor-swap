//! OS 起動時自動起動の登録 / 解除
//!
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\CursorForge` に
//! 実行ファイルパスを書き込む / 削除する。HKCU のため UAC 不要。
//!
//! 設定 `general.auto_start` を Source of Truth とし、`update_config` 経由および
//! アプリ起動直後にレジストリを同期する。

use crate::errors::{AppError, AppResult};

/// HKCU 配下の Run キー
const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// レジストリに登録する値名 (= アプリ表示名)
const APP_VALUE_NAME: &str = "CursorForge";

/// `--autostart` 引数を付与した起動コマンド文字列を組み立てる。
///
/// 引数を付けることで、将来トレイ常駐モード等の分岐を main.rs で行えるよう余地を残す。
#[cfg(windows)]
fn build_run_command() -> AppResult<String> {
    let exe = std::env::current_exe()
        .map_err(|e| AppError::Other(format!("実行ファイルパス取得失敗: {}", e)))?;
    Ok(format!("\"{}\" --autostart", exe.display()))
}

/// 自動起動レジストリの状態を確認する。
///
/// レジストリに値が存在し、文字列として読み取れる場合のみ `true`。
/// 値の中身（パス）は検証しない (旧パスが残っているケースも有効扱い)。
pub fn is_enabled() -> bool {
    is_enabled_with_name(APP_VALUE_NAME)
}

fn is_enabled_with_name(name: &str) -> bool {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        match hkcu.open_subkey(RUN_KEY_PATH) {
            Ok(key) => key.get_value::<String, _>(name).is_ok(),
            Err(_) => false,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = name;
        false
    }
}

/// 自動起動を有効化 / 無効化する。
///
/// - `enabled = true`: Run キーに実行ファイルパスを書き込む
/// - `enabled = false`: Run キーから値を削除する (存在しなければ no-op)
pub fn set_enabled(enabled: bool) -> AppResult<()> {
    #[cfg(windows)]
    {
        let command = if enabled { Some(build_run_command()?) } else { None };
        write_value(APP_VALUE_NAME, command.as_deref())
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Ok(())
    }
}

/// 値を書き込む / 削除する低レベルヘルパー。テスト用に値名を切り替え可能にする。
///
/// `command = Some(_)` で書き込み、`None` で削除。
#[cfg(windows)]
fn write_value(name: &str, command: Option<&str>) -> AppResult<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    match command {
        Some(cmd) => {
            let (key, _disp) = hkcu
                .create_subkey(RUN_KEY_PATH)
                .map_err(|e| AppError::Other(format!("Run キー作成失敗: {}", e)))?;
            key.set_value(name, &cmd.to_string())
                .map_err(|e| AppError::Other(format!("自動起動登録失敗: {}", e)))?;
            tracing::info!("自動起動を登録しました ({})", name);
        }
        None => match hkcu.open_subkey_with_flags(RUN_KEY_PATH, KEY_WRITE) {
            Ok(key) => match key.delete_value(name) {
                Ok(()) => {
                    tracing::info!("自動起動を解除しました ({})", name);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // すでに登録なし → 解除としては成功
                }
                Err(e) => {
                    return Err(AppError::Other(format!("自動起動解除失敗: {}", e)));
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Run キー自体が無い → 解除としては成功
            }
            Err(e) => {
                return Err(AppError::Other(format!("Run キー オープン失敗: {}", e)));
            }
        },
    }
    Ok(())
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::*;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    /// 既存ユーザーの Run エントリと衝突しないよう、テストごとにユニークな値名を生成する。
    fn unique_name(suffix: &str) -> String {
        format!(
            "CursorForge_test_{}_{}",
            std::process::id(),
            suffix
        )
    }

    /// テスト後に値が残らないようクリーンアップ
    fn cleanup(name: &str) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey_with_flags(RUN_KEY_PATH, winreg::enums::KEY_WRITE) {
            let _ = key.delete_value(name);
        }
    }

    #[test]
    fn enabling_writes_value_to_run_key() {
        let name = unique_name("enable");
        cleanup(&name);

        write_value(&name, Some("\"C:\\fake\\path.exe\" --autostart")).unwrap();
        assert!(is_enabled_with_name(&name), "書き込み後に有効と判定されるべき");

        cleanup(&name);
    }

    #[test]
    fn disabling_removes_value() {
        let name = unique_name("disable");
        cleanup(&name);

        write_value(&name, Some("\"C:\\fake\\path.exe\" --autostart")).unwrap();
        assert!(is_enabled_with_name(&name));

        write_value(&name, None).unwrap();
        assert!(!is_enabled_with_name(&name), "削除後は無効と判定されるべき");
    }

    #[test]
    fn disabling_when_not_present_is_noop() {
        let name = unique_name("noop");
        cleanup(&name);

        // 削除対象が存在しなくてもエラーにならない
        write_value(&name, None).unwrap();
        assert!(!is_enabled_with_name(&name));
    }

    #[test]
    fn build_run_command_quotes_path_and_appends_autostart_flag() {
        let cmd = build_run_command().unwrap();
        assert!(cmd.starts_with('"'), "パスは二重引用符で囲まれるべき: {}", cmd);
        assert!(
            cmd.ends_with("--autostart"),
            "末尾に --autostart 引数が付くべき: {}",
            cmd
        );
    }
}
