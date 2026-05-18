//! `.cursorpack` (ZIP) パッケージ入出力 / インポート / エクスポート / 削除 / 複製。
//!
//! セキュリティ防御 (zip 爆弾対策・path traversal 防御 など) は本ファイル内の
//! [`ThemeManager::import_cursorpack_bytes`] に集約。
//!
//! `theme/mod.rs` から分割 (2026-05-18, refactor/yellow-items)。

use super::listing::set_metadata_source;
use super::sanitize::sanitize_archive_path;
use super::types::{
    self, clone_with_suffix, copy_dir_recursive, zip_dir_recursive, CursorDefinition,
    CursorpackInspection, ExistingTheme, LocalizedString, ThemeMetadata,
};
use super::ThemeManager;
use crate::errors::AppResult;
use std::collections::HashMap;
use uuid::Uuid;

impl ThemeManager {
    /// Windows のレジストリスキームを `.cursorpack` として書き出す。
    ///
    /// クリエイターを介さず、ユーザーが既に Windows のマウスのプロパティで
    /// 保存している配色をそのままパッケージ化したい用途。`.cur` / `.ani` /
    /// `.ico` といった元の拡張子を保持したまま zip に格納し、theme.json には
    /// 各ロールの相対パス (`cursors/<role>.<ext>`) を記録する。
    ///
    /// `name` が空のロール (= 既定継承スロット) は theme.cursors に含めない。
    /// 戻り値: 書き込んだバイト数。
    pub fn export_scheme_as_cursorpack(
        scheme_name: &str,
        cursor_paths: &HashMap<String, String>,
        output_path: &std::path::Path,
        author: Option<&str>,
    ) -> AppResult<(Uuid, u64)> {
        use crate::errors::AppError;
        use std::io::Write;

        let entries: Vec<(String, std::path::PathBuf)> = cursor_paths
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.clone(), std::path::PathBuf::from(v)))
            .collect();
        if entries.is_empty() {
            return Err(AppError::Theme(
                "スキームに有効なカーソルファイルがありません".to_string(),
            ));
        }

        // theme.json のメタデータ構築
        let mut name_map: HashMap<String, String> = HashMap::new();
        name_map.insert("ja".into(), scheme_name.to_string());
        name_map.insert("en".into(), scheme_name.to_string());

        let mut cursors_meta: HashMap<String, CursorDefinition> = HashMap::new();
        let mut zip_files: Vec<(String, Vec<u8>)> = Vec::new();
        for (role, path) in &entries {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("cur")
                .to_ascii_lowercase();
            // パッケージ内パスは lower_case 拡張子で統一
            let rel = format!("cursors/{}.{}", role, ext);
            let bytes = std::fs::read(path).map_err(|e| {
                AppError::Theme(format!(
                    "{} ({}) の読込に失敗: {}",
                    role,
                    crate::logging::redact_path(path),
                    e
                ))
            })?;
            cursors_meta.insert(
                role.clone(),
                CursorDefinition {
                    file: rel.clone(),
                    hotspot: types::Hotspot::ZERO,
                    resize_method: "lanczos".to_string(),
                    size_overrides: None,
                },
            );
            zip_files.push((rel, bytes));
        }

        let metadata = ThemeMetadata {
            schema_version: 1,
            id: Uuid::new_v4(),
            name: LocalizedString::Localized(name_map),
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            requires_os_shadow: true,
            cursors: cursors_meta,
            author: author
                .map(|s| s.to_string())
                .or_else(|| Some("Windows".to_string())),
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
            tags: Vec::new(),
            source: crate::theme::types::ThemeSource::Local,
        };

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(output_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let metadata_json = serde_json::to_vec_pretty(&metadata)?;
        zip.start_file("theme.json", opts)?;
        zip.write_all(&metadata_json)?;

        for (rel, bytes) in zip_files {
            zip.start_file(rel, opts)?;
            zip.write_all(&bytes)?;
        }
        zip.finish()?;

        let size = std::fs::metadata(output_path)?.len();
        tracing::info!(
            "exported scheme as cursorpack: '{}' ({} roles) → {} ({} bytes)",
            scheme_name,
            metadata.cursors.len(),
            crate::logging::redact_path(output_path),
            size
        );
        Ok((metadata.id, size))
    }

    /// `.cursorpack` (ZIP) のバイト列を `~/.custom_cursors/<UUID>/` に展開する。
    ///
    /// セキュリティ防御:
    ///  - 圧縮サイズの上限チェック (50 MB)
    ///  - 展開後合計サイズの上限チェック (200 MB) — 逐次計測で Zip 爆弾を遮断
    ///  - 個別ファイルサイズの上限チェック (10 MB)
    ///  - Path traversal 防御 (`..` / 絶対パスを拒否)
    ///  - シンボリックリンクと特殊エントリの拒否
    ///  - `theme.json` の Magic Byte (`{` の存在) と JSON パース成功を必須化
    ///  - レジストリキー注入対策: `.cursorpack` 内のファイル名はそのまま展開し、
    ///    レジストリ書き込み時に `apply_theme` が役割名を別途サニタイズ
    ///
    /// 戻り値: 展開後のテーマ ID (theme.json の `id` フィールド)。
    /// 既存の同 ID テーマがあれば上書きする。
    pub fn import_cursorpack_bytes(bytes: &[u8]) -> AppResult<Uuid> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        use std::io::{Cursor, Read};

        // 圧縮サイズの SoT は config.rs の DEFAULT_MAX_PACK_COMPRESSED_SIZE。
        // 残り 2 つ (uncompressed total / per-file size) は本 commit のスコープ外。
        use crate::config::DEFAULT_MAX_PACK_COMPRESSED_SIZE;
        const MAX_UNCOMPRESSED_TOTAL: u64 = 200 * 1024 * 1024;
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

        if bytes.len() as u64 > DEFAULT_MAX_PACK_COMPRESSED_SIZE {
            return Err(AppError::Theme(format!(
                ".cursorpack 圧縮サイズ {} bytes が上限 {} を超えています",
                bytes.len(),
                DEFAULT_MAX_PACK_COMPRESSED_SIZE
            )));
        }

        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| AppError::Theme(format!(".cursorpack の解析に失敗: {}", e)))?;

        // 1) theme.json を先読みして ID を確定
        let metadata: ThemeMetadata = {
            let mut entry = archive
                .by_name("theme.json")
                .map_err(|_| AppError::Theme("theme.json が見つかりません".to_string()))?;
            // theme.json の Magic Byte: 先頭 1 バイトが `{`
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            if !buf.trim_start().starts_with('{') {
                return Err(AppError::Theme(
                    "theme.json が JSON ではありません".to_string(),
                ));
            }
            serde_json::from_str(&buf)?
        };

        let theme_id = metadata.id;
        let cursors_dir = ConfigManager::cursors_dir()?;
        let target_dir = cursors_dir.join(theme_id.to_string());

        // 既存テーマがあれば一旦削除 (上書きインポート)
        if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir)?;
        }
        std::fs::create_dir_all(&target_dir)?;

        // 2) 全エントリを安全に展開
        let mut total_uncompressed: u64 = 0;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let raw_name = entry.name().to_string();

            // シンボリックリンク・特殊ビットの拒否
            // unix_mode が `S_IFLNK = 0xA000` を含むなら symlink
            if let Some(mode) = entry.unix_mode() {
                const S_IFMT: u32 = 0xF000;
                const S_IFLNK: u32 = 0xA000;
                if mode & S_IFMT == S_IFLNK {
                    return Err(AppError::Theme(format!(
                        "シンボリックリンクを含む .cursorpack は受け入れません: {}",
                        raw_name
                    )));
                }
            }

            // Path traversal 防御
            let safe_path = sanitize_archive_path(&raw_name)?;
            let dest = target_dir.join(&safe_path);

            // 念のため target_dir 配下に収まるか再チェック (canonicalize は存在前提なので
            // ここでは構造的にチェック)
            if !dest.starts_with(&target_dir) {
                return Err(AppError::Theme(format!(
                    "Path traversal を検出: {}",
                    raw_name
                )));
            }

            if entry.is_dir() {
                std::fs::create_dir_all(&dest)?;
                continue;
            }

            // 個別ファイルサイズ
            if entry.size() > MAX_FILE_SIZE {
                return Err(AppError::Theme(format!(
                    "ファイル {} のサイズ {} bytes が上限 {} を超えています",
                    raw_name,
                    entry.size(),
                    MAX_FILE_SIZE
                )));
            }

            // 累積サイズ (Zip 爆弾の最終防衛線)
            total_uncompressed = total_uncompressed.saturating_add(entry.size());
            if total_uncompressed > MAX_UNCOMPRESSED_TOTAL {
                let _ = std::fs::remove_dir_all(&target_dir);
                return Err(AppError::Theme(format!(
                    "展開後合計サイズが上限 {} bytes を超えました",
                    MAX_UNCOMPRESSED_TOTAL
                )));
            }

            // 親ディレクトリ確保 + 書き込み
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
        }

        tracing::info!(
            "テーマ {} ({}) を展開: {} bytes / {} files",
            metadata.name.get("ja"),
            theme_id,
            total_uncompressed,
            archive.len()
        );

        Ok(theme_id)
    }

    /// `.cursorpack` をローカルファイルパスから読み込んでインポートする。
    pub fn import_cursorpack_file(path: &std::path::Path) -> AppResult<Uuid> {
        let bytes = std::fs::read(path)?;
        Self::import_cursorpack_bytes(&bytes)
    }

    /// 内部 helper: in-memory `Vec<u8>` に `.cursorpack` zip バイト列を書き出す。
    ///
    /// `export_cursorpack` (ファイル出力) と `export_cursorpack_streamed` の
    /// `destination::File` / `destination::Library` 経路から呼ばれる。
    ///
    /// **拡張子方針**: `metadata.cursors[role].file` をそのまま zip エントリ名と
    /// theme.json の `file` フィールドに使う。呼び出し側が `.cur` / `.ani` 等を
    /// あらかじめ設定しておくこと。ここで強制的に `.cur` 化することはしない
    /// (旧実装の落とし穴: `.ani` を吸い取る経路が出来ても出力で必ず .cur 化されて
    /// しまい、Library 保存後にロード時の拡張子矛盾でカーソルが壊れていた)。
    ///
    /// `file` が空 / 異常な場合だけ安全な既定 `cursors/<role>.cur` にフォールバックする。
    pub fn write_cursorpack_to_buffer(
        metadata: &mut ThemeMetadata,
        cursors: &HashMap<String, Vec<u8>>,
    ) -> AppResult<Vec<u8>> {
        use crate::errors::AppError;
        use std::io::{Cursor, Write};

        // 各ロールの zip エントリ名を確定する。`def.file` を尊重しつつ、空・不正のときだけ
        // `cursors/<role>.cur` に正規化する。
        for (role, def) in metadata.cursors.iter_mut() {
            let raw = def.file.trim();
            let needs_fallback = raw.is_empty()
                || raw.contains("..")
                || raw.starts_with('/')
                || raw.starts_with('\\')
                || raw.contains(':');
            if needs_fallback {
                def.file = format!("cursors/{}.cur", role);
            }
        }

        // 全カーソル分のバイトが揃っているか検証
        for role in metadata.cursors.keys() {
            if !cursors.contains_key(role) {
                return Err(AppError::Theme(format!(
                    "役割 {} のカーソルバイト列が指定されていません",
                    role
                )));
            }
        }

        let metadata_json = serde_json::to_vec_pretty(metadata)?;

        let mut buf: Vec<u8> = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(cursor);
            let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            zip.start_file("theme.json", opts)?;
            zip.write_all(&metadata_json)?;

            // zip エントリ名は theme.json の `file` フィールドと完全一致させる
            // (= import_cursorpack_bytes が `archive.by_name(&def.file)` で引けるように)
            for (role, bin) in cursors {
                let entry_name = metadata
                    .cursors
                    .get(role)
                    .map(|d| d.file.clone())
                    .unwrap_or_else(|| format!("cursors/{}.cur", role));
                zip.start_file(entry_name, opts)?;
                zip.write_all(bin)?;
            }

            // previews/<role>.png を同梱 (Marketplace 詳細モーダルの 3×2 表示用)
            // cursors マップに無いロールはスキップ。失敗してもパッケージ全体は壊さない。
            let previews = Self::build_preview_pngs(cursors, &metadata.cursors);
            for (role, png) in previews {
                let name = format!("previews/{}.png", role);
                if zip.start_file(&name, opts).is_ok() {
                    let _ = zip.write_all(&png);
                }
            }

            zip.finish()?;
        }

        Ok(buf)
    }

    /// `.cursorpack` をファイルに出力する。Public な薄いラッパで、内部は
    /// `write_cursorpack_to_buffer` + `fs::write`。
    ///
    /// theme.json の `cursors[role].file` は自動的に `cursors/<role>.cur` にリライトされる。
    /// 戻り値: 書き込んだバイト数。
    pub fn export_cursorpack(
        metadata: &mut ThemeMetadata,
        cursors: &HashMap<String, Vec<u8>>,
        output_path: &std::path::Path,
    ) -> AppResult<u64> {
        let bytes = Self::write_cursorpack_to_buffer(metadata, cursors)?;

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(output_path, &bytes)?;

        let size = bytes.len() as u64;
        tracing::info!(
            "exported cursorpack: theme={} ({}) → {} ({} bytes)",
            metadata.name.get("ja"),
            metadata.id,
            crate::logging::redact_path(output_path),
            size
        );
        Ok(size)
    }

    /// `.cursorpack` のバイト列から theme.json だけを読み出す軽量検査。
    /// 既存ライブラリ内に同 ID のテーマがあるか、バージョン比較する用途に使用。
    pub fn inspect_cursorpack_bytes(bytes: &[u8]) -> AppResult<CursorpackInspection> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        use std::io::{Cursor, Read};

        use crate::config::DEFAULT_MAX_PACK_COMPRESSED_SIZE;
        if bytes.len() as u64 > DEFAULT_MAX_PACK_COMPRESSED_SIZE {
            return Err(AppError::Theme(format!(
                ".cursorpack 圧縮サイズ {} が上限 {} を超えています",
                bytes.len(),
                DEFAULT_MAX_PACK_COMPRESSED_SIZE
            )));
        }

        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| AppError::Theme(format!(".cursorpack の解析に失敗: {}", e)))?;

        let metadata: ThemeMetadata = {
            let mut entry = archive
                .by_name("theme.json")
                .map_err(|_| AppError::Theme("theme.json が見つかりません".to_string()))?;
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            if !buf.trim_start().starts_with('{') {
                return Err(AppError::Theme(
                    "theme.json が JSON ではありません".to_string(),
                ));
            }
            serde_json::from_str(&buf)?
        };

        // 既存ライブラリと衝突チェック
        let cursors_dir = ConfigManager::cursors_dir()?;
        let existing_dir = cursors_dir.join(metadata.id.to_string());
        let existing = if existing_dir.exists() {
            let theme_json = existing_dir.join("theme.json");
            if theme_json.exists() {
                let content = std::fs::read_to_string(&theme_json)?;
                let existing_meta: ThemeMetadata = serde_json::from_str(&content)?;
                Some(ExistingTheme {
                    name: existing_meta.name.get("ja"),
                    version: existing_meta.version,
                    author: existing_meta.author,
                    role_count: existing_meta.cursors.len() as u32,
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok(CursorpackInspection {
            id: metadata.id,
            name: metadata.name.get("ja"),
            version: metadata.version,
            author: metadata.author,
            role_count: metadata.cursors.len() as u32,
            existing,
        })
    }

    pub fn inspect_cursorpack_file(path: &std::path::Path) -> AppResult<CursorpackInspection> {
        let bytes = std::fs::read(path)?;
        Self::inspect_cursorpack_bytes(&bytes)
    }

    /// 指定 ID のテーマディレクトリ (`~/.custom_cursors/<UUID>/`) を完全削除する。
    ///
    /// アクティブテーマを削除した場合、`HKCU\Control Panel\Cursors` (現在値) 側は
    /// パスがファイル不在になっても Windows が既定にフォールバックするので触らない。
    /// 一方、`HKCU\Control Panel\Cursors\Schemes` (= マウスのプロパティ → ポインター →
    /// "デザイン" ドロップダウンの保存済みスキーム一覧) には `apply_theme` 時に
    /// `RegistryManager::register_scheme` で書き込んだエントリが残るため、ここで
    /// 明示的にクリーンアップする (best-effort: 失敗しても削除自体は成功扱い)。
    ///
    /// 呼び出し側は config.active_theme_id を None に戻す責任を持つ。
    pub fn delete_theme(id: Uuid) -> AppResult<()> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        use crate::registry::RegistryManager;
        let cursors_dir = ConfigManager::cursors_dir()?;
        let theme_dir = cursors_dir.join(id.to_string());
        if !theme_dir.exists() {
            return Err(AppError::Theme(format!("テーマ {} が見つかりません", id)));
        }
        std::fs::remove_dir_all(&theme_dir)?;

        // Windows のマウスのプロパティの "デザイン" に削除済みテーマが残らないよう、
        // このテーマディレクトリを指す Schemes 値を掃除する。ベストエフォート扱い:
        // 失敗してもファイル削除自体は成功しているのでエラーは伝播させない。
        match RegistryManager::unregister_schemes_for_theme(&theme_dir) {
            Ok(n) if n > 0 => {
                tracing::info!(
                    "Schemes から {} 件のテーマ参照を削除しました (theme={})",
                    n,
                    id
                )
            }
            Ok(_) => {}
            Err(e) => tracing::warn!(
                "Schemes クリーンアップに失敗 (ファイル削除自体は成功): {}",
                e
            ),
        }

        tracing::info!("テーマ {} を削除しました", id);
        Ok(())
    }

    /// 指定 ID のテーマを複製し、新しい UUID を持つ別テーマとして保存する。
    ///
    /// `theme.json` は同内容のコピーで `id` だけ新規 UUID に、`name` には ` (Copy)` を
    /// 末尾付与して識別しやすくする。戻り値は新テーマの UUID。
    pub fn duplicate_theme(source_id: Uuid) -> AppResult<Uuid> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        let cursors_dir = ConfigManager::cursors_dir()?;
        let source_dir = cursors_dir.join(source_id.to_string());
        if !source_dir.exists() {
            return Err(AppError::Theme(format!(
                "複製元テーマ {} が見つかりません",
                source_id
            )));
        }

        let source_theme_json = source_dir.join("theme.json");
        let mut metadata: ThemeMetadata =
            serde_json::from_str(&std::fs::read_to_string(&source_theme_json)?)?;

        let new_id = Uuid::new_v4();
        metadata.id = new_id;
        let original_name = metadata.name.clone();
        metadata.name = clone_with_suffix(&original_name, " (Copy)");

        let target_dir = cursors_dir.join(new_id.to_string());
        copy_dir_recursive(&source_dir, &target_dir)?;
        std::fs::write(
            target_dir.join("theme.json"),
            serde_json::to_vec_pretty(&metadata)?,
        )?;

        // 複製先は常に Local にリセット (Marketplace 由来テーマを複製したコピーは編集可能にする)
        set_metadata_source(&target_dir, types::ThemeSource::Local)?;

        tracing::info!("テーマ {} を {} として複製しました", source_id, new_id);
        Ok(new_id)
    }

    /// 既存テーマディレクトリを `.cursorpack` (ZIP) に再パッケージしてエクスポートする。
    ///
    /// クリエイターを経由せずライブラリから直接書き出したい場合に使う。
    /// theme.json + 全カーソルファイルをそのままアーカイブする。
    pub fn repackage_theme(id: Uuid, output_path: &std::path::Path) -> AppResult<u64> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        let cursors_dir = ConfigManager::cursors_dir()?;
        let theme_dir = cursors_dir.join(id.to_string());
        if !theme_dir.exists() {
            return Err(AppError::Theme(format!("テーマ {} が見つかりません", id)));
        }
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = std::fs::File::create(output_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip_dir_recursive(&theme_dir, &theme_dir, &mut zip, opts)?;
        zip.finish()?;

        let size = std::fs::metadata(output_path)?.len();
        tracing::info!(
            "repackaged theme: {} -> {} ({} bytes)",
            id,
            crate::logging::redact_path(output_path),
            size
        );
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_cursorpack_to_buffer_produces_valid_zip() {
        let mut metadata = ThemeMetadata {
            schema_version: 1,
            id: uuid::Uuid::new_v4(),
            name: LocalizedString::Simple("Buf Test".to_string()),
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            requires_os_shadow: false,
            cursors: HashMap::new(),
            author: None,
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
            tags: Vec::new(),
            source: types::ThemeSource::Local,
        };
        let cursors: HashMap<String, Vec<u8>> = HashMap::new();

        let bytes = ThemeManager::write_cursorpack_to_buffer(&mut metadata, &cursors).unwrap();

        // 先頭は zip マジック "PK\x03\x04"
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        // theme.json を含む = inspect_cursorpack_bytes でメタデータが取り出せる
        let inspected = ThemeManager::inspect_cursorpack_bytes(&bytes).unwrap();
        assert_eq!(inspected.id, metadata.id);

        // previews/ は cursors が空なので含まれないこと
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        let preview_entries: Vec<_> = names
            .iter()
            .filter(|n| n.starts_with("previews/"))
            .collect();
        assert!(
            preview_entries.is_empty(),
            "cursors 空なら previews も空: {names:?}"
        );
    }

    /// 回帰テスト: `.ani` 拡張子のロールが zip 出力で `.cur` に書き換えられないこと。
    /// 旧実装ではすべてのロールが `cursors/<role>.cur` に強制リネームされ、
    /// `.ani` バイトを `.cur` ファイル名で埋め込んで Library に保存していた。
    #[test]
    fn write_cursorpack_to_buffer_preserves_ani_extension() {
        use std::io::Read;

        let mut cursors_meta = HashMap::new();
        cursors_meta.insert(
            "Arrow".to_string(),
            CursorDefinition {
                file: "cursors/Arrow.ani".to_string(),
                hotspot: types::Hotspot::ZERO,
                resize_method: "lanczos".to_string(),
                size_overrides: None,
            },
        );
        cursors_meta.insert(
            "IBeam".to_string(),
            CursorDefinition {
                file: "cursors/IBeam.cur".to_string(),
                hotspot: types::Hotspot::ZERO,
                resize_method: "lanczos".to_string(),
                size_overrides: None,
            },
        );
        let mut metadata = ThemeMetadata {
            schema_version: 1,
            id: uuid::Uuid::new_v4(),
            name: LocalizedString::Simple("Ani Mix".to_string()),
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            requires_os_shadow: false,
            cursors: cursors_meta,
            author: None,
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
            tags: Vec::new(),
            source: types::ThemeSource::Local,
        };
        // ani は "RIFF" マジックで始まるダミー、cur は "CUR1" ダミー
        let mut cursors: HashMap<String, Vec<u8>> = HashMap::new();
        cursors.insert("Arrow".to_string(), b"RIFF__ANI_PAYLOAD__".to_vec());
        cursors.insert("IBeam".to_string(), b"CUR1__IBEAM_PAYLOAD__".to_vec());

        let zip_bytes = ThemeManager::write_cursorpack_to_buffer(&mut metadata, &cursors).unwrap();

        // theme.json の `file` が保持されていること
        assert_eq!(metadata.cursors["Arrow"].file, "cursors/Arrow.ani");
        assert_eq!(metadata.cursors["IBeam"].file, "cursors/IBeam.cur");

        // zip エントリ名と中身が `def.file` に対応していること
        let reader = std::io::Cursor::new(&zip_bytes);
        let mut archive = zip::ZipArchive::new(reader).unwrap();
        {
            let mut e = archive
                .by_name("cursors/Arrow.ani")
                .expect("Arrow.ani entry");
            let mut buf = Vec::new();
            e.read_to_end(&mut buf).unwrap();
            assert_eq!(buf, b"RIFF__ANI_PAYLOAD__");
        }
        {
            let mut e = archive
                .by_name("cursors/IBeam.cur")
                .expect("IBeam.cur entry");
            let mut buf = Vec::new();
            e.read_to_end(&mut buf).unwrap();
            assert_eq!(buf, b"CUR1__IBEAM_PAYLOAD__");
        }
        // 旧バグでは Arrow.cur に書かれていた → 存在しないことを確認
        assert!(
            archive.by_name("cursors/Arrow.cur").is_err(),
            "Arrow.ani が誤って Arrow.cur に書かれていない"
        );
    }

    #[test]
    fn write_cursorpack_includes_preview_for_arrow() {
        use crate::theme::types::{Hotspot, Ratio01};

        // 32×32 の赤 PNG を build_cur_from_png に流して .cur を作る
        let img: image::RgbaImage =
            image::ImageBuffer::from_pixel(32, 32, image::Rgba([200, 50, 50, 255]));
        let mut png = Vec::new();
        image::ImageEncoder::write_image(
            image::codecs::png::PngEncoder::new(&mut png),
            img.as_raw(),
            32,
            32,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
        let cur_bytes = crate::cursor::build_cur_from_png(
            &png,
            0,
            0,
            crate::cursor::ResizeMethod::Lanczos,
            None,
            None,
        )
        .unwrap();

        let mut cursors_meta = std::collections::HashMap::new();
        cursors_meta.insert(
            "Arrow".into(),
            CursorDefinition {
                file: "cursors/Arrow.cur".into(),
                hotspot: Hotspot {
                    x: Ratio01::new(0.0),
                    y: Ratio01::new(0.0),
                },
                resize_method: "lanczos".into(),
                size_overrides: None,
            },
        );
        let mut metadata = ThemeMetadata {
            schema_version: 1,
            id: uuid::Uuid::new_v4(),
            name: LocalizedString::Simple("Preview Test".into()),
            version: "1.0.0".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            requires_os_shadow: false,
            cursors: cursors_meta,
            author: None,
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
            tags: vec![],
            source: types::ThemeSource::Local,
        };
        let mut cursors = std::collections::HashMap::new();
        cursors.insert("Arrow".to_string(), cur_bytes);

        let bytes = ThemeManager::write_cursorpack_to_buffer(&mut metadata, &cursors).unwrap();

        // ZIP 内に previews/Arrow.png があり、64×64 PNG であることを確認
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
        let mut preview_bytes = Vec::new();
        {
            let mut f = archive
                .by_name("previews/Arrow.png")
                .expect("previews/Arrow.png が必要");
            std::io::Read::read_to_end(&mut f, &mut preview_bytes).unwrap();
        }
        let img =
            image::load_from_memory_with_format(&preview_bytes, image::ImageFormat::Png).unwrap();
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 64);
    }
}
