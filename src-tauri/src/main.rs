//! EasyCursorSwap - メインエントリポイント
//!
//! Tauri アプリケーションの初期化とトレイ常駐を統括する。

// リリースビルドではコンソールウィンドウを非表示にする
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app_lib::appusermodel;
use app_lib::autostart;
use app_lib::cancel_registry;
use app_lib::commands;
use app_lib::commands::cursor_io::{
    handle_pending_cursorpack, stash_pending_cursorpack, PendingCursorpack,
};
use app_lib::config::ConfigManager;
use app_lib::crash;
use app_lib::cursor_watcher;
use app_lib::health::{RollbackTarget, StartupCheck};
use app_lib::hotkey;
use app_lib::logging;
use app_lib::registry::RegistryManager;
use app_lib::tray;

/// 連続起動失敗 3 回検出時のロールバック案内ダイアログ。
/// ユーザーが「はい」を選んだ場合、自動で旧版インストーラを DL → 検証 →
/// サイレント再インストールする。失敗時はブラウザ fallback。
#[cfg(windows)]
fn show_rollback_dialog(target: &RollbackTarget) {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_ICONWARNING, MB_TOPMOST, MB_YESNO,
    };

    let body = format!(
        "EasyCursorSwap が 3 回連続して正常に起動できませんでした。\n\n\
        前バージョン v{} に自動でロールバックしますか?\n\n\
        「はい」を選択すると旧版インストーラを自動でダウンロードして再インストールします。\n\
        ダウンロード後の検証 (Ed25519) に失敗した場合はブラウザでリリースページを開いて手動回復に切り替えます。",
        target.version
    );
    let title = HSTRING::from("EasyCursorSwap — 起動失敗を検出");
    let body_h = HSTRING::from(body);

    let result = unsafe {
        MessageBoxW(
            None,
            &body_h,
            &title,
            MB_YESNO | MB_ICONWARNING | MB_TOPMOST,
        )
    };
    if result != IDYES {
        return;
    }

    match auto_rollback_install(target) {
        Ok(()) => {
            tracing::info!("rollback installer 起動成功。プロセスを終了します");
            std::process::exit(0);
        }
        Err(e) => {
            tracing::warn!("自動ロールバック失敗: {} → ブラウザ fallback", e);
            open_release_page_in_browser(&target.releases_page_url);
        }
    }
}

/// 旧版インストーラを DL → minisign 検証 → サイレント起動。
#[cfg(windows)]
fn auto_rollback_install(target: &RollbackTarget) -> Result<(), app_lib::rollback::RollbackError> {
    use app_lib::rollback;
    tracing::info!("rollback: {} の DL を開始", target.installer_url);
    let installer = rollback::download_to_temp(
        &target.installer_url,
        &installer_filename_from_url(&target.installer_url),
    )?;
    tracing::info!("rollback: 署名検証中 (.sig DL)");
    let sig_url = format!("{}.sig", target.installer_url);
    let sig_path = rollback::download_to_temp(&sig_url, "rollback-installer.sig")?;
    let installer_bytes = std::fs::read(&installer)?;
    let sig_text = std::fs::read_to_string(&sig_path)?;
    rollback::verify_minisign(&installer_bytes, &sig_text, rollback::EMBEDDED_PUBKEY)?;
    tracing::info!("rollback: 検証 OK、サイレントインストール起動");
    rollback::launch_silent_installer(&installer)?;
    Ok(())
}

