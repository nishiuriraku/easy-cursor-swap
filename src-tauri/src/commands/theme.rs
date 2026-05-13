//! テーマ単体に対する CRUD 系 IPC。
//!
//! - 一覧 / プレビュー取得
//! - 適用 (active_theme_id を config に永続化)
//! - 削除 / 複製 / `.cursorpack` 再エクスポート
//! - `.cursorpack` の inspect / import
//! - 17 ロール定義の取得 (`get_cursor_roles`)
//!
//! .cur ビルドや署名フローは [`super::cursor_build`] (build / cancel / dto / sign / stream に分割済み)、
//! Windows スキーム連携は [`super::windows_scheme`] にある。

use crate::config::ConfigManager;
use crate::cursor::clear_resize_cache;
use crate::errors::AppError;
use crate::registry::{CursorRole, RegistryManager};
use crate::theme::{CursorpackInspection, RolePreview, ThemeManager, ThemeSummary};
use serde::Serialize;
use tauri::State;

/// フロントエンドに返すカーソル役割情報
#[derive(Debug, Serialize)]
pub struct CursorRoleInfo {
    /// レジストリ値名
    pub id: String,
    /// 日本語表示名
    pub name_ja: String,
    /// 英語表示名
    pub name_en: String,
    /// Schemes 内でのインデックス
    pub index: usize,
}

/// 全17種のカーソル役割情報を返す
#[tauri::command]
pub fn get_cursor_roles() -> Vec<CursorRoleInfo> {
    CursorRole::all()
        .iter()
        .map(|role| CursorRoleInfo {
            id: role.registry_name().to_string(),
            name_ja: role.display_name_ja().to_string(),
            name_en: role.display_name_en().to_string(),
            index: role.scheme_index(),
        })
        .collect()
}

/// 現在のカーソル設定をレジストリから読み取る
#[tauri::command]
pub fn get_current_cursors() -> Result<std::collections::HashMap<String, String>, AppError> {
    RegistryManager::read_current_cursors()
}

/// テーマ一覧を取得する。
///
/// `is_active` は config の `active_theme_id` に加えてレジストリ実態を検証する。
/// Windows 側で別スキームに切り替えられたり、リセットされたりした場合は
/// 該当テーマの `is_active` を **false** にして返し、`config` 側の
/// `active_theme_id` もクリアする (Source of Truth はレジストリ)。
#[tauri::command]
pub fn get_themes(config: State<'_, ConfigManager>) -> Result<Vec<ThemeSummary>, AppError> {
    let cfg = config.get()?;
    let mut active_id = cfg.general.active_theme_id;

    // 実態と乖離していれば clear (例: ユーザーが Windows のマウスのプロパティで
    // 別スキームを選択 / 既定にリセットした直後)
    if let Some(id) = active_id {
        if !ThemeManager::theme_active_in_registry(id) {
            tracing::info!(
                "active_theme_id={} はレジストリ実態と一致しないためクリアします",
                id
            );
            let _ = config.update(|c| c.general.active_theme_id = None);
            active_id = None;
        }
    }

    ThemeManager::list_themes(active_id, &cfg.general.favorites, &cfg.general.usage)
}

/// テーマのお気に入りフラグを永続化する。
/// 戻り値は更新後のお気に入り ID リスト (UI でクライアントキャッシュを更新する用途)。
#[tauri::command]
pub fn set_theme_favorite(
    config: State<'_, ConfigManager>,
    theme_id: String,
    is_favorite: bool,
) -> Result<Vec<String>, AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    let updated = config.update(|c| {
        let already = c.general.favorites.contains(&id);
        if is_favorite && !already {
            c.general.favorites.push(id);
        } else if !is_favorite && already {
            c.general.favorites.retain(|x| x != &id);
        }
    })?;
    Ok(updated
        .general
        .favorites
        .iter()
        .map(|u| u.to_string())
        .collect())
}

/// 指定テーマのロール毎 PNG プレビューを返す。
///
/// `roles` が空配列なら全ロールを返す。値が指定されていればそのロールのみ。
/// レスポンスは `HashMap<role, PNG bytes>` で、IPC では `Vec<u8>` がそのまま JSON 配列化される。
#[tauri::command]
pub fn get_theme_previews(
    theme_id: String,
    roles: Vec<String>,
) -> Result<std::collections::HashMap<String, Vec<u8>>, AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    let filter: Option<&[String]> = if roles.is_empty() { None } else { Some(&roles) };
    ThemeManager::load_role_previews(id, filter)
}

