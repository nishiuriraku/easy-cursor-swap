//! `.cursorprofile` ロケーション-バックアップ
//!
//! `~/.custom_cursors/` 全テーマと `config.json` をまとめた Zip アーカイブ。
//! PC 移行 / OS 再インストール時の復元を想定。
//! `.cursorpack` (テーマ単体) との使い分け:
//!  - `.cursorpack` = 配布用 (1 テーマ)
//!  - `.cursorprofile` = 個人バックアップ (設定 + 全テーマ)
//!
//! 構造:
//! ```
//! profile.json            ← AppConfig 全体 + schema_version
//! cursors/<UUID>/...      ← ~/.custom_cursors/<UUID>/ をそのまま
//! ```

use crate::config::{AppConfig, ConfigManager};
use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// `profile.json` のスキーマ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEnvelope {
    /// バックアップフォーマットバージョン (将来の互換性用)
    pub schema_version: u32,
    /// バックアップ作成時刻 (ISO8601)
    pub exported_at: String,
    /// アプリのバージョン
    pub app_version: String,
    /// アプリ設定スナップショット
    pub config: AppConfig,
}

const PROFILE_SCHEMA_VERSION: u32 = 1;
const PROFILE_JSON_NAME: &str = "profile.json";
const CURSORS_PREFIX: &str = "cursors/";
/// 個別ファイル/累積サイズの上限はインポート時のみ適用 (エクスポートはユーザー責任)
const MAX_PROFILE_FILE_SIZE: u64 = 50 * 1024 * 1024;
const MAX_PROFILE_TOTAL_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2 GB

pub struct BackupManager;

impl BackupManager {
    /// 現在の設定 + 全テーマを Zip にまとめて指定パスに書き出す。
    pub fn export(out_path: &Path, config: &AppConfig) -> AppResult<()> {
        let cursors_dir = ConfigManager::cursors_dir()?;

        let envelope = ProfileEnvelope {
            schema_version: PROFILE_SCHEMA_VERSION,
            exported_at: chrono::Utc::now().to_rfc3339(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            config: config.clone(),
        };
        let envelope_json = serde_json::to_vec_pretty(&envelope)?;

        // 親ディレクトリが存在しなければ作成
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(out_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions =
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

        // 1) profile.json
        zip.start_file(PROFILE_JSON_NAME, opts)?;
        zip.write_all(&envelope_json)?;

        // 2) cursors/<UUID>/... をそのまま追加
        if cursors_dir.exists() {
            let mut count = 0u32;
            for entry in std::fs::read_dir(&cursors_dir)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('_') {
                        // _initial_snapshot.json / _pending_apply.snapshot は対象外
                        continue;
                    }
                }
                add_dir_to_zip(&mut zip, &path, &PathBuf::from(CURSORS_PREFIX), &opts)?;
                count += 1;
            }
            tracing::info!(
                "profile export: {} themes packed → {}",
                count,
                crate::logging::redact_path(out_path)
            );
        }

        zip.finish()?;
        Ok(())
    }

    /// 指定 Zip を読み込んで `~/.custom_cursors/` と config を復元する。
    /// `merge` が true なら既存テーマを保持し、新規テーマと設定のみ反映。
    /// false なら既存テーマも上書き。
    pub fn import(path: &Path, merge: bool) -> AppResult<ProfileEnvelope> {
        let bytes = std::fs::read(path)?;
        let reader = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| AppError::Theme(format!(".cursorprofile の解析に失敗: {}", e)))?;

        // 1) profile.json を読み込み
        let envelope: ProfileEnvelope = {
            use std::io::Read;
            let mut entry = archive
                .by_name(PROFILE_JSON_NAME)
                .map_err(|_| AppError::Theme("profile.json が見つかりません".to_string()))?;
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            if !buf.trim_start().starts_with('{') {
                return Err(AppError::Theme(
                    "profile.json が JSON ではありません".to_string(),
                ));
            }
            serde_json::from_str(&buf)?
        };

        if envelope.schema_version > PROFILE_SCHEMA_VERSION {
            return Err(AppError::Theme(format!(
                ".cursorprofile スキーマ ({}) がアプリの対応範囲外です",
                envelope.schema_version
            )));
        }

        let cursors_dir = ConfigManager::cursors_dir()?;
        std::fs::create_dir_all(&cursors_dir)?;

