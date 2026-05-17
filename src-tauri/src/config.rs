//! EasyCursorSwap 設定管理モジュール
//!
//! アプリケーション設定の Source of Truth を Rust 側で管理する。
//! 設定は `config.json` に永続化し、UIが閉じていても常駐プロセスが参照できる。

use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use uuid::Uuid;

/// バックアップファイルの情報
#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    /// ファイル名 (例: "config.corrupt.1746123456.json")
    pub file_name: String,
    /// UTC の ISO 8601 最終更新日時
    pub modified_utc: String,
    /// ファイルサイズ (バイト)
    pub size_bytes: u64,
    /// "corrupt" 固定 (パースエラー時の退避ファイル)
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

    /// セキュリティ設定
    pub security: SecurityConfig,

    /// ログ設定
    pub logging: LoggingConfig,

    /// Marketplace 提出用 GitHub アカウント (Device Flow で連携済みの場合のみ Some)。
    /// token 本体は `keystore.rs` の DPAPI スロットに別保管し、ここはメタのみ。
    /// v1 スキーマ互換のため `serde(default)` で `None` フォールバック。
    #[serde(default)]
    pub github_account: Option<GithubAccount>,
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
    /// 有効にすると、ビルド時に環境変数で埋め込まれた送信先エンドポイント / App Token
    /// (`EASY_CURSOR_SWAP_CRASH_REPORT_ENDPOINT` / `_APP_TOKEN`) を用いて
    /// Cloudflare Worker (private repo: <https://github.com/nishiuriraku/easy-cursor-swap-crash-report-worker>) に POST し、
    /// `nishiuriraku/easy-cursor-swap` の Issue として転送される。
    /// 環境変数未設定でビルドされた場合は本フラグが true でも送信は行われない。
    #[serde(default)]
    pub crash_reporting: bool,

    /// お気に入り登録されたテーマ ID。Library 画面の星マークで永続化する。
    /// 旧スキーマ互換のため `serde(default)` で空配列にフォールバック。
    #[serde(default)]
    pub favorites: Vec<Uuid>,

    /// テーマごとの利用統計 (適用回数 + 最終適用日時)。
    /// Library 画面の「最近使用」フィルタと sortApplied 用。
    /// 旧スキーマ互換のため `serde(default)`。
    #[serde(default)]
    pub usage: HashMap<Uuid, ThemeUsage>,
}

/// テーマ利用統計 (1 テーマあたり)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeUsage {
    /// 累積適用回数
    pub apply_count: u32,
    /// 最終適用日時 (RFC3339)
    pub last_applied_at: Option<String>,
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

/// Marketplace 提出フローで連携した GitHub アカウントのメタ情報。
/// アクセストークン本体は `keystore.rs` の DPAPI スロット (`_keys/github_oauth.token`)
/// に別保管し、ここはユーザーへの表示と「いつ連携したか」の記録のみ持つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubAccount {
    /// GitHub のログイン名 (例: "octocat")
    pub login: String,
    /// トークンを保存した日時 (RFC3339, UTC)
    pub token_saved_at: String,
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
                favorites: Vec::new(),
                usage: HashMap::new(),
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
            github_account: None,
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

/// `CUSTOM_CURSORS_DIR_OVERRIDE` を読む `ConfigManager::cursors_dir()` は
/// プロセス全体で env var を共有するため、override を使うテストはこの共有ロックで直列化する。
/// クレート横断 (`config::tests` / `commands::theme::tests` / `marketplace::tests` 等) の
/// 並走でも 1 つのミューテックスを共有することで env var の競合を防ぐ。
#[cfg(test)]
pub(crate) fn cursors_dir_override_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

