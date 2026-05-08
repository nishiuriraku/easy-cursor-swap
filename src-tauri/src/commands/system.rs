//! 設定・OS 連携・診断系の IPC コマンド集。
//!
//! ここに集約しているのは「アプリ全体の状態 / OS 情報 / config 永続化」など、
//! 特定ドメイン (theme / cursor / marketplace / keystore) に紐付かないユーティリティ的なコマンド。

use crate::accessibility::AccessibilityConflicts;
use crate::autostart;
use crate::config::{AppConfig, BackupInfo, ConfigManager};
use crate::darkmode;
use crate::environment::EnvironmentReport;
use crate::errors::AppError;
use crate::health::is_major_bump;
use crate::registry::RegistryManager;
use serde::Serialize;
use tauri::State;

/// Windows 既定カーソルにリセットする（パニックボタン）
#[tauri::command]
pub fn reset_to_default() -> Result<(), AppError> {
    RegistryManager::reset_to_windows_default()
}

/// インストール前の状態にリセットする
#[tauri::command]
pub fn reset_to_initial() -> Result<(), AppError> {
    RegistryManager::restore_from_initial_snapshot()
}

/// 現在のダークモード状態を取得する
#[tauri::command]
pub fn get_dark_mode_status() -> Result<bool, AppError> {
    darkmode::is_dark_mode()
}

/// 動作環境レポートを返す (RDP / Server SKU 検出)。
/// UI 起動時に呼んで警告ダイアログ表示判定に使う。
#[tauri::command]
pub fn get_environment_report() -> EnvironmentReport {
    EnvironmentReport::detect()
}

/// アプリケーション設定を取得する
#[tauri::command]
pub fn get_config(config: State<'_, ConfigManager>) -> Result<AppConfig, AppError> {
    config.get()
}

/// アプリケーション設定を更新する。
///
/// 副作用として `general.auto_start` をレジストリ (HKCU\...\Run) に同期する。
/// 同期に失敗してもログを出すのみで設定保存自体はエラーとしない (UI 操作の妨げを防ぐため)。
#[tauri::command]
pub fn update_config(
    config: State<'_, ConfigManager>,
    updates: AppConfig,
) -> Result<AppConfig, AppError> {
    let auto_start = updates.general.auto_start;
    let saved = config.update(|c| {
        *c = updates;
    })?;
    if let Err(e) = autostart::set_enabled(auto_start) {
        tracing::warn!("自動起動レジストリ同期失敗: {}", e);
    }
    Ok(saved)
}

/// 現在の自動起動 (HKCU Run) 登録状態を返す。
///
/// 設定 `general.auto_start` とレジストリ実態が乖離していないかの確認用。
#[tauri::command]
pub fn get_autostart_status() -> bool {
    autostart::is_enabled()
}

/// アプリ情報を返す（バージョン等）
#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub cursors_dir: String,
    pub config_dir: String,
    pub os_version: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    let cursors_dir = ConfigManager::cursors_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let config_dir = dirs::data_local_dir()
        .map(|p| p.join("EasyCursorSwap").to_string_lossy().to_string())
        .unwrap_or_default();

    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        cursors_dir,
        config_dir,
        os_version: get_os_version(),
    }
}

/// OS バージョンを取得する
fn get_os_version() -> String {
    #[cfg(windows)]
    {
        let info = windows::Win32::System::SystemInformation::OSVERSIONINFOW::default();
        format!("Windows {}.{}", info.dwMajorVersion, info.dwMinorVersion)
    }
    #[cfg(not(windows))]
    {
        "Non-Windows".to_string()
    }
}

/// 設定バックアップファイルの一覧を返す。
///
/// `config.bak.v*.json` (スキーマ移行バックアップ) と
/// `config.corrupt.*.json` (破損退避ファイル) を列挙し、最終更新日時の降順で返す。
#[tauri::command]
pub fn list_config_backups(config: State<'_, ConfigManager>) -> Result<Vec<BackupInfo>, AppError> {
    config.list_backups()
}

