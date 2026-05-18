//! テーマ適用 / アクティブ判定。
//!
//! - `apply_theme`: 指定 ID をレジストリに反映 (スナップショット保存 → 書込 → SPI_SETCURSORS)
//! - `theme_active_in_registry`: 現在の HKCU\Control Panel\Cursors と一致するか
//!
//! `theme/mod.rs` から分割 (2026-05-18, refactor/yellow-items)。

use super::types::ThemeMetadata;
use super::ThemeManager;
use crate::errors::AppResult;
use std::collections::HashMap;
use uuid::Uuid;

impl ThemeManager {
    /// 指定 ID のテーマが現在実際にレジストリに適用されているかを判定する。
    ///
    /// `theme.json` に書かれた各役割の絶対パスと、`HKCU\Control Panel\Cursors`
    /// の現在値を比較する。ユーザーが Windows のマウスのプロパティで別スキーム
    /// に切り替えた / 既定へリセットした直後はここで `false` になり、UI の
    /// "active" 表示も外れる。
    pub fn theme_active_in_registry(id: Uuid) -> bool {
        use crate::config::ConfigManager;
        use crate::registry::paths_match_current_registry;

        let Ok(cursors_dir) = ConfigManager::cursors_dir() else {
            return false;
        };
        let theme_dir = cursors_dir.join(id.to_string());
        let theme_json_path = theme_dir.join("theme.json");
        if !theme_json_path.is_file() {
            return false;
        }
        let content = match std::fs::read_to_string(&theme_json_path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let metadata: ThemeMetadata = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(_) => return false,
        };
        let mut expected: HashMap<String, String> = HashMap::new();
        for (role, def) in &metadata.cursors {
            let abs = theme_dir.join(&def.file);
            expected.insert(role.clone(), abs.to_string_lossy().to_string());
        }
        paths_match_current_registry(&expected)
    }

    /// 指定 ID のテーマを探してレジストリに適用する。
    ///
    /// 流れ:
    /// 1. `~/.custom_cursors/<theme>/theme.json` を走査して該当テーマを検索
    /// 2. metadata.cursors から `役割名 → 絶対カーソルファイルパス` のマップを構築
    /// 3. `RegistryManager::apply_cursors` でレジストリ書き込み + SPI_SETCURSORS
    ///
    /// 内部で `RegistryManager` がスナップショット保存・失敗時ロールバックを担う。
    pub fn apply_theme(id: Uuid) -> AppResult<()> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        use crate::registry::RegistryManager;
        use std::path::PathBuf;

        let cursors_dir = ConfigManager::cursors_dir()?;

        // 該当 ID のテーマディレクトリを線形探索 (テーマ数は通常 < 100)
        let mut target: Option<(PathBuf, ThemeMetadata)> = None;
        if cursors_dir.exists() {
            for entry in std::fs::read_dir(&cursors_dir)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('_') {
                        continue;
                    }
                }
                let theme_json = path.join("theme.json");
                if !theme_json.exists() {
                    continue;
                }
                let content = std::fs::read_to_string(&theme_json)?;
                let metadata: ThemeMetadata = serde_json::from_str(&content)?;
                if metadata.id == id {
                    target = Some((path, metadata));
                    break;
                }
            }
        }

        let (theme_dir, metadata) =
            target.ok_or_else(|| AppError::Theme(format!("テーマ {} が見つかりません", id)))?;

        // 役割名 → 絶対パスのマップを構築
        let mut cursor_paths: HashMap<String, PathBuf> = HashMap::new();
        for (role, def) in &metadata.cursors {
            let abs = theme_dir.join(&def.file);
            if !abs.exists() {
                tracing::warn!(
                    "カーソルファイルが存在しない: {} ({})",
                    role,
                    crate::logging::redact_path(&abs)
                );
                continue;
            }
            cursor_paths.insert(role.clone(), abs);
        }

        if cursor_paths.is_empty() {
            return Err(AppError::Theme(
                "適用可能なカーソルファイルが見つかりません".to_string(),
            ));
        }

        RegistryManager::apply_cursors(&cursor_paths)?;

        // Windows のコントロールパネルから参照可能なよう Schemes にも登録する。
        // 失敗しても適用自体は成功扱い (UX への影響は最小限)
        let scheme_name = format!("EasyCursorSwap - {}", metadata.name.get("ja"));
        if let Err(e) = RegistryManager::register_scheme(&scheme_name, &cursor_paths) {
            tracing::warn!("Schemes 登録に失敗 (適用自体は成功): {}", e);
        }

        // OS 標準ポインターの影制御
        if let Err(e) = RegistryManager::set_cursor_shadow(metadata.requires_os_shadow) {
            tracing::warn!("ポインター影設定の更新に失敗: {}", e);
        }

        tracing::info!("テーマ {} を適用しました", metadata.name.get("ja"));
        Ok(())
    }
}