/// [`get_theme_previews`] のリッチ版。各ロールに PNG + 寸法 + ホットスポット座標を返す。
///
/// テーマ詳細ドロワーで「ホットスポットの位置」を視覚化する用途のみ使用。
/// 旧 [`get_theme_previews`] はテーマカードのサムネ等で使い続ける (ペイロード軽量)。
#[tauri::command]
pub fn get_theme_role_previews(
    theme_id: String,
    roles: Vec<String>,
) -> Result<std::collections::HashMap<String, RolePreview>, AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    let filter: Option<&[String]> = if roles.is_empty() { None } else { Some(&roles) };
    ThemeManager::load_role_previews_with_hotspots(id, filter)
}

/// 指定 ID のテーマをシステムに適用する。
/// 失敗時は内部のスナップショットから自動ロールバックされる。
/// 成功時は config の `active_theme_id` と `usage` を更新して永続化する。
#[tauri::command]
pub fn apply_theme(config: State<'_, ConfigManager>, theme_id: String) -> Result<(), AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    ThemeManager::apply_theme(id)?;
    // 適用成功 → アクティブテーマ ID + 利用統計を永続化
    let now = chrono::Utc::now().to_rfc3339();
    config.update(|c| {
        c.general.active_theme_id = Some(id);
        let entry = c.general.usage.entry(id).or_default();
        entry.apply_count = entry.apply_count.saturating_add(1);
        entry.last_applied_at = Some(now);
    })?;
    Ok(())
}

/// リサイズ結果キャッシュをクリアする。
/// クリエイターで素材を差し替えた直後など、明示的にメモリを開放したいときに使用。
#[tauri::command]
pub fn clear_cursor_cache() {
    clear_resize_cache();
    tracing::info!("リサイズキャッシュをクリアしました");
}

/// `.cursorpack` をインポートする前のメタデータ検査。
/// 既存ライブラリに同 ID のテーマがあればバージョン比較情報を返す。
#[tauri::command]
pub fn inspect_cursorpack(path: String) -> Result<CursorpackInspection, AppError> {
    let buf = std::path::PathBuf::from(&path);
    if !buf.exists() {
        return Err(AppError::Theme(format!(
            "ファイルが見つかりません: {}",
            path
        )));
    }
    ThemeManager::inspect_cursorpack_file(&buf)
}

/// ローカルの `.cursorpack` ファイルをライブラリにインポートする。
/// パストラバーサル / Zip 爆弾 / シンボリックリンク防御つきで展開し、
/// 戻り値として展開後のテーマ ID (UUID 文字列) を返す。
#[tauri::command]
pub fn import_cursorpack(path: String) -> Result<String, AppError> {
    let buf = std::path::PathBuf::from(&path);
    if !buf.exists() {
        return Err(AppError::Theme(format!(
            "ファイルが見つかりません: {}",
            path
        )));
    }
    // 拡張子を弱バリデーション (Magic Byte は ThemeManager 内で再チェック)
    let ext_ok = buf
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("cursorpack"))
        .unwrap_or(false);
    if !ext_ok {
        return Err(AppError::Theme(
            ".cursorpack 以外の拡張子は受け入れません".to_string(),
        ));
    }
    let id = ThemeManager::import_cursorpack_file(&buf)?;
    Ok(id.to_string())
}

/// 指定 ID のテーマを ~/.custom_cursors/<UUID>/ ごと完全削除する。
///
/// 削除されたテーマがアクティブだった場合、呼び出し側 (UI) は config の
/// active_theme_id をクリアする責任を持つ。Windows 側はファイル不在時に
/// 既定カーソルへフォールバックするので追加処理は不要。
#[tauri::command]
pub fn delete_theme(config: State<'_, ConfigManager>, theme_id: String) -> Result<(), AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    ThemeManager::delete_theme(id)?;
    // 削除されたテーマが active なら config 側もクリアする
    if let Ok(c) = config.get() {
        if c.general.active_theme_id == Some(id) {
            let _ = config.update(|c| c.general.active_theme_id = None);
        }
    }
    Ok(())
}

/// 指定 ID のテーマを複製する。新テーマの UUID を返す。
#[tauri::command]
pub fn duplicate_theme(theme_id: String) -> Result<String, AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    let new_id = ThemeManager::duplicate_theme(id)?;
    Ok(new_id.to_string())
}

/// 既存ライブラリのテーマを `.cursorpack` ファイルに書き出す。
///
/// クリエイターを介さずライブラリ画面からそのままエクスポートできるよう、
/// `~/.custom_cursors/<UUID>/` を ZIP 化して指定パスに保存する。戻り値は
/// 書き込んだバイト数。
#[tauri::command]
pub fn repackage_theme(theme_id: String, output_path: String) -> Result<u64, AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    let path = std::path::PathBuf::from(&output_path);
    ThemeManager::repackage_theme(id, &path)
}
