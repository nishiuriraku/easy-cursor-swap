//! EasyCursorSwap 設定管理モジュール
//!
//! アプリケーション設定の Source of Truth を Rust 側で管理する。
//! 設定は `config.json` に永続化し、UIが閉じていても常駐プロセスが参照できる。

use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use uuid::Uuid;

/// バックアップファイルの情報
#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    /// ファイル名 (例: "config.bak.v2.json", "config.corrupt.1746123456.json")
    pub file_name: String,
    /// UTC の ISO 8601 最終更新日時
    pub modified_utc: String,
    /// ファイルサイズ (バイト)
    pub size_bytes: u64,
    /// "versioned" | "corrupt"
    pub kind: String,
}

/// 設定スキーマの現在のバージョン
const CURRENT_SCHEMA_VERSION: u32 = 1;

/// アプリケーション設定（Source of Truth）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 設定スキーマバージョン（マイグレーション用）
    pub schema_version: u32,

    /// 一般設定
    pub general: GeneralConfig,

    /// ダークモード設定
    pub dark_mode: DarkModeConfig,

    /// セキュリティ設定
    pub security: SecurityConfig,

    /// ログ設定
    pub logging: LoggingConfig,
}

/// 一般設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// OS起動時に自動起動するか
    pub auto_start: bool,
    /// 自動アップデート有効/無効
    pub auto_update: bool,
    /// 表示言語 ("ja" / "en" / "auto")
    pub language: String,
    /// 現在適用中のテーマID
    pub active_theme_id: Option<Uuid>,
    /// グローバルホットキー（パニックボタン）
    pub panic_hotkey: String,
    /// クラッシュレポート送信オプトイン (デフォルト false)
    ///
    /// 有効にしても `easycursorswap/index` のサーバー側エンドポイントが
    /// 整備されるまでは収集のみ。`crash::list_reports` で UI に表示する。
    #[serde(default)]
    pub crash_reporting: bool,
}

/// ダークモード連動設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarkModeConfig {
    /// ダークモード連動を有効にするか
    pub enabled: bool,
    /// ライトモード時に使用するテーマID
    pub light_theme_id: Option<Uuid>,
    /// ダークモード時に使用するテーマID
    pub dark_theme_id: Option<Uuid>,
}

/// セキュリティ閾値設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// .cursorpack 圧縮時サイズ上限 (バイト)
    pub max_pack_compressed_size: u64,
    /// 解凍後合計サイズ上限 (バイト)
    pub max_pack_uncompressed_size: u64,
    /// 個別画像ファイルサイズ上限 (バイト)
    pub max_image_file_size: u64,
    /// ストレージ警告閾値 (バイト)
    pub storage_warning_threshold: u64,
}

/// ログ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// ログレベル ("TRACE" / "DEBUG" / "INFO" / "WARN" / "ERROR")
    pub level: String,
    /// ログ保持日数
    pub retention_days: u32,
    /// ログ総容量上限 (バイト)
    pub max_total_size: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            general: GeneralConfig {
                auto_start: true,
                auto_update: true,
                language: "auto".to_string(),
                active_theme_id: None,
                panic_hotkey: "Ctrl+Alt+Shift+R".to_string(),
                crash_reporting: false,
            },
            dark_mode: DarkModeConfig {
                enabled: false,
                light_theme_id: None,
                dark_theme_id: None,
            },
            security: SecurityConfig {
                // 50 MB
                max_pack_compressed_size: 50 * 1024 * 1024,
                // 200 MB
                max_pack_uncompressed_size: 200 * 1024 * 1024,
                // 10 MB
                max_image_file_size: 10 * 1024 * 1024,
                // 1 GB
                storage_warning_threshold: 1024 * 1024 * 1024,
            },
            logging: LoggingConfig {
                level: "INFO".to_string(),
                retention_days: 14,
                // 100 MB
                max_total_size: 100 * 1024 * 1024,
            },
        }
    }
}

/// アプリケーション設定の管理を行うマネージャー
pub struct ConfigManager {
    /// 設定データ（スレッドセーフな読み書きロック）
    config: RwLock<AppConfig>,
    /// 設定ファイルのパス
    config_path: PathBuf,
}