#[cfg(windows)]
fn open_release_page_in_browser(url: &str) {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let url_h = HSTRING::from(url);
    let verb = HSTRING::from("open");
    unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(url_h.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

/// `https://.../EasyCursorSwap_0.1.0_x64-setup.exe` → `EasyCursorSwap_0.1.0_x64-setup.exe`
fn installer_filename_from_url(url: &str) -> String {
    url.rsplit('/')
        .next()
        .unwrap_or("rollback-installer.exe")
        .to_string()
}

#[cfg(not(windows))]
fn show_rollback_dialog(_target: &RollbackTarget) {}

/// 設定マイグレーション失敗時の専用ダイアログ。
/// Win32 MessageBox で「バックアップの場所」と「復旧手順」を表示する。
/// Tauri ランタイムを起動できる前のフェーズなので Tauri Dialog ではなく Win32 を直接呼ぶ。
#[cfg(windows)]
fn show_migration_failure_dialog(err: &str) {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK, MB_TOPMOST};

    let config_dir = dirs::data_local_dir()
        .map(|p| p.join("EasyCursorSwap").to_string_lossy().to_string())
        .unwrap_or_else(|| "%LOCALAPPDATA%\\EasyCursorSwap".to_string());

    let body = format!(
        "EasyCursorSwap の設定ファイルを読み込めませんでした。\n\n\
        理由: {err}\n\n\
        バックアップ:\n  {config_dir}\\config.corrupt.*.json\n\n\
        ファイルをリネームして config.json に戻すと前回状態に復旧できます。\n\
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
    // panic フックを最優先で仕込む。ロギング初期化前の panic もファイルに記録できるよう
    // ここで設定する。デフォルトの stderr 出力フックは内部で温存される。
    crash::install_panic_hook();
    // 起動時に古いクラッシュレポート (30 日経過) を掃除。失敗してもアプリは続行。
    if let Err(e) = crash::prune_old_reports() {
        eprintln!("[crash] prune_old_reports warn: {}", e);
    }

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

    tracing::info!(
        "EasyCursorSwap v{} を起動しています...",
        env!("CARGO_PKG_VERSION")
    );

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
            tracing::warn!("連続起動失敗 3 回を検出。ロールバックダイアログを表示します。");
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

    // 多重起動防止 + argv ハンドオーバは tauri-plugin-single-instance に集約。
    // 詳細は builder の .plugin(tauri_plugin_single_instance::init(...)) を参照。

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
    // release ビルドでは debug_assertions ブロックが消えて再代入が無くなるため
    // `mut` が unused 扱いになる。条件付きで allow する。
    #[cfg_attr(not(debug_assertions), allow(unused_mut))]
    let mut builder = tauri::Builder::default();
    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_mcp_bridge::init());
    }
    builder
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            // 2 重起動時: 既存インスタンス側で実行される callback。
            let cwd_path = std::path::PathBuf::from(&cwd);
            handle_pending_cursorpack(app, &argv, &cwd_path);
            // ウィンドウ前面化 (破棄されてトレイ常駐中なら再生成)
            use tauri::Manager;
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            } else {
                tray::show_or_recreate_main_window(app);
            }
            tracing::info!("第二インスタンス要求でメインウィンドウを前面化");
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(config_manager)
        .manage(cancel_registry::CancelRegistry::default())
        .manage(PendingCursorpack::default())
        .manage(crate::commands::marketplace_submit::DeviceFlowState::default())
        // 閉じるボタン → WebView を破棄してメモリ解放 (Phase 4-1)
        // アプリ自体はトレイに常駐し続ける。再表示時に tray::show_or_recreate_main_window が
        // ウィンドウを再生成する。
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.destroy();
                    tracing::info!("メインウィンドウを破棄しました (トレイ常駐 / メモリ解放)");
                }
            }
        })
        .invoke_handler(commands::get_command_handlers())
        .setup(move |app| {
            let handle = app.handle().clone();

            // 初回起動時の argv に .cursorpack があれば PendingCursorpack に積む。
            // フロントは take_pending_cursorpack IPC でマウント後にプルする。
            let argv: Vec<String> = std::env::args().collect();
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            stash_pending_cursorpack(&handle, &argv, &cwd);

            // システムトレイの初期化
            if let Err(e) = tray::setup_tray(&handle) {
                tracing::error!("システムトレイの初期化に失敗: {}", e);
            }

            // (旧 single_instance::start_show_window_listener は
            //  tauri-plugin-single-instance の callback に統合した)

            // パニックホットキー (`Ctrl+Alt+Shift+R` 等 config 値) の登録
            // 押下時はフロントへ `panic-hotkey` イベントを発火し、PanicFlow を起動させる
            let hotkey_handle = handle.clone();
            let spec = hotkey_spec.clone();
            if let Err(e) = hotkey::register_panic_hotkey(&spec, move || {
                use tauri::Emitter;
                tracing::info!("panic-hotkey イベントを発火");
                // 破棄されていれば再生成してから前面化
                tray::show_or_recreate_main_window(&hotkey_handle);
                if let Err(err) = hotkey_handle.emit("panic-hotkey", ()) {
                    tracing::warn!("panic-hotkey emit 失敗: {}", err);
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

            // クラッシュレポート自動送信。発火条件:
            //   1. ビルド時 env で endpoint+token が埋め込まれている (release/CI ビルドのみ)
            //   2. ユーザーが crash_reporting オプトインを true にしている
            // ベストエフォート: 失敗してもアプリ動作には影響しない。
            if let Some((endpoint, token)) = crash::embedded_credentials() {
                let submit_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    use tauri::Manager;
                    let cfg_state: tauri::State<ConfigManager> = submit_handle.state();
                    let cfg = match cfg_state.get() {
                        Ok(c) => c,
                        Err(err) => {
                            tracing::debug!("crash-submit: config 取得失敗 (skip): {}", err);
                            return;
                        }
                    };
                    if !cfg.general.crash_reporting {
                        return;
                    }
                    match crash::submit_pending_reports(endpoint, token).await {
                        Ok(s) => {
                            if s.sent + s.failed + s.skipped > 0 {
                                tracing::info!(
                                    "crash-submit: sent={} failed={} skipped={}",
                                    s.sent,
                                    s.failed,
                                    s.skipped
                                );
                            }
                        }
                        Err(e) => {
                            tracing::debug!("crash-submit: 起動時送信エラー (silent): {}", e);
                        }
                    }
                });
            } else {
                tracing::debug!(
                    "crash-submit: ビルド時 env 未設定のため自動送信を無効化 (機能ごと skip)"
                );
            }

            tracing::info!("EasyCursorSwap が正常に起動しました");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("EasyCursorSwap の実行中にエラーが発生しました")
        .run(|_app_handle, event| {
            // 閉じるボタンで `window.destroy()` した後もトレイ常駐させるための gatekeeper。
            // Tauri v2 はデフォルトで「最後のウィンドウ消滅 → プロセス終了」だが、
            // 本アプリは tray + global hotkey + cursor_watcher + auto_start silent boot を
            // 抱えているのでウィンドウが無くてもプロセスは生かす必要がある。
            //
            // `code: None` はユーザー操作 (ウィンドウ閉じる) 起点のリクエスト、
            // `code: Some(_)` は `AppHandle::exit(_)` / `AppHandle::restart()` などの
            // プログラム要求。tray メニューの「終了」は app.exit(0) を呼ぶので Some(0) で来る。
            // 前者だけ prevent_exit して、後者はそのまま終了させる。
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}
