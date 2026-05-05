//! CursorForge - メインエントリポイント
//!
//! Tauri アプリケーションの初期化、トレイ常駐、ダークモード監視を統括する。

// リリースビルドではコンソールウィンドウを非表示にする
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app_lib::commands;
use app_lib::config::ConfigManager;
use app_lib::cursor_watcher;
use app_lib::darkmode;
use app_lib::logging;
use app_lib::registry::RegistryManager;
use app_lib::single_instance::SingleInstanceLock;
use app_lib::tray;

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

    tracing::info!("CursorForge v{} を起動しています...", env!("CARGO_PKG_VERSION"));

    // 多重起動防止: Named Mutex を取得。既存インスタンスがあれば中断。
    // _instance_lock は drop 時にミューテックスを解放するので main の最後まで保持。
    let _instance_lock = match SingleInstanceLock::acquire() {
        Ok(lock) => lock,
        Err(e) => {
            tracing::warn!("多重起動を検出: {}", e);
            // TODO: 既存インスタンスのトレイアイコンへフォーカスを移す
            //       (CreateEvent 経由のシグナル + 既存側で待機)
            eprintln!("CursorForge は既に起動しています");
            return;
        }
    };

    // 設定マネージャー初期化
    let config_manager = match ConfigManager::init() {
        Ok(cm) => cm,
        Err(e) => {
            tracing::error!("設定の初期化に失敗: {}", e);
            // 設定読み込み失敗時はデフォルト設定で続行せず中断
            // （仕様: マイグレーション失敗時はアプリ起動を中断）
            eprintln!("設定の初期化に失敗しました: {}", e);
            return;
        }
    };

    // 初回起動時のスナップショット保存
    if let Err(e) = RegistryManager::save_initial_snapshot() {
        tracing::warn!("初回スナップショットの保存に失敗: {}", e);
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

    // Tauri アプリケーションビルド
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(config_manager)
        .invoke_handler(commands::get_command_handlers())
        .setup(|app| {
            let handle = app.handle().clone();

            // システムトレイの初期化
            if let Err(e) = tray::setup_tray(&handle) {
                tracing::error!("システムトレイの初期化に失敗: {}", e);
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

            tracing::info!("CursorForge が正常に起動しました");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("CursorForge の実行中にエラーが発生しました");
}
