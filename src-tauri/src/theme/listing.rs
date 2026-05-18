//! テーマ一覧取得 / metadata 読込 / 孤児カーソル復旧 / お気に入り判定。
//!
//! `theme/mod.rs` から分割 (2026-05-18, refactor/yellow-items)。
//! `apply_theme` 周りは [`super::apply`]、`.cursorpack` zip 入出力は
//! [`super::package`]、PNG プレビュー生成は [`super::preview`] を参照。

use super::types::{self, ThemeMetadata, ThemeSummary};
use super::ThemeManager;
use crate::errors::AppResult;
use uuid::Uuid;

/// 指定テーマディレクトリの `theme.json` を読んで `source` フィールドのみ書き換える。
/// `duplicate_theme` での Local リセットと `marketplace::install` での Marketplace 書込で共有する。
pub(crate) fn set_metadata_source(
    theme_dir: &std::path::Path,
    source: types::ThemeSource,
) -> AppResult<()> {
    let path = theme_dir.join("theme.json");
    let content = std::fs::read_to_string(&path)?;
    let mut metadata: ThemeMetadata = serde_json::from_str(&content)?;
    metadata.source = source;
    let new_content = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(&path, new_content)?;
    Ok(())
}

impl ThemeManager {
    /// 指定 ID の theme.json をパースして返す。
    /// `source` フィールドのチェックなど、テーマ単体のメタデータが欲しい場面で使う。
    pub fn load_metadata(id: Uuid) -> AppResult<ThemeMetadata> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        let cursors_dir = ConfigManager::cursors_dir()?;
        let path = cursors_dir.join(id.to_string()).join("theme.json");
        if !path.is_file() {
            return Err(AppError::Theme(format!("テーマ {} が見つかりません", id)));
        }
        let content = std::fs::read_to_string(&path)?;
        let metadata: ThemeMetadata = serde_json::from_str(&content)?;
        Ok(metadata)
    }

    /// 指定 ID のテーマがディスク上に存在するかを確認する。
    /// `~/.custom_cursors/<UUID>/theme.json` の存在のみで判定する (中身は検証しない)。
    pub fn theme_exists(id: Uuid) -> bool {
        use crate::config::ConfigManager;
        let cursors_dir = match ConfigManager::cursors_dir() {
            Ok(d) => d,
            Err(_) => return false,
        };
        cursors_dir
            .join(id.to_string())
            .join("theme.json")
            .is_file()
    }

    /// 起動時の孤児カーソル復旧チェック。
    ///
    /// config が指すテーマ ID (`active_theme_id`) がディスク上に存在しない場合、
    /// レジストリを Windows 既定に戻し、`active_theme_id = None` に戻す。
    ///
    /// 何もする必要がなければ `Ok(false)` を返す。復旧した場合は `Ok(true)`。
    pub fn cleanup_orphan_references(config: &crate::config::ConfigManager) -> AppResult<bool> {
        use crate::registry::RegistryManager;

        let cfg = config.get()?;
        let active_orphan = cfg
            .general
            .active_theme_id
            .is_some_and(|id| !Self::theme_exists(id));

        if !active_orphan {
            return Ok(false);
        }

        tracing::warn!(
            "孤児カーソル検出: active_theme_id={:?} のディレクトリが消失 → Windows 既定へ復元",
            cfg.general.active_theme_id
        );
        if let Err(e) = RegistryManager::reset_to_windows_default() {
            tracing::warn!("孤児復旧時の Windows 既定への戻し失敗: {}", e);
        }

        config.update(|c| {
            c.general.active_theme_id = None;
        })?;
        Ok(true)
    }

    /// インストール済みテーマの一覧を取得する。
    /// `active_id` (config.general.active_theme_id) と一致するテーマだけ
    /// `is_active = true` で返却する。
    /// `favorites` / `usage` は config.general から渡し、各サマリーに反映する。
    pub fn list_themes(
        active_id: Option<Uuid>,
        favorites: &[Uuid],
        usage: &std::collections::HashMap<Uuid, crate::config::ThemeUsage>,
    ) -> AppResult<Vec<ThemeSummary>> {
        use crate::config::ConfigManager;

        let cursors_dir = ConfigManager::cursors_dir()?;
        let mut themes = Vec::new();

        if !cursors_dir.exists() {
            return Ok(themes);
        }

        // ~/.custom_cursors/ 配下の各ディレクトリをスキャン
        for entry in std::fs::read_dir(&cursors_dir)? {
            let entry = entry?;
            let path = entry.path();

            // _で始まる特殊ディレクトリはスキップ
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('_') {
                    continue;
                }
            }

            if !path.is_dir() {
                continue;
            }

            // theme.json を読み込む
            let theme_json_path = path.join("theme.json");
            if !theme_json_path.exists() {
                continue;
            }

            match Self::load_theme_summary(&theme_json_path, &path, active_id, favorites, usage) {
                Ok(summary) => themes.push(summary),
                Err(e) => {
                    tracing::warn!(
                        "テーマの読み込みに失敗 ({}): {}",
                        crate::logging::redact_path(&path),
                        e
                    );
                }
            }
        }

        Ok(themes)
    }

    /// theme.json からサマリー情報を読み込む。
    ///
    /// `favorites` / `usage` は呼び出し側 (`list_themes`) で 1 回だけ取り出した
    /// 値を共有して渡し、ディレクトリ走査の毎回 config を読み直すのを避ける。
    fn load_theme_summary(
        theme_json_path: &std::path::Path,
        theme_dir: &std::path::Path,
        active_id: Option<Uuid>,
        favorites: &[Uuid],
        usage: &std::collections::HashMap<Uuid, crate::config::ThemeUsage>,
    ) -> AppResult<ThemeSummary> {
        let content = std::fs::read_to_string(theme_json_path)?;
        let metadata: ThemeMetadata = serde_json::from_str(&content)?;

        // schema_version 検証: v1 のみ対応 (リリース前のため将来のマイグレーション窓口だけ用意)
        if metadata.schema_version != 1 {
            tracing::warn!(
                "テーマ {} スキップ: 非対応 schema_version {} (expected 1)",
                crate::logging::short_hash(metadata.id.to_string().as_bytes()),
                metadata.schema_version
            );
            return Err(crate::errors::AppError::Theme(format!(
                "schema_version {} は非対応 (expected 1)",
                metadata.schema_version
            )));
        }

        let included_roles: Vec<String> = metadata.cursors.keys().cloned().collect();
        let is_active = active_id == Some(metadata.id);
        let tags = metadata.tags.clone();
        let size_bytes = Self::dir_size_bytes(theme_dir);
        let signed = metadata.signature.is_some();
        let is_favorite = favorites.contains(&metadata.id);
        let usage_entry = usage.get(&metadata.id).cloned().unwrap_or_default();

        Ok(ThemeSummary {
            id: metadata.id,
            name: metadata.name.get("ja"), // TODO: ロケールに応じて切替
            author: metadata.author,
            version: metadata.version,
            created_at: metadata.created_at,
            is_active,
            is_favorite,
            apply_count: usage_entry.apply_count,
            last_applied_at: usage_entry.last_applied_at,
            included_roles,
            path: theme_dir.to_string_lossy().to_string(),
            tags,
            size_bytes,
            signed,
            description: metadata.description.as_ref().map(|d| d.get("ja")),
            schema_version: metadata.schema_version,
            license: metadata.license.clone(),
            homepage: metadata.homepage.clone(),
            source: metadata.source,
        })
    }

    /// テーマディレクトリ全体のバイト合計を再帰的に計算する。
    /// 読めないエントリ (権限エラー等) は静かにスキップする — UI 表示用なので厳密性より頑健性。
    pub(super) fn dir_size_bytes(dir: &std::path::Path) -> u64 {
        let mut total: u64 = 0;
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return 0,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(meta) = entry.metadata() {
                    total = total.saturating_add(meta.len());
                }
            } else if path.is_dir() {
                total = total.saturating_add(Self::dir_size_bytes(&path));
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_exists_returns_false_for_random_uuid() {
        // ~/.custom_cursors/<random-uuid>/theme.json はまず存在しないので false
        let id = uuid::Uuid::new_v4();
        assert!(!ThemeManager::theme_exists(id));
    }

    #[test]
    fn set_metadata_source_writes_marketplace() {
        let temp = tempfile::TempDir::new().unwrap();
        let theme_dir = temp.path();
        let theme_json = theme_dir.join("theme.json");
        std::fs::write(
            &theme_json,
            r#"{
                "schema_version":1,
                "id":"00000000-0000-0000-0000-000000000000",
                "name":"T",
                "version":"1.0.0",
                "created_at":"2026-05-14T00:00:00Z",
                "requires_os_shadow":false,
                "cursors":{}
            }"#,
        )
        .unwrap();
        set_metadata_source(theme_dir, types::ThemeSource::Marketplace).unwrap();
        let back: ThemeMetadata =
            serde_json::from_str(&std::fs::read_to_string(&theme_json).unwrap()).unwrap();
        assert!(matches!(back.source, types::ThemeSource::Marketplace));
    }

    #[test]
    fn set_metadata_source_resets_to_local() {
        let temp = tempfile::TempDir::new().unwrap();
        let theme_dir = temp.path();
        std::fs::write(
            theme_dir.join("theme.json"),
            r#"{
                "schema_version":1,
                "id":"00000000-0000-0000-0000-000000000000",
                "name":"T",
                "version":"1.0.0",
                "created_at":"2026-05-14T00:00:00Z",
                "requires_os_shadow":false,
                "cursors":{},
                "source":"marketplace"
            }"#,
        )
        .unwrap();
        set_metadata_source(theme_dir, types::ThemeSource::Local).unwrap();
        let back: ThemeMetadata =
            serde_json::from_str(&std::fs::read_to_string(theme_dir.join("theme.json")).unwrap())
                .unwrap();
        assert!(matches!(back.source, types::ThemeSource::Local));
    }
}