impl ConfigManager {
    /// カーソル保存ディレクトリのパスを返す
    /// ~/.custom_cursors/
    ///
    /// テスト時は `CUSTOM_CURSORS_DIR_OVERRIDE` 環境変数で上書きできる。
    pub fn cursors_dir() -> AppResult<PathBuf> {
        #[cfg(test)]
        if let Ok(override_path) = std::env::var("CUSTOM_CURSORS_DIR_OVERRIDE") {
            return Ok(PathBuf::from(override_path));
        }
        let home = dirs::home_dir()
            .ok_or_else(|| AppError::Config("ホームディレクトリが見つかりません".to_string()))?;
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
    ///     - 同じか古い → そのまま使用 (旧フィールド欠落は `serde(default)` で透過補填)
    ///     - 新しい → アプリ更新が必要 → エラー (`Config(...)` を返し、main 側で専用画面表示)
    ///  3. ファイルあり → パース失敗 → `config.corrupt.{ts}.json` に退避してデフォルトで再作成
    pub fn init() -> AppResult<Self> {
        let config_path = Self::config_file_path()?;

        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            match serde_json::from_str::<AppConfig>(&content) {
                Ok(parsed) => Self::handle_versioned(parsed)?,
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

    /// schema_version を検査し、CURRENT_SCHEMA_VERSION より新しければエラーを返す。
    /// 古い場合 (将来 v2 を導入してから戻った場合等) は `serde(default)` による
    /// 透過的なフィールド補填だけ行い、書き換えはしない。
    fn handle_versioned(config: AppConfig) -> AppResult<AppConfig> {
        if config.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(AppError::Config(format!(
                "設定ファイルのバージョン ({}) はこのアプリ ({}) より新しいです。\nアプリの更新が必要です。",
                config.schema_version, CURRENT_SCHEMA_VERSION
            )));
        }
        Ok(config)
    }

    /// パース不可な設定ファイルを `config.corrupt.{epoch}.json` に退避する。
    fn backup_corrupt(config_path: &Path, raw: &str, reason: &str) -> AppResult<()> {
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
        let config = self
            .config
            .read()
            .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
        Ok(config.clone())
    }

    /// 設定を更新し、ディスクに永続化する
    pub fn update<F>(&self, updater: F) -> AppResult<AppConfig>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self
            .config
            .write()
            .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;

        updater(&mut config);

        // ディスクに保存
        let content = serde_json::to_string_pretty(&*config)?;
        fs::write(&self.config_path, content)?;

        Ok(config.clone())
    }

    /// 設定ディレクトリ内のバックアップファイル一覧を返す。
    ///
    /// 対象: `config.corrupt.*.json` (パースエラー時の退避ファイル)
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
                if !(name.starts_with("config.corrupt.") && name.ends_with(".json")) {
                    return None;
                }
                let kind = "corrupt";
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
    /// セキュリティ: `file_name` は `config.corrupt.*.json` のみ許可。
    pub fn restore_backup(&self, file_name: &str) -> AppResult<()> {
        // ファイル名の簡易バリデーション (パストラバーサル防止)
        let valid = file_name.starts_with("config.corrupt.")
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
        let restored: AppConfig = serde_json::from_str(&content)
            .map_err(|e| AppError::Config(format!("バックアップファイルが無効です: {}", e)))?;

        // config.json を上書き
        fs::write(&self.config_path, serde_json::to_string_pretty(&restored)?)?;

        // in-memory 更新
        let mut guard = self
            .config
            .write()
            .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
        *guard = restored;

        tracing::info!(
            "バックアップから復旧: {} → config.json",
            crate::logging::redact_path(&backup_path)
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_uses_current_schema_version() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn default_has_sane_security_thresholds() {
        // 仕様書 §「セキュリティ」: 50/200/10/1024 MB の固定上限
        let cfg = AppConfig::default();
        assert_eq!(cfg.security.max_pack_compressed_size, 50 * 1024 * 1024);
        assert_eq!(cfg.security.max_pack_uncompressed_size, 200 * 1024 * 1024);
        assert_eq!(cfg.security.max_image_file_size, 10 * 1024 * 1024);
        assert_eq!(cfg.security.storage_warning_threshold, 1024 * 1024 * 1024);
    }

    #[test]
    fn default_panic_hotkey_is_ctrl_alt_shift_r() {
        // パニックボタンの既定は仕様で固定。ユーザーが変更する前は必ずこの値。
        let cfg = AppConfig::default();
        assert_eq!(cfg.general.panic_hotkey, "Ctrl+Alt+Shift+R");
    }

    #[test]
    fn default_crash_reporting_is_opt_in() {
        // プライバシー優先で既定は false。ユーザーが明示 ON にしないと送信しない。
        let cfg = AppConfig::default();
        assert!(!cfg.general.crash_reporting);
    }

    #[test]
    fn json_roundtrip_preserves_all_fields() {
        // serde で書き出して読み戻して同一になることを確認する。
        let original = AppConfig::default();
        let json = serde_json::to_string(&original).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.schema_version, original.schema_version);
        assert_eq!(restored.general.auto_start, original.general.auto_start);
        assert_eq!(restored.general.language, original.general.language);
        assert_eq!(restored.general.panic_hotkey, original.general.panic_hotkey);
        assert_eq!(
            restored.general.crash_reporting,
            original.general.crash_reporting
        );
        assert_eq!(
            restored.security.max_pack_compressed_size,
            original.security.max_pack_compressed_size
        );
        assert_eq!(
            restored.logging.retention_days,
            original.logging.retention_days
        );
    }

    #[test]
    fn deserialize_accepts_missing_crash_reporting() {
        // 旧スキーマ (crash_reporting が無い) からの後方互換: serde(default) で false。
        // JSON 中に残っている "dark_mode" ブロックは新スキーマでは未知フィールドとなり、
        // serde の既定挙動 (deny_unknown_fields 未指定) で読み飛ばされる。
        // これにより既存ユーザーの config.json から dark_mode キーが自然消滅する
        // ソフトマイグレーションが壊れていないことを兼ねて検証している。
        let json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R"
            },
            "dark_mode": {
                "enabled": false,
                "light_theme_id": null,
                "dark_theme_id": null
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
        let cfg: AppConfig = serde_json::from_str(json).expect("legacy schema should parse");
        assert!(!cfg.general.crash_reporting);
        assert_eq!(cfg.general.language, "ja");
    }

    #[test]
    fn deserialize_accepts_missing_favorites_and_usage() {
        // 旧スキーマ (favorites / usage が無い) は serde(default) で空コレクションになる。
        // dark_mode は新スキーマで削除済だが、旧 config.json には残っている。
        // serde が未知フィールドを読み飛ばすことで透過マイグレーションする。
        let json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false
            },
            "dark_mode": {
                "enabled": false,
                "light_theme_id": null,
                "dark_theme_id": null
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
        let cfg: AppConfig = serde_json::from_str(json).expect("legacy schema should parse");
        assert!(cfg.general.favorites.is_empty());
        assert!(cfg.general.usage.is_empty());
    }

    #[test]
    fn default_favorites_and_usage_are_empty() {
        let cfg = AppConfig::default();
        assert!(cfg.general.favorites.is_empty());
        assert!(cfg.general.usage.is_empty());
    }

    #[test]
    fn deserialize_accepts_missing_github_account() {
        // v1 config (github_account 欠落) は serde(default) で None になる。
        let json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
        let cfg: AppConfig = serde_json::from_str(json).expect("legacy schema should parse");
        assert!(cfg.github_account.is_none());
    }

    #[test]
    fn github_account_round_trips_through_json() {
        // struct update 構文で clippy::field_reassign_with_default を回避する。
        let cfg = AppConfig {
            github_account: Some(GithubAccount {
                login: "octocat".to_string(),
                token_saved_at: "2026-05-14T12:00:00Z".to_string(),
            }),
            ..AppConfig::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.github_account.as_ref().unwrap().login, "octocat");
        assert_eq!(
            back.github_account.as_ref().unwrap().token_saved_at,
            "2026-05-14T12:00:00Z"
        );
    }

    #[test]
    fn default_schema_version_is_current() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.schema_version, super::CURRENT_SCHEMA_VERSION);
        assert_eq!(cfg.schema_version, 1);
    }

    #[test]
    fn cursors_dir_is_under_home() {
        // ~/.custom_cursors/ をホーム配下に解決できる。
        // 同プロセスで `CUSTOM_CURSORS_DIR_OVERRIDE` を設定する別テスト
        // (`commands::theme::tests`) と直列化するため、共有ロックを取得する。
        let _g = super::cursors_dir_override_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let saved = std::env::var("CUSTOM_CURSORS_DIR_OVERRIDE").ok();
        std::env::remove_var("CUSTOM_CURSORS_DIR_OVERRIDE");
        let dir = ConfigManager::cursors_dir().unwrap();
        if let Some(v) = saved {
            std::env::set_var("CUSTOM_CURSORS_DIR_OVERRIDE", v);
        }
        assert!(dir.ends_with(".custom_cursors"));
        if let Some(home) = dirs::home_dir() {
            assert!(dir.starts_with(&home));
        }
    }
}