/// 指定したバックアップファイルを `config.json` に上書きして設定を復旧する。
///
/// 復旧後は UI 側でアプリを再起動するかページをリロードすること。
#[tauri::command]
pub fn restore_config_backup(
    file_name: String,
    config: State<'_, ConfigManager>,
) -> Result<(), AppError> {
    config.restore_backup(&file_name)
}

/// 指定 URL をシステムのデフォルトブラウザで開く。
///
/// URL は `https://` または `http://` で始まる必要がある。
/// それ以外は `AppError::InvalidInput` を返す。
///
/// Windows 専用実装: Win32 ShellExecuteW を直接呼ぶ。
#[tauri::command]
pub fn open_url(url: String) -> Result<(), AppError> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(AppError::InvalidInput(format!(
            "不正な URL スキーム: {}",
            url
        )));
    }
    #[cfg(windows)]
    {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let url_h = HSTRING::from(url.as_str());
        let result = unsafe {
            ShellExecuteW(
                None,
                PCWSTR(HSTRING::from("open").as_ptr()),
                PCWSTR(url_h.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        // ShellExecuteW は HINSTANCE を返す; ポインタ値が 32 より大きければ成功
        if (result.0 as usize) <= 32 {
            return Err(AppError::Other("ShellExecuteW が失敗しました".to_string()));
        }
    }
    #[cfg(not(windows))]
    {
        return Err(AppError::Other("open_url は Windows 専用です".to_string()));
    }
    Ok(())
}

/// アクセシビリティ機能との競合を検出する。
///
/// レジストリから MouseSonar / HighContrast / CursorBaseSize を読み取り、
/// テーマ適用時にユーザーへ警告すべき状態かを返す。
#[tauri::command]
pub fn get_accessibility_conflicts() -> AccessibilityConflicts {
    AccessibilityConflicts::detect()
}

/// 現行バージョンから新バージョンへの更新がメジャー跨ぎかどうかを返す。
///
/// フロントエンドはアップデート確認時にこれを呼び出し、`true` の場合は
/// 追加の確認ダイアログをユーザーに表示すること。
#[tauri::command]
pub fn check_update_is_major_jump(current_version: String, new_version: String) -> bool {
    is_major_bump(&current_version, &new_version)
}

/// 保存済みクラッシュレポート (panic) の一覧を返す。
///
/// 新しい順、上限 50 件 (`crash::list_reports` の内部上限)。
/// レポート本体には PII 除外済みのメッセージのみ含まれる。
#[tauri::command]
pub fn list_crash_reports() -> Result<Vec<crate::crash::CrashReport>, AppError> {
    crate::crash::list_reports()
}

/// 保存済みクラッシュレポートを全削除する。戻り値は削除した件数。
///
/// ユーザーが「設定 → ログ → クラッシュ履歴を消去」を実行したときに呼ぶ想定。
#[tauri::command]
pub fn clear_crash_reports() -> Result<usize, AppError> {
    crate::crash::clear_reports()
}

/// 保留中のクラッシュレポートを Cloudflare Worker に送信する (オプトイン UI ボタン用)。
///
/// 起動時の自動送信 (`main.rs` の setup) と同じロジックを手動トリガーで呼び出す。
/// 以下のいずれかに該当すると `Ok(SubmitSummary::default())` を返し、ネットワーク発呼しない:
///
/// - `general.crash_reporting == false` (オプトイン無効)
/// - ビルド時 env 未設定 (`embedded_credentials()` が `None`)
///
/// それ以外は [`crate::crash::submit_pending_reports`] に委譲する。
/// 戻り値の `sent / failed / skipped` を UI 側でトースト表示するなどに使う。
#[tauri::command]
pub async fn submit_crash_reports(
    config: State<'_, ConfigManager>,
) -> Result<crate::crash::SubmitSummary, AppError> {
    let cfg = config.get()?;
    if !cfg.general.crash_reporting {
        return Ok(crate::crash::SubmitSummary::default());
    }
    let Some((endpoint, token)) = crate::crash::embedded_credentials() else {
        return Ok(crate::crash::SubmitSummary::default());
    };
    crate::crash::submit_pending_reports(endpoint, token).await
}
