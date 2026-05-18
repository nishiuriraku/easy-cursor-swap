//! Windows カーソルスキーム (HKCU\...\Cursors\Schemes) 連携 IPC。
//!
//! ユーザーが「マウスのプロパティ」で保存したスキームを EasyCursorSwap から
//! 一覧 / プレビュー / 適用 / `.cursorpack` エクスポートできるようにする。
//! スキームは編集対象外 — read-only として扱う (export とコピー適用のみ)。

use crate::errors::AppError;
use crate::registry::{RegistryManager, WindowsScheme};
use crate::theme::{RolePreview, ThemeManager};
use serde::Serialize;

/// 指定名のスキームを `HKCU\...\Cursors\Schemes` から検索。未登録ならエラー。
fn lookup_scheme(name: &str) -> Result<WindowsScheme, AppError> {
    RegistryManager::list_windows_schemes()?
        .into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| AppError::Registry(format!("スキーム '{}' が見つかりません", name)))
}

/// 指定スキーム名のロール毎 PNG プレビューを返す。
///
/// `list_windows_schemes` で得たスキーム名を渡すと、各 `.cur` / `.ani` /
/// `.ico` を最大解像度 PNG に変換して `HashMap<role, PNG bytes>` で返す。
/// ファイルが見つからないロールはスキップ (1 つの欠損で全体表示を諦めない)。
#[tauri::command]
pub fn get_windows_scheme_previews(
    name: String,
) -> Result<std::collections::HashMap<String, Vec<u8>>, AppError> {
    let scheme = lookup_scheme(&name)?;
    Ok(ThemeManager::render_paths_as_previews(&scheme.cursor_paths))
}

/// [`get_windows_scheme_previews`] のリッチ版。
/// 各ロールに PNG + ネイティブ寸法 + `.cur` ヘッダ由来のホットスポット座標を返す。
#[tauri::command]
pub fn get_windows_scheme_role_previews(
    name: String,
) -> Result<std::collections::HashMap<String, RolePreview>, AppError> {
    let scheme = lookup_scheme(&name)?;
    Ok(ThemeManager::render_paths_as_previews_with_hotspots(
        &scheme.cursor_paths,
    ))
}

/// 指定 Windows スキームを `.cursorpack` として書き出す。
///
/// `HKCU\Cursors\Schemes` の値が指す各カーソルファイル (.cur / .ani / .ico) を
/// 拡張子を保ったまま zip 化し、theme.json を自動生成する。クリエイターを
/// 通さず、ユーザーが既に Windows 側で構築した配色を共有・バックアップできる。
#[derive(Debug, Serialize)]
pub struct ExportSchemeResult {
    pub theme_id: String,
    pub size_bytes: u64,
}

#[tauri::command]
pub fn export_windows_scheme_as_cursorpack(
    name: String,
    output_path: String,
) -> Result<ExportSchemeResult, AppError> {
    let scheme = lookup_scheme(&name)?;
    let path = std::path::PathBuf::from(&output_path);
    let (id, size) =
        ThemeManager::export_scheme_as_cursorpack(&scheme.name, &scheme.cursor_paths, &path, None)?;
    Ok(ExportSchemeResult {
        theme_id: id.to_string(),
        size_bytes: size,
    })
}

/// `HKCU\Control Panel\Cursors\Schemes` に保存されたカーソルスキーム一覧を返す。
///
/// ライブラリ画面に「Windows のマウスのプロパティに保存済みのスキーム」を
/// マージ表示するために使う。EasyCursorSwap で適用済みのテーマも同じキー配下に
/// 入っているが、それらは `get_themes` 側で重複除去すべき (UI 層で判断)。
#[tauri::command]
pub fn list_windows_schemes() -> Result<Vec<WindowsScheme>, AppError> {
    RegistryManager::list_windows_schemes()
}

/// 指定された Windows スキームをシステムに適用する。
///
/// `apply_theme` (`.cursorpack` 形式の独自テーマ) と区別するため別コマンドにしている。
/// スキームは編集・エクスポート・署名検証の対象外で、適用のみが許される。
#[tauri::command]
pub fn apply_windows_scheme(name: String) -> Result<(), AppError> {
    let scheme = lookup_scheme(&name)?;
    RegistryManager::apply_windows_scheme(&scheme)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `lookup_scheme` は存在しないスキーム名に対し `AppError::Registry` を返し、
    /// メッセージ内にスキーム名を含むこと。
    ///
    /// 実レジストリ (HKCU\Control Panel\Cursors\Schemes) は Windows なら常に
    /// アクセス可能 (Microsoft 既定スキームが入っている) ため、ここでは絶対に
    /// 衝突しないスキーム名で `find` が None になる経路だけを確認する。
    #[test]
    fn lookup_scheme_returns_registry_error_when_not_found() {
        let result = lookup_scheme("__definitely_not_a_real_scheme_xyz__");
        match result {
            Err(AppError::Registry(msg)) => {
                assert!(
                    msg.contains("__definitely_not_a_real_scheme_xyz__"),
                    "error message should include the scheme name, got: {}",
                    msg
                );
            }
            other => panic!(
                "expected AppError::Registry for unknown scheme, got {:?}",
                other
            ),
        }
    }
}
