//! EasyCursorSwap システムトレイモジュール
//!
//! システムトレイ（タスクトレイ）への常駐と、トレイメニューの管理を行う。

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

/// システムトレイを初期化する
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // メニューアイテムの作成
    let show_item = MenuItem::with_id(app, "show", "EasyCursorSwap を開く", true, None::<&str>)?;
    let separator1 = MenuItem::with_id(app, "sep1", "────────────", false, None::<&str>)?;
    let panic_default = MenuItem::with_id(
        app,
        "panic_default",
        "🔄 Windows 既定に戻す",
        true,
        None::<&str>,
    )?;
    let panic_initial = MenuItem::with_id(
        app,
        "panic_initial",
        "⏪ インストール前の状態に戻す",
        true,
        None::<&str>,
    )?;
    let separator2 = MenuItem::with_id(app, "sep2", "────────────", false, None::<&str>)?;
    let dark_mode_status = MenuItem::with_id(
        app,
        "dark_mode_status",
        "🌙 ダークモード連動: 無効",
        false,
        None::<&str>,
    )?;
    let separator3 = MenuItem::with_id(app, "sep3", "────────────", false, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;

    // メニューの構築
    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &separator1,
            &panic_default,
            &panic_initial,
            &separator2,
            &dark_mode_status,
            &separator3,
            &quit_item,
        ],
    )?;

    // トレイアイコンの構築
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("EasyCursorSwap")
        .on_menu_event(move |app, event| {
            handle_tray_menu_event(app, event.id().as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            // ダブルクリックでメイン画面を開く
            if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                let app = tray.app_handle();
                show_main_window(app);
            }
        })
        .build(app)?;

    Ok(())
}

/// トレイメニューのイベントハンドラ
fn handle_tray_menu_event(app: &AppHandle, menu_id: &str) {
    match menu_id {
        "show" => {
            show_main_window(app);
        }
        "panic_default" => {
            tracing::info!("パニックボタン: Windows 既定に戻す");
            match crate::registry::RegistryManager::reset_to_windows_default() {
                Ok(_) => {
                    tracing::info!("Windows 既定カーソルに復旧しました");
                }
                Err(e) => {
                    tracing::error!("復旧に失敗: {}", e);
                }
            }
        }
        "panic_initial" => {
            tracing::info!("パニックボタン: インストール前の状態に戻す");
            match crate::registry::RegistryManager::restore_from_initial_snapshot() {
                Ok(_) => {
                    tracing::info!("インストール前のカーソル設定に復旧しました");
                }
                Err(e) => {
                    tracing::error!("復旧に失敗: {}", e);
                }
            }
        }
        "quit" => {
            tracing::info!("アプリケーションを終了します");
            app.exit(0);
        }
        _ => {}
    }
}

/// メインウィンドウを表示する（破棄済みなら再生成）
fn show_main_window(app: &AppHandle) {
    show_or_recreate_main_window(app);
}

/// メインウィンドウが存在すれば前面化、破棄されていれば再生成する。
///
/// 「閉じるボタン押下時に WebView を破棄してメモリを解放、トレイから再オープン時に
/// 復活」という Phase 4-1 のメモリ最適化フローに対応する共通ヘルパー。
/// tray メニュー / 第二インスタンス通知 / グローバルホットキーから共有して呼ぶ。
pub fn show_or_recreate_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }

    // 破棄済み → tauri.conf.json の "main" 定義から再生成
    use tauri::{WebviewWindowBuilder, WebviewUrl};
    match WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("EasyCursorSwap")
        .inner_size(1100.0, 750.0)
        .min_inner_size(900.0, 600.0)
        .resizable(true)
        .center()
        .decorations(true)
        .build()
    {
        Ok(w) => {
            let _ = w.show();
            let _ = w.set_focus();
            tracing::info!("メインウィンドウを再生成しました");
        }
        Err(e) => {
            tracing::error!("メインウィンドウ再生成失敗: {}", e);
        }
    }
}