impl ConfigManager {
    /// カーソル保存ディレクトリのパスを返す
    /// ~/.custom_cursors/
    pub fn cursors_dir() -> AppResult<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            AppError::Config("ホームディレクトリが見つかりません".to_string())
        })?;
        Ok(home.join(".custom_cursors"))
    }

    /// 設定ファイルのパスを返す
    /// %LOCALAPPDATA%/EasyCursorSwap/config.json
    fn config_file_path() -> AppResult<PathBuf> {
        let local_data = dirs::data_local_dir().ok_or_else(|| {
            AppError::Config("LocalAppData ディレクトリが見つかりません".to_string())
        })?;
        Ok(local_data.join("EasyCursorSwap").join("config.json"))
    }

    /// 設定マネージャーを初期化する。
    ///
    /// 動作:
    ///  1. ファイルなし → デフォルト設定で新規作成
    ///  2. ファイルあり → パース成功 → schema_version 比較
    ///     - 同じ → そのまま使用
    ///     - 古い → 自動マイグレーション (現状はフィールド追加のみで透過的) + バックアップ作成
    ///     - 新しい → アプリ更新が必要 → エラー (`Config(...)` を返し、main 側で専用画面表示)
    ///  3. ファイルあり → パース失敗 → `config.corrupt.{ts}.json` に退避してデフォルトで再作成
    pub fn init() -> AppResult<Self> {
        let config_path = Self::config_file_path()?;

        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            match serde_json::from_str::<AppConfig>(&content) {
                Ok(parsed) => Self::handle_versioned(parsed, &config_path)?,
                Err(e) => {
                    // パース失敗 → 退避して新規作成
                    Self::backup_corrupt(&config_path, &content, &e.to_string())?;
                    let fresh = AppConfig::default();
                    fs::write(&config_path, serde_json::to_string_pretty(&fresh)?)?;
                    tracing::warn!("設定ファイルが破損していたためデフォルトで再作成しました");
                    fresh
                }
            }
        } else {
            let fresh = AppConfig::default();
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&config_path, serde_json::to_string_pretty(&fresh)?)?;
            fresh
        };

        let cursors_dir = Self::cursors_dir()?;
        if !cursors_dir.exists() {
            fs::create_dir_all(&cursors_dir)?;
        }

        Ok(Self {
            config: RwLock::new(config),
            config_path,
        })
    }

    /// schema_version を検査し、古ければマイグレーション + バックアップ、
    /// 新しければエラーを返す。
    fn handle_versioned(config: AppConfig, config_path: &PathBuf) -> AppResult<AppConfig> {
        if config.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(AppError::Config(format!(
                "設定ファイルのバージョン ({}) はこのアプリ ({}) より新しいです。\nアプリの更新が必要です。",
                config.schema_version, CURRENT_SCHEMA_VERSION
            )));
        }

        if config.schema_version < CURRENT_SCHEMA_VERSION {
            // 古いバージョン → バックアップ後にマイグレート
            let from_version = config.schema_version;
            Self::write_versioned_backup(config_path, from_version, &config)?;

            // マイグレーション本体: 現状は serde の default で穴埋めされるので、
            // schema_version を更新して書き戻すだけで OK。
            let mut migrated = config;
            migrated.schema_version = CURRENT_SCHEMA_VERSION;
            fs::write(config_path, serde_json::to_string_pretty(&migrated)?)?;
            tracing::info!(
                "設定をマイグレーション: v{} → v{}",
                from_version,
                CURRENT_SCHEMA_VERSION
            );
            return Ok(migrated);
        }

        Ok(config)
    }

    /// `config.bak.v{N}.json` 形式でバージョン番号付きバックアップを作成する。
    /// 同じバージョンのバックアップが既存なら上書きしない (最古を保護)。
    fn write_versioned_backup(
        config_path: &PathBuf,
        from_version: u32,
        config: &AppConfig,
    ) -> AppResult<()> {
        let bak = config_path.with_file_name(format!("config.bak.v{}.json", from_version));
        if bak.exists() {
            return Ok(());
        }
        fs::write(&bak, serde_json::to_string_pretty(config)?)?;
        tracing::info!(
            "バックアップを作成: {}",
            crate::logging::redact_path(&bak)
        );
        Ok(())
    }

    /// パース不可な設定ファイルを `config.corrupt.{epoch}.json` に退避する。
    fn backup_corrupt(config_path: &PathBuf, raw: &str, reason: &str) -> AppResult<()> {
        let ts = chrono::Utc::now().timestamp();
        let bak = config_path.with_file_name(format!("config.corrupt.{}.json", ts));
        fs::write(&bak, raw)?;
        tracing::error!(
            "設定ファイルが破損 ({}) → 退避: {}",
            reason,
            crate::logging::redact_path(&bak)
        );
        Ok(())
    }

    /// 現在の設定を取得する
    pub fn get(&self) -> AppResult<AppConfig> {
        let config = self.config.read().map_err(|e| {
            AppError::Config(format!("設定のロックに失敗: {}", e))
        })?;
        Ok(config.clone())
    }

    /// 設定を更新し、ディスクに永続化する
    pub fn update<F>(&self, updater: F) -> AppResult<AppConfig>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.write().map_err(|e| {
            AppError::Config(format!("設定のロックに失敗: {}", e))
        })?;

        updater(&mut config);

        // ディスクに保存
        let content = serde_json::to_string_pretty(&*config)?;
        fs::write(&self.config_path, content)?;

        Ok(config.clone())
    }

    /// 設定ディレクトリ内のバックアップファイル一覧を返す。
    ///
    /// 対象: `config.bak.v*.json` (versioned) / `config.corrupt.*.json` (corrupt)
    /// 返却: 最終更新日時の降順（最新が先頭）
    pub fn list_backups(&self) -> AppResult<Vec<BackupInfo>> {
        let dir = self
            .config_path
            .parent()
            .ok_or_else(|| AppError::Config("設定ディレクトリの取得に失敗".to_string()))?;

        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut backups: Vec<BackupInfo> = fs::read_dir(dir)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                let kind = if name.starts_with("config.bak.v") && name.ends_with(".json") {
                    "versioned"
                } else if name.starts_with("config.corrupt.") && name.ends_with(".json") {
                    "corrupt"
                } else {
                    return None;
                };
                let meta = entry.metadata().ok()?;
                let modified = meta.modified().ok()?;
                let secs = modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0)
                    .unwrap_or_default();
                Some(BackupInfo {
                    file_name: name,
                    modified_utc: dt.to_rfc3339(),
                    size_bytes: meta.len(),
                    kind: kind.to_string(),
                })
            })
            .collect();

        // 最新が先頭
        backups.sort_by(|a, b| b.modified_utc.cmp(&a.modified_utc));
        Ok(backups)
    }

    /// 指定したバックアップファイルを `config.json` に上書きして設定を再ロードする。
    ///
    /// セキュリティ: `file_name` は `config.bak.v*.json` / `config.corrupt.*.json` のみ許可。
    pub fn restore_backup(&self, file_name: &str) -> AppResult<()> {
        // ファイル名の簡易バリデーション (パストラバーサル防止)
        let valid = (file_name.starts_with("config.bak.v") || file_name.starts_with("config.corrupt."))
            && file_name.ends_with(".json")
            && !file_name.contains('/')
            && !file_name.contains('\\')
            && !file_name.contains("..");
        if !valid {
            return Err(AppError::Config(format!(
                "不正なバックアップファイル名: {}",
                file_name
            )));
        }

        let dir = self
            .config_path
            .parent()
            .ok_or_else(|| AppError::Config("設定ディレクトリの取得に失敗".to_string()))?;
        let backup_path = dir.join(file_name);

        if !backup_path.exists() {
            return Err(AppError::Config(format!(
                "バックアップファイルが見つかりません: {}",
                file_name
            )));
        }

        // バックアップを読み込んで有効な JSON (AppConfig) か確認
        let content = fs::read_to_string(&backup_path)?;
        let restored: AppConfig = serde_json::from_str(&content).map_err(|e| {
            AppError::Config(format!("バックアップファイルが無効です: {}", e))
        })?;

        // config.json を上書き
        fs::write(&self.config_path, serde_json::to_string_pretty(&restored)?)?;

        // in-memory 更新
        let mut guard = self.config.write().map_err(|e| {
            AppError::Config(format!("設定のロックに失敗: {}", e))
        })?;
        *guard = restored;

        tracing::info!(
            "バックアップから復旧: {} → config.json",
            crate::logging::redact_path(&backup_path)
        );
        Ok(())
    }
}
