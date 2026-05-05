//! CursorForge 設定管理モジュール
//!
//! アプリケーション設定の Source of Truth を Rust 側で管理する。
//! 設定は `config.json` に永続化し、UIが閉じていても常駐プロセスが参照できる。

use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use uuid::Uuid;

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
    /// %LOCALAPPDATA%/CursorForge/config.json
    fn config_file_path() -> AppResult<PathBuf> {
        let local_data = dirs::data_local_dir().ok_or_else(|| {
            AppError::Config("LocalAppData ディレクトリが見つかりません".to_string())
        })?;
        Ok(local_data.join("CursorForge").join("config.json"))
    }

    /// 設定マネージャーを初期化する
    /// 既存の設定ファイルがあれば読み込み、なければデフォルト値で作成
    pub fn init() -> AppResult<Self> {
        let config_path = Self::config_file_path()?;

        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: AppConfig = serde_json::from_str(&content).map_err(|e| {
                AppError::Config(format!("設定ファイルの解析に失敗: {}", e))
            })?;

            // スキーマバージョンチェック（将来のマイグレーション用）
            if config.schema_version > CURRENT_SCHEMA_VERSION {
                return Err(AppError::Config(format!(
                    "設定ファイルのバージョン({})が対応範囲外です。アプリの更新が必要です。",
                    config.schema_version
                )));
            }

            config
        } else {
            let config = AppConfig::default();
            // ディレクトリがなければ作成
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = serde_json::to_string_pretty(&config)?;
            fs::write(&config_path, content)?;
            config
        };

        // カーソル保存ディレクトリも事前に作成
        let cursors_dir = Self::cursors_dir()?;
        if !cursors_dir.exists() {
            fs::create_dir_all(&cursors_dir)?;
        }

        Ok(Self {
            config: RwLock::new(config),
            config_path,
        })
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
}
