//! 設定・OS 連携・診断系の IPC コマンド集。
//!
//! ここに集約しているのは「アプリ全体の状態 / OS 情報 / config 永続化」など、
//! 特定ドメイン (theme / cursor / marketplace / keystore) に紐付かないユーティリティ的なコマンド。

use crate::accessibility::AccessibilityConflicts;
use crate::autostart;
use crate::config::{AppConfig, BackupInfo, ConfigManager};
use crate::environment::EnvironmentReport;
use crate::errors::AppError;
use crate::health::is_major_bump;
use crate::registry::RegistryManager;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

/// reset 系 IPC の共通後処理。
///
/// レジストリ操作 (`RegistryManager::reset_to_windows_default` 等) を closure で
/// 受け、成功した場合のみ `active_theme_id` クリアと `cursor-changed` 発火を
/// 走らせる。 `action_label` はログ出力時の prefix。
///
/// `cursor-changed` を明示発火する理由: cursor_watcher は `HWND_MESSAGE` で
/// 作られた message-only window で WM_SETTINGCHANGE のブロードキャストを
/// 受け取れない。SPI_SETCURSORS による即時反映だけでは UI に伝わらない。
fn reset_with_cleanup<F>(
    app: AppHandle,
    config: State<'_, ConfigManager>,
    action_label: &str,
    registry_action: F,
) -> Result<(), AppError>
where
    F: FnOnce() -> Result<(), AppError>,
{
    registry_action()?;
    if let Err(err) = config.update(|c| c.general.active_theme_id = None) {
        tracing::warn!("{}: active_theme_id クリア失敗: {}", action_label, err);
    }
    if let Err(err) = app.emit("cursor-changed", ()) {
        tracing::warn!("{}: cursor-changed emit 失敗: {}", action_label, err);
    }
    Ok(())
}

/// Windows 既定カーソルにリセットする（パニックボタン）。
///
/// 副作用:
///  - レジストリ `HKCU\Control Panel\Cursors` を Windows 既定に戻す
///  - config の `active_theme_id` を `None` にクリア
///  - `cursor-changed` イベントを発火
#[tauri::command]
pub fn reset_to_default(app: AppHandle, config: State<'_, ConfigManager>) -> Result<(), AppError> {
    reset_with_cleanup(app, config, "reset_to_default", || {
        RegistryManager::reset_to_windows_default()
    })
}

