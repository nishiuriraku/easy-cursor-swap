//! EasyCursorSwap Tauri IPC コマンド定義
//!
//! フロントエンド (Nuxt) から呼び出し可能なコマンドを定義する。
//! 各コマンドは Tauri の `#[tauri::command]` マクロで公開され、
//! [`get_command_handlers`] が `tauri::Builder.invoke_handler()` に渡される。
//!
//! 責務別にサブモジュール分割している:
//!
//! | モジュール | 担当領域 |
//! |---|---|
//! | [`cursor_build`] | `.cur` ビルド / `.cursorpack` エクスポート (同期 / ストリーミング) / 署名 |
//! | [`cursor_io`]    | 単一 `.cur` / `.ico` / `.ani` ファイルの読み込み |
//! | [`keystore`]     | Ed25519 鍵ペア管理 (生成 / 削除 / Export / Import) |
//! | [`marketplace`]  | 公式インデックス取得 / インストール |
//! | [`profile`]      | `.cursorprofile` (config + 全テーマ) の export / import |
//! | [`system`]       | アプリ設定 / OS 状態 / クラッシュ / 自動起動など雑多な情報系 |
//! | [`theme`]        | テーマ単体の CRUD と `.cursorpack` の inspect / import |
//! | [`windows_scheme`] | Windows カーソルスキーム (HKCU\Cursors\Schemes) 連携 |
//!
//! `bulk_import` 系 IPC は [`crate::bulk_import`] に直接定義されている (キャンセル可能なバックグラウンド処理を伴うため)。

pub mod cursor_build;
pub mod cursor_io;
pub mod keystore;
pub mod marketplace;
pub mod profile;
pub mod system;
pub mod theme;
pub mod windows_scheme;

/// Tauri Builder に全コマンドを登録するためのヘルパー。
///
/// `main.rs` の `tauri::Builder.invoke_handler()` から 1 回だけ呼ばれる。
/// 新しい IPC コマンドを追加するときは、対応する `commands/<module>.rs` に置き、
/// このマクロのリストに `<module>::<fn_name>` を追加する。
pub fn get_command_handlers() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![
        // テーマ
        theme::get_cursor_roles,
        theme::get_current_cursors,
        theme::get_themes,
        theme::get_theme_previews,
        theme::apply_theme,
        theme::set_theme_favorite,
        theme::clear_cursor_cache,
        theme::inspect_cursorpack,
        theme::import_cursorpack,
        theme::delete_theme,
        theme::duplicate_theme,
        theme::repackage_theme,
        // .cur / .cursorpack ビルド
        cursor_build::build_cursor_file,
        cursor_build::export_cursorpack,
        cursor_build::export_cursorpack_streamed,
        cursor_build::cancel_build,
        // .cur / .ico / .ani 取り込み
        cursor_io::import_cursor_file,
        cursor_io::inspect_ani_file,
        // 鍵管理
        keystore::keystore_info,
        keystore::keystore_generate,
        keystore::keystore_delete,
        keystore::keystore_export,
        keystore::keystore_import,
        // 公式インデックス
        marketplace::marketplace_fetch_index,
        marketplace::marketplace_install,
        // バックアッププロファイル
        profile::export_profile,
        profile::import_profile,
        // Windows カーソルスキーム
        windows_scheme::list_windows_schemes,
        windows_scheme::apply_windows_scheme,
        windows_scheme::get_windows_scheme_previews,
        windows_scheme::export_windows_scheme_as_cursorpack,
        // システム / 設定 / 診断
        system::reset_to_default,
        system::reset_to_initial,
        system::get_dark_mode_status,
        system::get_environment_report,
        system::get_config,
        system::update_config,
        system::get_app_info,
        system::list_config_backups,
        system::restore_config_backup,
        system::check_update_is_major_jump,
        system::open_url,
        system::get_accessibility_conflicts,
        system::get_autostart_status,
        system::list_crash_reports,
        system::clear_crash_reports,
        system::submit_crash_reports,
        // 一括取り込み (実装本体は crate::bulk_import::{assets,cursorpack} 側)
        crate::bulk_import::assets::bulk_resolve_assets,
        crate::bulk_import::assets::cancel_bulk_import,
        crate::bulk_import::cursorpack::parse_cursorpack_for_creator,
    ]
}
