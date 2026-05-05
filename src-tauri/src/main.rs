//! EasyCursorSwap - メインエントリポイント
//!
//! Tauri アプリケーションの初期化、トレイ常駐、ダークモード監視を統括する。

// リリースビルドではコンソールウィンドウを非表示にする
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app_lib::appusermodel;
use app_lib::autostart;
use app_lib::commands;
use app_lib::config::ConfigManager;
use app_lib::cursor_watcher;
use app_lib::darkmode;
use app_lib::health::{RollbackTarget, StartupCheck};
use app_lib::hotkey;
use app_lib::logging;
use app_lib::registry::RegistryManager;
use app_lib::single_instance::{self, SingleInstanceLock};
use app_lib::tray;

/// 連続起動失敗 3 回検出時のロールバック案内ダイアログ。
/// ユーザーが「はい」を選んだ場合に前バージョンのリリースページを開く。
#[cfg(windows)]
fn show_rollback_dialog(target: &RollbackTarget) {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_ICONWARNING, MB_TOPMOST, MB_YESNO,
    };

    let body = format!(
        "EasyCursorSwap が 3 回連続して正常に起動できませんでした。\n\n\
        前バージョン v{} にロールバックしますか?\n\n\
        「はい」をクリックするとリリースページをブラウザで開きます。\n\
        インストーラをダウンロードして再インストールしてください。",
        target.version
    );
    let title = HSTRING::from("EasyCursorSwap — 起動失敗を検出");
    let body_h = HSTRING::from(body);

    let result = unsafe {
        MessageBoxW(None, &body_h, &title, MB_YESNO | MB_ICONWARNING | MB_TOPMOST)
    };
    if result == IDYES {
        // ShellExecute でブラウザを開く
        use windows::core::PCWSTR;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let url = HSTRING::from(target.releases_page_url.as_str());
        unsafe {
            ShellExecuteW(
                None,
                PCWSTR(HSTRING::from("open").as_ptr()),
                PCWSTR(url.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
        }
    }
}

#[cfg(not(windows))]
fn show_rollback_dialog(_target: &RollbackTarget) {}

/// 設定マイグレーション失敗時の専用ダイアログ。
/// Win32 MessageBox で「バックアップの場所」と「復旧手順」を表示する。
/// Tauri ランタイムを起動できる前のフェーズなので Tauri Dialog ではなく Win32 を直接呼ぶ。
#[cfg(windows)]
fn show_migration_failure_dialog(err: &str) {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_OK, MB_TOPMOST,
    };

    let config_dir = dirs::data_local_dir()
        .map(|p| p.join("EasyCursorSwap").to_string_lossy().to_string())
        .unwrap_or_else(|| "%LOCALAPPDATA%\\EasyCursorSwap".to_string());

    let body = format!(
        "EasyCursorSwap の設定ファイルを読み込めませんでした。\n\n\
        理由: {err}\n\n\
        バックアップ:\n  {config_dir}\\config.bak.v*.json\n  {config_dir}\\config.corrupt.*.json\n\n\
        いずれかをリネームして config.json に戻すと前回状態に復旧できます。\n\
        詳細はドキュメントを参照してください。"
    );
    let title = HSTRING::from("EasyCursorSwap — 設定読み込みエラー");
    let body_h = HSTRING::from(body);

    unsafe {
        MessageBoxW(None, &body_h, &title, MB_OK | MB_ICONERROR | MB_TOPMOST);
    }
}

#[cfg(not(windows))]
fn show_migration_failure_dialog(_err: &str) {}