/// インストール前の状態にリセットする。
///
/// `reset_to_default` と同じく `active_theme_id` クリア + `cursor-changed` 発火を行う。
#[tauri::command]
pub fn reset_to_initial(app: AppHandle, config: State<'_, ConfigManager>) -> Result<(), AppError> {
    reset_with_cleanup(app, config, "reset_to_initial", || {
        RegistryManager::restore_from_initial_snapshot()
    })
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

/// OS バージョンを取得する。
///
/// `GetVersionExW` はアプリマニフェスト互換のため Windows 8.1 以降では
/// クランプされ得る (常に 6.2 を返す可能性) ため、`ntdll.dll` の
/// `RtlGetVersion` を直接呼ぶ。Microsoft 公式が "true OS version"
/// を取得する用途で推奨している経路。
fn get_os_version() -> String {
    #[cfg(windows)]
    {
        use windows::Wdk::System::SystemServices::RtlGetVersion;
        use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };
        // SAFETY: `RtlGetVersion` は dwOSVersionInfoSize を正しく設定した
        // OSVERSIONINFOW へのポインタを要求する。`info` はスタック上の
        // 有効な値で、上で `dwOSVersionInfoSize` を設定済み。
        let status = unsafe { RtlGetVersion(&mut info) };
        if status.is_ok() {
            format!(
                "Windows {}.{} (build {})",
                info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
            )
        } else {
            "Windows (unknown)".to_string()
        }
    }
    #[cfg(not(windows))]
    {
        "Non-Windows".to_string()
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// `get_os_version` は "Windows {major}.{minor} (build {build})" 形式で返し、
    /// major が 10 以上 (Win10 22H2 / Win11 のいずれか) であることを確認する。
    /// 旧実装は `OSVERSIONINFOW::default()` のフィールドゼロから "Windows 0.0"
    /// を返していた回帰防止。
    #[test]
    fn get_os_version_returns_real_windows_version() {
        let v = get_os_version();
        assert!(v.starts_with("Windows "), "unexpected prefix: {v}");
        let rest = v.trim_start_matches("Windows ");
        let major: u32 = rest
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        assert!(major >= 10, "expected major >= 10 (Win10/Win11), got {v}");
    }

    /// `open_url` の許可スキーム判定 contract: `https://` / `http://` / `ms-settings:`
    /// の 3 種類のみを受け付け、それ以外 (file: / javascript: / カスタム scheme / 空文字 /
    /// プレフィックス類似 ms-cmd: 等) はすべて拒否する。
    ///
    /// この契約が崩れると (1) 設定 → 一般 → カーソルサイズ の「Windows の設定を開く」
    /// deep-link が再び動かなくなる、もしくは (2) 任意 URL スキームを通してしまい
    /// shell 経由の攻撃面が拡大する、のいずれかの回帰が起きるので固定で守る。
    #[test]
    fn is_allowed_url_scheme_accepts_known_schemes() {
        assert!(is_allowed_url_scheme("https://example.com"));
        assert!(is_allowed_url_scheme("http://example.com"));
        assert!(is_allowed_url_scheme("ms-settings:easeofaccess-cursor"));
        assert!(is_allowed_url_scheme("ms-settings:display"));
    }

    #[test]
    fn is_allowed_url_scheme_rejects_unknown_schemes() {
        // よくある危険スキーム
        assert!(!is_allowed_url_scheme(
            "file:///C:/Windows/System32/cmd.exe"
        ));
        assert!(!is_allowed_url_scheme("javascript:alert(1)"));
        assert!(!is_allowed_url_scheme("data:text/html,<script>"));
        // ms-settings: と紛らわしいが別物
        assert!(!is_allowed_url_scheme("ms-cmd:foo"));
        assert!(!is_allowed_url_scheme("ms-settings"));
        // 相対パス / 空文字
        assert!(!is_allowed_url_scheme("/etc/passwd"));
        assert!(!is_allowed_url_scheme(""));
        // scheme 違い
        assert!(!is_allowed_url_scheme("ftp://example.com"));
    }
}

/// 設定バックアップファイルの一覧を返す。
///
/// `config.corrupt.*.json` (パースエラー時の退避ファイル) を列挙し、
/// 最終更新日時の降順で返す。
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

/// Windows shell 経由でファイル / URL を開く共通 helper。
///
/// `verb` を `Some("open")` で渡すと URL の関連付け起動、`None` でフォルダや
/// ファイルの既定アクション起動。失敗時の HINSTANCE 値が 32 以下なら失敗。
///
/// HSTRING を let-binding で受けてから PCWSTR を取得することで、以前の
/// `open_url` 実装にあった temporary HSTRING パターンの潜在 UB を解消する。
#[cfg(windows)]
fn shell_execute_w(verb: Option<&str>, path: &str) -> Result<(), AppError> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let path_h = HSTRING::from(path);
    let verb_h = verb.map(HSTRING::from);
    let verb_ptr = verb_h
        .as_ref()
        .map(|h| PCWSTR(h.as_ptr()))
        .unwrap_or(PCWSTR::null());

    let result = unsafe {
        ShellExecuteW(
            None,
            verb_ptr,
            PCWSTR(path_h.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW は HINSTANCE を返す; ポインタ値が 32 より大きければ成功
    if (result.0 as usize) <= 32 {
        return Err(AppError::Other(format!(
            "ShellExecuteW が失敗しました (path={:?})",
            path
        )));
    }
    Ok(())
}

/// `open_url` が受け付ける URL スキーム一覧。
///
/// 拡張時の方針: Windows shell が解釈する URI スキームのうち、attack surface が
/// 限定的でアプリから明示的に呼ぶ用途があるものだけを許可する。任意のカスタム
/// スキーム (例: `app:` / `file:` / `javascript:`) は拒否する。
///
/// - `https://` / `http://`: 既定のブラウザで開く (Marketplace / About 等)。
/// - `ms-settings:`: Windows Settings deep-link (例: `ms-settings:easeofaccess-cursor`)。
///   引数は Settings page 識別子で shell command ではないため、ShellExecuteW 経由で
///   呼んでも shell command injection は発生しない。
const ALLOWED_URL_SCHEME_PREFIXES: &[&str] = &["https://", "http://", "ms-settings:"];

/// URL が許可スキームで始まっているかを判定する pure 関数 (テスト容易化用)。
fn is_allowed_url_scheme(url: &str) -> bool {
    ALLOWED_URL_SCHEME_PREFIXES
        .iter()
        .any(|prefix| url.starts_with(prefix))
}

/// 指定 URL をシステムのデフォルトハンドラで開く。
///
/// 受け付けるスキームは [`ALLOWED_URL_SCHEME_PREFIXES`] に限定する。
/// それ以外は `AppError::InvalidInput` を返す。
///
/// Windows 専用実装: [`shell_execute_w`] 経由。
#[tauri::command]
pub fn open_url(url: String) -> Result<(), AppError> {
    if !is_allowed_url_scheme(&url) {
        return Err(AppError::InvalidInput(format!(
            "不正な URL スキーム: {}",
            url
        )));
    }
    #[cfg(windows)]
    {
        shell_execute_w(Some("open"), &url)?;
    }
    #[cfg(not(windows))]
    {
        let _ = url;
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

/// マウスポインターのサイズ (HKCU\Control Panel\Cursors\CursorBaseSize) を設定する。
///
/// Windows 設定アプリの「アクセシビリティ → マウスポインターとタッチ → サイズ」
/// スライダーと等価な操作。引数 `size` は DWORD 値で、`set_cursor_base_size` 側で
/// 32〜256 にクランプされる。戻り値は実際に書き込まれた値 (クランプ後)。
///
/// フロントエンドは Settings ページの一般セクションのスライダー (1〜15) を
/// `32 + 16 * (slider - 1)` で DWORD に変換してから呼ぶ。実換算式は Rust 側
/// (`registry::slider_position_to_base_size`) が single source of truth。
#[tauri::command]
pub fn set_cursor_base_size(app: AppHandle, size: u32) -> Result<u32, AppError> {
    tracing::info!("set_cursor_base_size 要求: size={}", size);
    let written = RegistryManager::set_cursor_base_size(size)?;
    // cursor_watcher は HWND_MESSAGE 上に作られた message-only window で
    // WM_SETTINGCHANGE のブロードキャストを受け取れないため、SPI_SETCURSORS で
    // 反映した変更を UI に伝える 'cursor-changed' を明示発火する
    // (reset_with_cleanup と同じ理由)。
    if let Err(err) = app.emit("cursor-changed", ()) {
        tracing::warn!("set_cursor_base_size: cursor-changed emit 失敗: {}", err);
    }
    Ok(written)
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

/// 現在のログ出力ディレクトリを Windows Explorer で開く。
///
/// `logging::log_dir()` が返すパス (= `%LOCALAPPDATA%\EasyCursorSwap\logs\`) を
/// [`shell_execute_w`] で Explorer に渡す。`open_url` と異なり verb を NULL に
/// すると「ディレクトリの既定動作 = Explorer で開く」となる (Win32 仕様)。
///
/// ログ出力が一度も走っていない起動直後でもボタンを押して開けるよう、
/// 存在しない場合は `create_dir_all` で先に作る。
#[tauri::command]
pub fn open_log_folder() -> Result<(), AppError> {
    let dir = crate::logging::log_dir()?;
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| {
            AppError::Other(format!("ログディレクトリ作成失敗 {}: {}", dir.display(), e))
        })?;
    }
    #[cfg(windows)]
    {
        shell_execute_w(None, &dir.to_string_lossy())?;
    }
    #[cfg(not(windows))]
    {
        let _ = dir;
        return Err(AppError::Other(
            "open_log_folder は Windows 専用です".to_string(),
        ));
    }
    Ok(())
}