        // 上書きモードの場合、既存テーマを削除 (_ で始まるディレクトリは保護)
        if !merge && cursors_dir.exists() {
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
                let _ = std::fs::remove_dir_all(&path);
            }
        }

        // 2) cursors/ プレフィックスのエントリを安全に展開
        let mut total: u64 = 0;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let raw_name = entry.name().to_string();

            // profile.json はスキップ
            if raw_name == PROFILE_JSON_NAME {
                continue;
            }
            // 想定外のトップレベルエントリは無視
            let stripped = match raw_name.strip_prefix(CURSORS_PREFIX) {
                Some(s) => s,
                None => continue,
            };
            if stripped.is_empty() {
                continue;
            }

            // シンボリックリンク防御
            if let Some(mode) = entry.unix_mode() {
                const S_IFMT: u32 = 0xF000;
                const S_IFLNK: u32 = 0xA000;
                if mode & S_IFMT == S_IFLNK {
                    return Err(AppError::Theme(format!(
                        "シンボリックリンクを含む .cursorprofile は受け入れません: {}",
                        raw_name
                    )));
                }
            }

            let safe_path = crate::theme::sanitize_archive_path_pub(stripped)?;
            let dest = cursors_dir.join(&safe_path);
            if !dest.starts_with(&cursors_dir) {
                return Err(AppError::Theme(format!(
                    "Path traversal 検出: {}",
                    raw_name
                )));
            }

            if entry.is_dir() {
                std::fs::create_dir_all(&dest)?;
                continue;
            }
            if entry.size() > MAX_PROFILE_FILE_SIZE {
                return Err(AppError::Theme(format!(
                    "ファイル {} のサイズ {} bytes が上限 {} を超えています",
                    raw_name,
                    entry.size(),
                    MAX_PROFILE_FILE_SIZE
                )));
            }
            total = total.saturating_add(entry.size());
            if total > MAX_PROFILE_TOTAL_SIZE {
                return Err(AppError::Theme(format!(
                    ".cursorprofile 展開後合計が上限 {} を超えました",
                    MAX_PROFILE_TOTAL_SIZE
                )));
            }

            // 既存上書き判定: merge モードでは既存ファイルをスキップ (テーマ単位で保持)
            if merge && dest.exists() && !entry.is_dir() {
                continue;
            }

            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
        }

        tracing::info!(
            "profile import done: schema_v{} app_v{} merge={}",
            envelope.schema_version,
            envelope.app_version,
            merge
        );
        Ok(envelope)
    }
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    src: &Path,
    archive_prefix: &Path,
    opts: &zip::write::SimpleFileOptions,
) -> AppResult<()> {
    let dir_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::Theme(format!("ディレクトリ名が取得できません: {}", src.display())))?;
    let archive_dir = archive_prefix.join(dir_name);

    for entry in walkdir(src)? {
        let path = entry?;
        let rel = path.strip_prefix(src).map_err(|e| {
            AppError::Theme(format!("相対パス計算失敗: {}", e))
        })?;
        let archive_path = archive_dir.join(rel);
        let archive_str = archive_path
            .to_str()
            .ok_or_else(|| AppError::Theme("非 UTF-8 パス".to_string()))?
            .replace('\\', "/");

        if path.is_dir() {
            // 空ディレクトリも保持
            zip.add_directory(format!("{}/", archive_str), *opts)?;
        } else {
            zip.start_file(archive_str, *opts)?;
            let mut f = std::fs::File::open(&path)?;
            std::io::copy(&mut f, zip)?;
        }
    }
    Ok(())
}

/// 標準ライブラリのみで小規模再帰トラバース (walkdir クレート未追加)。
fn walkdir(root: &Path) -> AppResult<Vec<AppResult<PathBuf>>> {
    let mut out: Vec<AppResult<PathBuf>> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        if !p.is_dir() {
            out.push(Ok(p));
            continue;
        }
        out.push(Ok(p.clone()));
        let entries = match std::fs::read_dir(&p) {
            Ok(e) => e,
            Err(e) => {
                out.push(Err(AppError::Io(e)));
                continue;
            }
        };
        for ent in entries {
            match ent {
                Ok(e) => stack.push(e.path()),
                Err(e) => out.push(Err(AppError::Io(e))),
            }
        }
    }
    // ルート自体は archive_dir に展開済みなので除く
    Ok(out.into_iter().filter(|r| r.as_ref().map_or(true, |p| p != root)).collect())
}