fn main() {
    // ロギング初期化（日次ローテ + 14日保持 + 100MB上限 + PII redaction）
    // _guard は drop 時に未書き出しバッファを flush するため main の最後まで保持。
    let _log_guard = match logging::init_logging("info") {
        Ok(g) => Some(g),
        Err(e) => {
            eprintln!("[logging] init failed: {}", e);
            // フォールバックの最小ロガー
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .with_target(false)
                .init();
            None
        }
    };

    tracing::info!("EasyCursorSwap v{} を起動しています...", env!("CARGO_PKG_VERSION"));

    // 起動ヘルスチェック: 連続失敗を検出
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let startup_check = match StartupCheck::begin(&app_version) {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!("startup health check の初期化に失敗: {}", e);
            None
        }
    };
    if let Some(ref check) = startup_check {
        if check.should_rollback {
            tracing::warn!(
                "連続起動失敗 3 回を検出。ロールバックダイアログを表示します。"
            );
            if let Some(target) = check.rollback_target() {
                // Win32 ダイアログを表示し、ユーザーが「はい」の場合にリリースページを開く
                show_rollback_dialog(&target);
            } else {
                tracing::warn!(
                    "ロールバック先バージョンが不明です。設定 → アップデートから手動で対処してください。"
                );
            }
        }
    }

    // AppUserModelID を明示登録 (トースト発信元の見える化)
    appusermodel::register_aumid();

    // 多重起動防止: Named Mutex を取得。既存インスタンスがあれば中断。
    // _instance_lock は drop 時にミューテックスを解放するので main の最後まで保持。
    let _instance_lock = match SingleInstanceLock::acquire() {
        Ok(lock) => lock,
        Err(e) => {
            tracing::warn!("多重起動を検出: {}", e);
            // 既存インスタンスへ「ウィンドウを表示せよ」シグナルを送って静かに終了
            if let Err(notify_err) = single_instance::notify_existing_instance() {
                tracing::warn!("既存インスタンスへの通知失敗: {}", notify_err);
                eprintln!("EasyCursorSwap は既に起動しています");
            } else {
                tracing::info!("既存インスタンスへフォーカス要求を送信しました");
            }
            return;
        }
    };

    // 設定マネージャー初期化
    let config_manager = match ConfigManager::init() {
        Ok(cm) => cm,
        Err(e) => {
            tracing::error!("設定の初期化に失敗: {}", e);

            // 仕様書に従いデフォルト強制起動はしない。
            // 専用ダイアログでバックアップの場所を明示してユーザーに復旧を促す。
            show_migration_failure_dialog(&e.to_string());

            eprintln!("設定の初期化に失敗しました: {}", e);
            return;
        }
    };

    // 自動起動レジストリ (HKCU\...\Run) を config に追従させる
    // ユーザーが手動で削除していても起動のたびに復元される (config が Source of Truth)
    {
        let auto_start = config_manager
            .get()
            .map(|c| c.general.auto_start)
            .unwrap_or(false);
        if let Err(e) = autostart::set_enabled(auto_start) {
            tracing::warn!("自動起動レジストリ同期失敗 (起動時): {}", e);
        }
    }

    // 初回起動時のスナップショット保存
    if let Err(e) = RegistryManager::save_initial_snapshot() {
        tracing::warn!("初回スナップショットの保存に失敗: {}", e);
    }

    // 孤児カーソル復旧: ~/.custom_cursors/<UUID>/ が手動削除されていた場合、
    // config の参照をクリアし、active なら Windows 既定へ戻す
    match app_lib::theme::ThemeManager::cleanup_orphan_references(&config_manager) {
        Ok(true) => tracing::info!("孤児カーソル参照を復旧しました"),
        Ok(false) => tracing::debug!("孤児カーソル参照なし"),
        Err(e) => tracing::warn!("孤児カーソルチェックに失敗: {}", e),
    }

    // クラッシュリカバリ: pending スナップショットの確認
    match RegistryManager::check_pending_snapshot() {
        Ok(Some(_snapshot)) => {
            tracing::warn!("前回の適用処理が中断されていました。復元を開始します...");
            // スナップショットから復元
            if let Err(e) = RegistryManager::reset_to_windows_default() {
                tracing::error!("クラッシュリカバリに失敗: {}", e);
            } else {
                tracing::info!("クラッシュリカバリ完了");
            }
            // スナップショットを削除
            let _ = RegistryManager::remove_pending_snapshot();
        }
        Ok(None) => {
            tracing::debug!("pending スナップショットなし（正常）");
        }
        Err(e) => {
            tracing::warn!("pending スナップショットの確認に失敗: {}", e);
        }
    }

    // setup クロージャは config_manager が move された後に実行されるため、
    // ホットキー文字列はここで先に取り出して持ち回す
    let hotkey_spec = config_manager
        .get()
        .map(|c| c.general.panic_hotkey.clone())
        .unwrap_or_else(|_| "Ctrl+Alt+Shift+R".to_string());

    // Tauri アプリケーションビルド
    let mut builder = tauri::Builder::default();
    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_mcp_bridge::init());
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(config_manager)
        .invoke_handler(commands::get_command_handlers())
        .setup(move |app| {
            let handle = app.handle().clone();

            // システムトレイの初期化
            if let Err(e) = tray::setup_tray(&handle) {
                tracing::error!("システムトレイの初期化に失敗: {}", e);
            }

            // 第二インスタンスからの「ウィンドウを表示せよ」シグナル待機
            let show_handle = handle.clone();
            if let Err(e) = single_instance::start_show_window_listener(move || {
                use tauri::Manager;
                if let Some(window) = show_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                    tracing::info!("第二インスタンス要求でメインウィンドウを前面化");
                } else {
                    tracing::warn!("show-window listener: main ウィンドウが見つからない");
                }
            }) {
                tracing::warn!("show-window listener の起動に失敗: {}", e);
            }

            // ダークモード監視の開始 — テーマ自動切替まで含めて配線
            let app_handle = handle.clone();
            if let Err(e) = darkmode::start_dark_mode_watcher(move |is_dark| {
                tracing::info!(
                    "ダークモード変更を検知: {}",
                    if is_dark { "ダーク" } else { "ライト" }
                );
                // app_lib::tauri を経由して State<ConfigManager> を引き出す
                use tauri::Manager;
                let cfg_state: tauri::State<ConfigManager> = app_handle.state();
                let config = match cfg_state.get() {
                    Ok(c) => c,
                    Err(err) => {
                        tracing::warn!("auto-switch: config 取得失敗: {}", err);
                        return;
                    }
                };
                if !config.dark_mode.enabled {
                    tracing::debug!("auto-switch: dark_mode.enabled = false なのでスキップ");
                    return;
                }
                let target = if is_dark {
                    config.dark_mode.dark_theme_id
                } else {
                    config.dark_mode.light_theme_id
                };
                let target = match target {
                    Some(id) => id,
                    None => {
                        tracing::info!(
                            "auto-switch: {} 側にテーマ未設定のためスキップ",
                            if is_dark { "Dark" } else { "Light" }
                        );
                        return;
                    }
                };
                if config.general.active_theme_id == Some(target) {
                    tracing::debug!("auto-switch: 既に対象テーマがアクティブ");
                    return;
                }
                match app_lib::theme::ThemeManager::apply_theme(target) {
                    Ok(()) => {
                        // active_theme_id 永続化
                        if let Err(err) = cfg_state.update(|c| {
                            c.general.active_theme_id = Some(target);
                        }) {
                            tracing::warn!("auto-switch: active_theme_id 保存失敗: {}", err);
                        } else {
                            tracing::info!("auto-switch: テーマ {} を適用しました", target);
                        }
                    }
                    Err(err) => {
                        tracing::error!("auto-switch: テーマ適用失敗 ({}): {}", target, err);
                    }
                }
            }) {
                tracing::warn!("ダークモード監視の開始に失敗: {}", e);
            }

            // パニックホットキー (`Ctrl+Alt+Shift+R` 等 config 値) の登録
            // 押下時はフロントへ `panic-hotkey` イベントを発火し、PanicFlow を起動させる
            let hotkey_handle = handle.clone();
            let spec = hotkey_spec.clone();
            if let Err(e) = hotkey::register_panic_hotkey(&spec, move || {
                use tauri::{Emitter, Manager};
                tracing::info!("panic-hotkey イベントを発火");
                if let Err(err) = hotkey_handle.emit("panic-hotkey", ()) {
                    tracing::warn!("panic-hotkey emit 失敗: {}", err);
                }
                // ウィンドウが最小化/トレイ収納の場合は表に出す
                if let Some(window) = hotkey_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }) {
                tracing::warn!("パニックホットキー登録に失敗: {}", e);
            }

            // 外部カーソル変更監視 — コントロールパネル等で書き換えられたら UI を再読込
            let cursor_handle = handle.clone();
            if let Err(e) = cursor_watcher::start_cursor_watcher(move || {
                use tauri::Emitter;
                tracing::info!("外部カーソル変更を検知 → cursor-changed イベント発火");
                if let Err(err) = cursor_handle.emit("cursor-changed", ()) {
                    tracing::warn!("cursor-changed イベント発火失敗: {}", err);
                }
            }) {
                tracing::warn!("カーソル監視の開始に失敗: {}", e);
            }

            // 全初期化に成功 → 起動ヘルスチェックを「正常」と記録
            // (この行に到達できない = panic/異常終了 → pending_failures がインクリメントされたまま)
            if let Some(ref check) = startup_check {
                if let Err(e) = check.mark_healthy(&app_version) {
                    tracing::warn!("startup health: mark_healthy 失敗: {}", e);
                }
            }

            tracing::info!("EasyCursorSwap が正常に起動しました");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("EasyCursorSwap の実行中にエラーが発生しました");
}
