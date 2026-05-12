//! OS 起動時自動起動の登録 / 解除
//!
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\EasyCursorSwap` に
//! 実行ファイルパスを書き込む / 削除する。HKCU のため UAC 不要。
//!
//! 設定 `general.auto_start` を Source of Truth とし、`update_config` 経由および
//! アプリ起動直後にレジストリを同期する。
//!
//! ## MSIX パッケージ環境での扱い
//!
//! MSIX (Microsoft Store / sideload) 配布で起動された場合は、AppxManifest.xml の
//! `<Extension Category="windows.startupTask">` 経由で OS が自動起動を管理する。
//! Run キーへの直接書き込みは Store ポリシー上推奨されないため、本モジュールは
//! MSIX 環境を検出した場合は no-op として早期 return し、有効状態の問い合わせも
//! 「OS startupTask 側に委譲」した結果を返す。
//!
//! 検出は `current_exe()` のパスに `\WindowsApps\` が含まれるかで簡易判定する
//! (Microsoft が `GetCurrentPackageFullName` の前段スクリーニングとして例示する手法)。

use crate::errors::{AppError, AppResult};

/// HKCU 配下の Run キー
const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// レジストリに登録する値名 (= アプリ表示名)
const APP_VALUE_NAME: &str = "EasyCursorSwap";

/// MSIX パッケージ環境で実行されているかを判定する。
///
/// `current_exe()` が `\WindowsApps\` 配下なら MSIX とみなす。失敗時は `false`
/// (= 通常の Win32 起動として扱う) にフォールバックする。
pub fn is_msix_packaged() -> bool {
    is_msix_packaged_for_path(std::env::current_exe().ok().as_deref())
}

/// テスト容易性のため、判定対象パスを引数で受け取る純粋関数。
fn is_msix_packaged_for_path(exe: Option<&std::path::Path>) -> bool {
    let Some(path) = exe else { return false };
    // 大文字小文字混在 (`WindowsApps` / `windowsapps`) を許容するため
    // OsStr → 小文字化した String で照合する。
    let s = path.to_string_lossy().to_ascii_lowercase();
    s.contains(r"\windowsapps\")
}

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
///
/// MSIX 環境では Run キーは原則使用しないため、AppxManifest の startupTask 宣言
/// (Enabled=false 初期値) に従い `false` を返す。実際の有効/無効状態の取得は
/// `Windows.ApplicationModel.StartupTask.GetAsync` を要するが、現状はユーザーが
/// 「設定 → スタートアップ アプリ」で切替する前提とする。
pub fn is_enabled() -> bool {
    if is_msix_packaged() {
        return false;
    }
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
        if is_msix_packaged() {
            // MSIX では AppxManifest の startupTask に委譲する。
            // ユーザーが「設定 → スタートアップ アプリ」で操作するのが Store ポリシー準拠の動線。
            tracing::info!(
                "MSIX 環境を検出: Run キーへの書き込みをスキップ (startupTask に委譲, requested={})",
                enabled
            );
            return Ok(());
        }
        let command = if enabled {
            Some(build_run_command()?)
        } else {
            None
        };
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
        format!("EasyCursorSwap_test_{}_{}", std::process::id(), suffix)
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
        assert!(
            is_enabled_with_name(&name),
            "書き込み後に有効と判定されるべき"
        );

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
        assert!(
            cmd.starts_with('"'),
            "パスは二重引用符で囲まれるべき: {}",
            cmd
        );
        assert!(
            cmd.ends_with("--autostart"),
            "末尾に --autostart 引数が付くべき: {}",
            cmd
        );
    }

    #[test]
    fn detects_msix_path_case_insensitive() {
        use std::path::PathBuf;
        // 大文字 / 小文字 / 混在のいずれも MSIX として判定される
        let cases = [
            r"C:\Program Files\WindowsApps\dev.easycursorswap.app_1.0.0_x64__abc\app.exe",
            r"C:\Program Files\windowsapps\dev.easycursorswap.app_1.0.0_x64__abc\app.exe",
            r"C:\PROGRAM FILES\WINDOWSAPPS\dev.easycursorswap.app_1.0.0_x64__abc\app.exe",
        ];
        for c in cases {
            let p = PathBuf::from(c);
            assert!(
                is_msix_packaged_for_path(Some(&p)),
                "MSIX として判定されるべき: {}",
                c
            );
        }
    }

    #[test]
    fn does_not_detect_normal_install_paths_as_msix() {
        use std::path::PathBuf;
        let cases = [
            r"C:\Program Files\EasyCursorSwap\easy-cursor-swap.exe",
            r"C:\Users\me\AppData\Local\Programs\EasyCursorSwap\app.exe",
            r"D:\dev\target\release\easy-cursor-swap.exe",
        ];
        for c in cases {
            let p = PathBuf::from(c);
            assert!(
                !is_msix_packaged_for_path(Some(&p)),
                "通常インストールは MSIX 扱いされるべきでない: {}",
                c
            );
        }
    }

    #[test]
    fn returns_false_when_exe_path_is_unavailable() {
        assert!(!is_msix_packaged_for_path(None));
    }
}
