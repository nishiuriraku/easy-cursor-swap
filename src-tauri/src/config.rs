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
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(export))]
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
///
/// v1 → v2 (Wave 1A): 6 つの UI 設定フィールドを追加
///   - `general.show_apply_toast` (default true)
///   - `general.apply_shadow_control` (default true)
///   - `general.start_minimized` (default false)
///   - `general.show_storage_warning` (default true)
///   - `security.require_signed_themes` (default false)
///   - `security.warn_unsigned_import` (default true)
///
/// v1 → v2 変換は `ConfigManager::migrate` を経由する。v1 構造体は
/// `config/v1.rs` に明示分離してあり、`#[serde(default)]` により v1 JSON は
/// そのまま v2 AppConfig にも deserialize 可能(透過フォールバック)。
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// 設定スキーマ v1 のスナップショットモジュール。
pub mod v1;

/// 設定 update 用の typed patch (Wave 2B / Task 3)。
/// `update_config` IPC の入力型を `AppConfigPatch` に固定し、フロントから
/// `schema_version` / `github_account` / セキュリティ閾値 / 履歴系フィールド
/// (`favorites` / `usage` / `active_theme_id`) を書き換えられないようにする。
pub mod patch;

/// 設定ファイル書込時に使う temp 拡張子。同じディレクトリに `config.json` と
/// `config.json.tmp` が並ぶ形になり、電源断 / プロセス落ちで原子的置換が
/// 中断しても temp ファイルが残るのみで本ファイルは無傷。
const CONFIG_TEMP_SUFFIX: &str = "json.tmp";

/// 指定パスへファイル内容を atomic 的に書き込む。
///
/// `fs::write` 直書きは電源断 / プロセス落ちで対象ファイルが中途半端な
/// サイズ/内容になり、次回起動時の `serde_json::from_str` 失敗 → `config.corrupt.*`
/// 退避 → デフォルト復元 → ユーザー設定消失、という復旧不能な連鎖を起こす。
/// 本ヘルパーは temp ファイルへ書いてから `fs::rename` で 1 ステップで
/// 置換することで、ディスク側で常に「旧ファイル」か「新ファイル」のいずれかが
/// 完全に観測できる状態しか存在しなくなる。
///
/// 失敗時は temp ファイルを削除して元のパスを一切変更しない (呼び出し側が
/// in-memory ロールバックを判断する)。
fn atomic_write(path: &Path, content: &str) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::Config(format!(
            "設定ファイルの親ディレクトリが取得できません: {}",
            path.display()
        ))
    })?;
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    let temp = path.with_extension(CONFIG_TEMP_SUFFIX);
    // 前回クラッシュ時に temp が残っていても今回書込みで上書きする前に掃除する
    // (複数 temp が累積する事態を防ぐ)。
    let _ = fs::remove_file(&temp);

    fs::write(&temp, content).map_err(|e| {
        AppError::Config(format!(
            "設定ファイル temp への書き出し失敗 ({}): {}",
            crate::logging::redact_path(&temp),
            e
        ))
    })?;

    if let Err(e) = fs::rename(&temp, path) {
        // rename 失敗: temp が残骸として残らないよう掃除してからエラーを返す。
        let _ = fs::remove_file(&temp);
        return Err(AppError::Config(format!(
            "設定ファイル atomic 置換失敗 ({} → {}): {}",
            crate::logging::redact_path(&temp),
            crate::logging::redact_path(path),
            e
        )));
    }
    Ok(())
}

/// pack (.cursorpack) 圧縮サイズの既定上限 (50 MB)。
///
/// `import_cursorpack_bytes` / `inspect_cursorpack_bytes` / `submit_theme_auto` の
/// 上限チェックで参照する。実行時に `SecurityConfig::max_pack_compressed_size` を
/// 変更しても本 const は不変 — 「default 値の SoT」として機能する。
/// runtime config 値を読む経路は別 PR で API カスケード変更とともに導入予定。
pub const DEFAULT_MAX_PACK_COMPRESSED_SIZE: u64 = 50 * 1024 * 1024;

/// pack 展開後合計サイズの既定上限 (200 MB)。zip 爆弾の最終防衛線。
///
/// `import_cursorpack_bytes` などの累積カウンタで参照する「default 値の SoT」。
pub const DEFAULT_MAX_PACK_UNCOMPRESSED_SIZE: u64 = 200 * 1024 * 1024;

/// pack 内 1 ファイルあたりの実サイズ既定上限 (10 MB)。
///
/// 申告サイズ (`entry.size()`) ではなく実伸長バイト数を `io::copy` の `take` で
/// 打ち切る基準としても使う「default 値の SoT」。
pub const DEFAULT_MAX_IMAGE_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// `serde(default = "default_true")` 用ヘルパー。`#[serde(default)]` だけだと
/// `Default::default()` の bool::default() (= false) が入るので、bool 既定が true
/// なフィールドではこの関数を指定する。
fn default_true() -> bool {
    true
}

/// アプリケーション設定（Source of Truth）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(export))]
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
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
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

    /// テーマ適用時のトースト通知を表示するか (Wave 1A で追加、default true)。
    /// 旧 v1 JSON には存在しないため `serde(default)` で true フォールバック。
    /// 消費側: テーマ適用成功/失敗時のトースト表示分岐。
    #[serde(default = "default_true")]
    pub show_apply_toast: bool,

    /// カーソル影の ON/OFF 制御をアプリが行うか (Wave 1A で追加、default true)。
    /// true = 新テーマの `requires_os_shadow` を `SPI_SETCURSORSHADOW` に反映。
    /// false = 影制御を行わない (Windows 既定挙動を維持)。
    #[serde(default = "default_true")]
    pub apply_shadow_control: bool,

    /// `--autostart` 起動時にウィンドウを最小化状態で起動するか
    /// (Wave 1A で追加、default false)。手動起動は常にウィンドウ表示。
    /// 消費側: `main.rs` の autostart 分岐で `WindowBuilder::visible(false)`。
    #[serde(default)]
    pub start_minimized: bool,

    /// ストレージ使用量が閾値超過したときの警告トーストを表示するか
    /// (Wave 1A で追加、default true)。
    /// 消費側: Library 画面のストレージ警告 UI。
    #[serde(default = "default_true")]
    pub show_storage_warning: bool,
}

/// テーマ利用統計 (1 テーマあたり)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
pub struct ThemeUsage {
    /// 累積適用回数
    pub apply_count: u32,
    /// 最終適用日時 (RFC3339)
    pub last_applied_at: Option<String>,
}

/// セキュリティ閾値設定
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
pub struct SecurityConfig {
    /// .cursorpack 圧縮時サイズ上限 (バイト)
    pub max_pack_compressed_size: u64,
    /// 解凍後合計サイズ上限 (バイト)
    pub max_pack_uncompressed_size: u64,
    /// 個別画像ファイルサイズ上限 (バイト)
    pub max_image_file_size: u64,
    /// ストレージ警告閾値 (バイト)
    pub storage_warning_threshold: u64,

    /// 未署名 .cursorpack のインポートを Rust 境界で拒否するか
    /// (Wave 1A で追加、default false)。
    /// true = ローカル `.cursorpack` インポートをテーマ JSON の `signature` が
    ///   Some かつ marketplace の `verify_signature` 経路を通ったもののみ許可。
    /// false = 既存挙動 (warn_unsigned_import トーストのみで通す)。
    /// marketplace install は既存 Ed25519 検証があるため追加対応なし。
    #[serde(default)]
    pub require_signed_themes: bool,

    /// 未署名 .cursorpack インポート時に確認ダイアログを出すか
    /// (Wave 1A で追加、default true)。
    /// require_signed_themes=false のときのみ意味を持つ。
    #[serde(default = "default_true")]
    pub warn_unsigned_import: bool,
}

/// ログ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
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
            general: GeneralConfig::default(),
            security: SecurityConfig::default(),
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

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            auto_start: true,
            auto_update: true,
            language: "auto".to_string(),
            active_theme_id: None,
            panic_hotkey: "Ctrl+Alt+Shift+R".to_string(),
            crash_reporting: false,
            favorites: Vec::new(),
            usage: HashMap::new(),
            // v2 で追加 (Wave 1A)。既定値は spec `develop/easy-cursor-swap/log/2026-07-25.md` の
            // Wave 1B セクションに準拠。
            show_apply_toast: true,
            apply_shadow_control: true,
            start_minimized: false,
            show_storage_warning: true,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_pack_compressed_size: DEFAULT_MAX_PACK_COMPRESSED_SIZE,
            max_pack_uncompressed_size: DEFAULT_MAX_PACK_UNCOMPRESSED_SIZE,
            max_image_file_size: DEFAULT_MAX_IMAGE_FILE_SIZE,
            // 1 GB
            storage_warning_threshold: 1024 * 1024 * 1024,
            // v2 で追加 (Wave 1A)。
            require_signed_themes: false,
            warn_unsigned_import: true,
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
    /// `CUSTOM_CURSORS_DIR_OVERRIDE` が設定されている場合はそのパスで上書きする。
    pub fn cursors_dir() -> AppResult<PathBuf> {
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
    ///
    /// 書込みはどちらも [`atomic_write`] 経由で temp+rename するため、
    /// 電源断 / プロセス落ちで設定ファイルが中途半端な状態にならない。
    pub fn init() -> AppResult<Self> {
        let config_path = Self::config_file_path()?;
        let config = Self::load_or_initialize(&config_path)?;

        let cursors_dir = Self::cursors_dir()?;
        if !cursors_dir.exists() {
            fs::create_dir_all(&cursors_dir)?;
        }

        Ok(Self {
            config: RwLock::new(config),
            config_path,
        })
    }

    /// テスト専用コンストラクタ。本番コードは `init()` を使う。
    ///
    /// ロックや env var を経由せず、与えられたパスをそのまま Source of Truth として
    /// 初期化する。`() テストの tempdir ベースで実ファイルへの永続化を検証する
    /// 用途を想定 (atomic_write 経路の回帰検知)。
    #[cfg(test)]
    pub(crate) fn init_at(config_path: &Path) -> AppResult<Self> {
        let config = Self::load_or_initialize(config_path)?;
        Ok(Self {
            config: RwLock::new(config),
            config_path: config_path.to_path_buf(),
        })
    }

    /// 設定ファイルを読み込み、必要ならデフォルトを atomic 書きする共通処理。
    /// `init()` / `init_at()` 両方から呼ばれる。
    ///
    /// Wave 1A: 設定 JSON 読み込みは `Self::migrate(&content)` 経由に統一。
    /// v1 / v2 / 不正 の 3 ケースを `migrate` が判定し、v1 の場合は V2 へアップグレード +
    /// atomic_write で永続化する。これにより「通常起動」「`restore_config_backup`」
    /// 「`.cursorprofile` import」の 3 入力経路が同一の V1→V2 変換を共有する。
    fn load_or_initialize(config_path: &Path) -> AppResult<AppConfig> {
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            match Self::migrate(&content) {
                Ok((migrated, changed)) => {
                    if changed {
                        // v1 → v2 昇格が発生したので、永続化して次回からは v2 を読む。
                        atomic_write(config_path, &serde_json::to_string_pretty(&migrated)?)?;
                    }
                    Ok(migrated)
                }
                Err(e) => {
                    // パース失敗 → 退避して新規作成
                    Self::backup_corrupt(config_path, &content, &e.to_string())?;
                    let fresh = AppConfig::default();
                    atomic_write(config_path, &serde_json::to_string_pretty(&fresh)?)?;
                    tracing::warn!("設定ファイルが破損していたためデフォルトで再作成しました");
                    Ok(fresh)
                }
            }
        } else {
            let fresh = AppConfig::default();
            atomic_write(config_path, &serde_json::to_string_pretty(&fresh)?)?;
            Ok(fresh)
        }
    }

    /// 設定 JSON 文字列を V2 AppConfig へ変換する共通エントリ。
    ///
    /// 3 つの入力経路(通常起動 / `restore_config_backup` / `.cursorprofile` import)が
    /// ここを共有する。返り値:
    ///   - `(AppConfig, false)`: 入力は既に V2。永続化不要。
    ///   - `(AppConfig, true)`: 入力は V1。V2 へ昇格したので永続化が望ましい
    ///     (= 呼び出し側が atomic_write で書き戻す)。
    ///   - `Err`: 不正 JSON / V1 としてパース不能 / 新しすぎる schema_version。
    ///
    /// V1 → V2 変換ではユーザー設定 (auto_start / language / favorites / usage /
    /// github_account / max_pack_* / storage_warning_threshold / logging) はそのまま
    /// 引き継ぎ、V2 で追加された 6 フィールドは V2 既定値で初期化する。
    pub fn migrate(raw_json: &str) -> AppResult<(AppConfig, bool)> {
        // まず schema_version だけ先読みして分岐する。
        // 直接 AppConfig / AppConfigV1 のどちらにも deserialize せず、Value 経由で
        // スキーマ番号を確実に拾う (未知フィールドで失敗しないように)。
        let value: serde_json::Value = serde_json::from_str(raw_json)
            .map_err(|e| AppError::Config(format!("設定 JSON のパースに失敗: {}", e)))?;
        let schema_version = value
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                AppError::Config("設定 JSON に schema_version がありません".to_string())
            })? as u32;

        match schema_version {
            1 => {
                // V1 → V2 変換
                let v1: v1::AppConfigV1 = serde_json::from_value(value).map_err(|e| {
                    AppError::Config(format!("v1 設定の解釈に失敗: {}", e))
                })?;
                Ok((v1.into_v2(), true))
            }
            n if n == CURRENT_SCHEMA_VERSION => {
                // 既に V2。
                let cfg: AppConfig = serde_json::from_value(value).map_err(|e| {
                    AppError::Config(format!("v2 設定の解釈に失敗: {}", e))
                })?;
                Ok((cfg, false))
            }
            n if n > CURRENT_SCHEMA_VERSION => Err(AppError::Config(format!(
                "設定ファイルのバージョン ({}) はこのアプリ ({}) より新しいです。\nアプリの更新が必要です。",
                n, CURRENT_SCHEMA_VERSION
            ))),
            n => Err(AppError::Config(format!(
                "設定ファイルのバージョン ({}) はこのアプリ ({}) より古すぎて解釈できません。",
                n, CURRENT_SCHEMA_VERSION
            ))),
        }
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
    ///
    /// 永続化は [`atomic_write`] 経由 (temp 書き出し + `fs::rename`) で原子的に
    /// 行う。書込みが失敗した場合は in-memory 状態もコミットせず、呼び出し側に
    /// エラーを返す。これにより「メモリ側は新状態、ディスクは旧/空」という
    /// 最悪の不整合を残さない。
    ///
    /// ロック獲得を `read` → クローン用 `draft` 作成 → `write` コミット、の
    /// 2 段にする理由は、rename 失敗時に `*config` へ書き戻す操作 (= 副作用)
    /// を「アトミック失敗」の中でする必要をなくすため。
    pub fn update<F>(&self, updater: F) -> AppResult<AppConfig>
    where
        F: FnOnce(&mut AppConfig),
    {
        // 1. 現在のスナップショットを取り、updater を適用した draft を作る
        //    (read lock 内で完結するため、副作用は発生しない)。
        let draft = {
            let guard = self
                .config
                .read()
                .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
            let mut draft = guard.clone();
            updater(&mut draft);
            draft
        };

        // 2. ディスクへ atomic 書き出し。失敗時は in-memory を一切変えない。
        let content = serde_json::to_string_pretty(&draft)?;
        atomic_write(&self.config_path, &content)?;

        // 3. 書込み成功を確認できたので、ようやく in-memory を commit。
        let mut guard = self
            .config
            .write()
            .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
        *guard = draft.clone();

        Ok(draft)
    }

    /// `update_config` IPC から呼ばれる typed-patch 適用の正準エントリ。
    ///
    /// `update<F>` クロージャ版と異なり、patch に存在しないフィールドは
    /// 一切触らない (`schema_version` / `github_account` / セキュリティ閾値 /
    /// `favorites` / `usage` / `active_theme_id` は patch に含まれないため、
    /// フロントから上書きされることは決してない)。
    ///
    /// ディスク永続化は `update<F>` と同じく atomic_write 経由。
    /// 失敗時は in-memory を変えない (Task 0 baseline で確認済みの安全網)。
    pub fn apply_patch(&self, patch: patch::AppConfigPatch) -> AppResult<AppConfig> {
        // 1. draft 作成 (read lock 内、副作用なし)
        let draft = {
            let guard = self
                .config
                .read()
                .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
            let mut draft = guard.clone();
            patch.apply_to(&mut draft);
            draft
        };

        // 2. ディスクへ atomic 書き出し
        let content = serde_json::to_string_pretty(&draft)?;
        atomic_write(&self.config_path, &content)?;

        // 3. in-memory commit
        let mut guard = self
            .config
            .write()
            .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
        *guard = draft.clone();
        Ok(draft)
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

        // バックアップを読み込んで有効な JSON (V1 / V2) か確認。
        // Wave 1A: `Self::migrate` 経由で V1 → V2 変換も許容する。
        // 古いバックアップ (v1 schema) もそのまま復元できるようになった。
        let content = fs::read_to_string(&backup_path)?;
        let (restored, _) = Self::migrate(&content)
            .map_err(|e| AppError::Config(format!("バックアップファイルが無効です: {}", e)))?;

        // config.json を atomic 書込み (temp → rename) で置換
        atomic_write(&self.config_path, &serde_json::to_string_pretty(&restored)?)?;

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
        assert_eq!(cfg.schema_version, 2);
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
        assert_eq!(cfg.schema_version, 2);
    }

    // ── Wave 2B / Task 3: typed-patch apply テスト ───────────────────

    /// `apply_patch` 経由で `general.show_apply_toast` を変更できる。
    #[test]
    fn apply_patch_changes_general_field() {
        let dir = make_tempdir("patch-1");
        let path = dir.join("config.json");
        let cm = ConfigManager::init_at(&path).unwrap();
        assert!(cm.get().unwrap().general.show_apply_toast);

        let patch = patch::AppConfigPatch {
            general: Some(patch::GeneralConfigPatch {
                show_apply_toast: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };
        let updated = cm.apply_patch(patch).unwrap();
        assert!(!updated.general.show_apply_toast);
        let _ = fs::remove_dir_all(&dir);
    }

    /// `apply_patch` は patch に含まれないフィールドを一切変更しない
    /// (e.g. `github_account` を patch 経由で送ろうとしても無視される)。
    ///
    /// serde unknown field 拒否 + 型が patch に存在しない二重防御。
    #[test]
    fn apply_patch_does_not_touch_github_account_or_schema_version() {
        let dir = make_tempdir("patch-2");
        let path = dir.join("config.json");
        let cm = ConfigManager::init_at(&path).unwrap();
        let baseline_schema = cm.get().unwrap().schema_version;
        let baseline_github_login = cm
            .get()
            .unwrap()
            .github_account
            .as_ref()
            .map(|g| g.login.clone());

        let patch = patch::AppConfigPatch {
            logging: Some(patch::LoggingConfigPatch {
                level: Some("DEBUG".to_string()),
            }),
            ..Default::default()
        };
        let updated = cm.apply_patch(patch).unwrap();
        assert_eq!(updated.schema_version, baseline_schema);
        assert_eq!(
            updated.github_account.as_ref().map(|g| g.login.clone()),
            baseline_github_login
        );
        assert_eq!(updated.logging.level, "DEBUG");
        let _ = fs::remove_dir_all(&dir);
    }

    /// `general.auto_start` を含む patch は disk に永続化される。
    /// (Task 0 baseline で確認された in-memory / disk 不整合の安全網が
    /// patch 経路でも機能することの回帰防止)
    #[test]
    fn apply_patch_persists_to_disk_atomically() {
        let dir = make_tempdir("patch-3");
        let path = dir.join("config.json");
        let cm = ConfigManager::init_at(&path).unwrap();
        let patch = patch::AppConfigPatch {
            general: Some(patch::GeneralConfigPatch {
                language: Some("en".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        cm.apply_patch(patch).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let reloaded: AppConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(reloaded.general.language, "en");
        let _ = fs::remove_dir_all(&dir);
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

    // ===== G4: `update_config` を atomic write + rename 化する回帰テスト =====
    //
    // 既存の `fs::write` 直書きは電源断 / プロセス落ちで config.json を中途半端な
    // 状態 (末尾が切れた JSON) にしてしまう。temp ファイルへ書いてから `rename` で
    // 置換する atomic_write ヘルパーでこのクラスを潰す。
    //
    // テスト戦略:
    //  - `atomic_write` ヘルパー自体は副作用が単純 (ファイル 1 個) なので直接検証
    //  - `ConfigManager::update` は tempdir ベースの test-only コンストラクタ
    //    `init_at` 経由で生成し、原子的書き戻しと in-memory ロールバックを検証

    /// テスト専用の作業ディレクトリ。プロセス ID + ナノ秒 nonce で衝突回避。
    /// 既存 `cursors_dir_is_under_home` と同様にグローバル env を触らない設計。
    fn make_tempdir(label: &str) -> PathBuf {
        let pid = std::process::id();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ecs-config-test-{}-{}-{}", label, pid, nonce));
        fs::create_dir_all(&dir).expect("tempdir 作成");
        dir
    }

    /// atomic_write: 新規パスへの書き込み
    #[test]
    fn atomic_write_creates_target_file() {
        let dir = make_tempdir("create");
        let path = dir.join("config.json");
        super::atomic_write(&path, "{\"a\":1}").expect("atomic_write 成功");
        assert!(path.exists(), "書き込み先が存在するべき");
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// atomic_write: 既存ファイルの内容を置換する
    #[test]
    fn atomic_write_overwrites_existing_file() {
        let dir = make_tempdir("overwrite");
        let path = dir.join("config.json");
        super::atomic_write(&path, "initial").unwrap();
        super::atomic_write(&path, "updated").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "updated");
        let _ = fs::remove_dir_all(&dir);
    }

    /// atomic_write: rename 失敗時 (target がディレクトリ) は元のファイルが消えない
    /// (rename 前の状態に戻る) ことを確認する。fs::write 直書きだと
    /// target ディレクトリ配下の該当パスが真っ先に消えて壊れるため、これで
    /// ロールバック挙動の差分が固定できる。
    #[test]
    fn atomic_write_preserves_existing_file_on_failure() {
        let dir = make_tempdir("preserve");
        let path = dir.join("config.json");
        super::atomic_write(&path, "original").unwrap();
        // ターゲットを「ディレクトリ化」して rename を失敗させる
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();

        let result = super::atomic_write(&path, "new-content");
        assert!(result.is_err(), "rename 失敗時はエラーを返すべき");

        // 失敗後にファイル/ディレクトリが「壊れた中途半端な状態」になっていないこと
        // (ディレクトリが残っている = ターゲット内容は破壊されていない)
        assert!(
            path.is_dir(),
            "rename 失敗時にターゲットディレクトリが消えるべきでない"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// atomic_write: rename 失敗時は temp ファイルを掃除して残骸を残さない
    #[test]
    fn atomic_write_cleans_temp_on_failure() {
        let dir = make_tempdir("cleanup");
        let path = dir.join("config.json");
        super::atomic_write(&path, "original").unwrap();
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();

        let _ = super::atomic_write(&path, "new-content");
        let temp = path.with_extension("json.tmp");
        assert!(
            !temp.exists(),
            "失敗時に temp ファイルが残骸として残ってはいけない: {}",
            temp.display()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// `update` が atomic 経路で永続化されること: 書き戻し後の content に
    /// 上書きしたフィールドが反映されており、別 ConfigManager で再読込しても
    /// 同じ状態になることを検証する (ラウンドトリップの原子性)。
    #[test]
    fn update_persists_changes_atomically() {
        let dir = make_tempdir("persist");
        let path = dir.join("config.json");
        let mgr = ConfigManager::init_at(&path).expect("init_at");

        mgr.update(|c| {
            c.general.language = "en".to_string();
        })
        .expect("update 成功");

        // ファイル内容に反映されている
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("\"language\": \"en\""),
            "language=en がファイルに書かれていない: {}",
            content
        );

        // 別 ConfigManager で再読込しても同じ
        let mgr2 = ConfigManager::init_at(&path).expect("init_at reload");
        assert_eq!(mgr2.get().unwrap().general.language, "en");
        let _ = fs::remove_dir_all(&dir);
    }

    /// `update` のディスク書き込みが失敗した場合、in-memory 状態も変更されない
    /// (atomicity: メモリとディスクの不整合を残さない)。
    #[test]
    fn update_rolls_back_in_memory_state_on_disk_failure() {
        let dir = make_tempdir("rollback");
        let path = dir.join("config.json");
        let mgr = ConfigManager::init_at(&path).expect("init_at");

        // 初期状態 (default) を記録
        let before = mgr.get().unwrap().general.language.clone();
        assert_eq!(before, "auto");

        // init 完了後にターゲットをディレクトリ化 → 以降の update は rename 失敗する
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();

        let result = mgr.update(|c| {
            c.general.language = "en".to_string();
        });
        assert!(result.is_err(), "disk 失敗時は update がエラーを返すべき");

        // メモリ側が書き換えられていない (temp への update は commit しない)
        let after = mgr.get().unwrap().general.language.clone();
        assert_eq!(
            after, "auto",
            "disk 失敗時に in-memory 状態が変更されてはいけない"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// `update` で複数フィールドが同時に更新されるケースも原子的に反映される
    /// (temp 経由で書き戻し → in-memory へ一括コミット)。
    #[test]
    fn update_commits_multiple_fields_atomically() {
        let dir = make_tempdir("multi");
        let path = dir.join("config.json");
        let mgr = ConfigManager::init_at(&path).expect("init_at");

        mgr.update(|c| {
            c.general.language = "ja".to_string();
            c.general.crash_reporting = true;
            c.logging.level = "DEBUG".to_string();
        })
        .expect("update 成功");

        let cfg = mgr.get().unwrap();
        assert_eq!(cfg.general.language, "ja");
        assert!(cfg.general.crash_reporting);
        assert_eq!(cfg.logging.level, "DEBUG");

        // 永続化内容にも 3 フィールド全部入っている
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"language\": \"ja\""));
        assert!(content.contains("\"crash_reporting\": true"));
        assert!(content.contains("\"level\": \"DEBUG\""));
        let _ = fs::remove_dir_all(&dir);
    }

    // ===== Wave 1A: Config v2 migration tests =====
    //
    // V1 → V2 変換を `ConfigManager::migrate` に集約した特性化テスト群。
    // log/2026-07-25.md Wave 1A 仕様「テスト: V1 JSON 5 種 (欠落キー / 未知キー /
    // 旧型 `github_account` 不在 / 旧 `general` 欠落 / 旧 logging 欠落) → V2 復元」
    // を網羅する。
    //
    // 各テストは `migrate` を直接呼び、`(cfg, changed)` の両方を検証する:
    //   - `cfg.schema_version == 2` に昇格している
    //   - 6 新フィールドが V2 既定値で埋まっている
    //   - ユーザー設定が保持されている
    //   - `changed == true` (V1 → V2 変換が発生した)

    /// V1 JSON (欠落キー: `github_account` 不在) → V2 復元。
    /// 旧 `general` / `security` / `logging` すべて存在し、`github_account` だけ省略。
    #[test]
    fn migrate_v1_missing_github_account_to_v2() {
        let v1_json = r#"{
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
        let (cfg, changed) = ConfigManager::migrate(v1_json).expect("V1 → V2 成功");
        assert!(changed, "V1 → V2 変換は changed=true を返すべき");
        assert_eq!(cfg.schema_version, 2);
        assert!(
            cfg.github_account.is_none(),
            "旧 V1 に無い github_account は None"
        );
        // 6 新フィールドが V2 既定値で埋まる
        assert!(cfg.general.show_apply_toast);
        assert!(cfg.general.apply_shadow_control);
        assert!(!cfg.general.start_minimized);
        assert!(cfg.general.show_storage_warning);
        assert!(!cfg.security.require_signed_themes);
        assert!(cfg.security.warn_unsigned_import);
    }

    /// V1 JSON (未知キー: 旧 `dark_mode` ブロック) → V2 復元。
    /// `serde` 既定の未知フィールド黙殺で `dark_mode` は無視される。
    #[test]
    fn migrate_v1_with_unknown_dark_mode_block_to_v2() {
        let v1_json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false,
                "favorites": ["00000000-0000-0000-0000-000000000001"],
                "usage": {
                    "00000000-0000-0000-0000-000000000001": {
                        "apply_count": 5,
                        "last_applied_at": "2026-05-14T12:00:00Z"
                    }
                }
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
            },
            "github_account": {
                "login": "octocat",
                "token_saved_at": "2026-05-14T12:00:00Z"
            }
        }"#;
        let (cfg, changed) = ConfigManager::migrate(v1_json).expect("V1 → V2 成功");
        assert!(changed);
        // dark_mode は黙殺され、それ以外のフィールドが引き継がれる
        assert_eq!(cfg.general.favorites.len(), 1);
        assert_eq!(cfg.general.usage.len(), 1);
        assert_eq!(cfg.general.usage.values().next().unwrap().apply_count, 5);
        assert_eq!(cfg.github_account.as_ref().unwrap().login, "octocat");
        assert_eq!(cfg.schema_version, 2);
    }

    /// V1 JSON (旧 `general` の必須フィールド欠落: `crash_reporting` 不在) → V2 復元。
    /// `serde(default)` で `crash_reporting=false` にフォールバック。
    #[test]
    fn migrate_v1_missing_general_field_to_v2() {
        let v1_json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": false,
                "auto_update": true,
                "language": "en",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R"
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
        let (cfg, changed) = ConfigManager::migrate(v1_json).expect("V1 → V2 成功");
        assert!(changed);
        // crash_reporting は serde(default) で false にフォールバック
        assert!(!cfg.general.crash_reporting);
        // favorites / usage も serde(default) で空
        assert!(cfg.general.favorites.is_empty());
        assert!(cfg.general.usage.is_empty());
        // ユーザー指定のフィールドは保持
        assert!(!cfg.general.auto_start);
        assert_eq!(cfg.general.language, "en");
    }

    /// V1 JSON (旧 `logging` 欠落) はエラー (logging は serde(default) 不在の必須フィールド)。
    /// この場合、v1 struct の deserialize 段階で失敗する。
    #[test]
    fn migrate_v1_missing_logging_block_returns_error() {
        let v1_json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R"
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            }
        }"#;
        let result = ConfigManager::migrate(v1_json);
        assert!(
            result.is_err(),
            "logging 欠落 V1 は v1 struct の serde 段階で失敗するべき"
        );
    }

    /// V1 JSON (完全に最小: 必須フィールドのみ) → V2 復元。
    /// `serde(default)` で補えるものはすべて補填される。
    #[test]
    fn migrate_v1_minimal_to_v2() {
        let v1_json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "auto",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R"
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
        let (cfg, changed) = ConfigManager::migrate(v1_json).expect("V1 → V2 成功");
        assert!(changed);
        assert_eq!(cfg.schema_version, 2);
        assert_eq!(cfg.general.language, "auto");
        // 6 新フィールドがすべて V2 既定値で埋まる
        assert!(cfg.general.show_apply_toast);
        assert!(cfg.general.apply_shadow_control);
        assert!(!cfg.general.start_minimized);
        assert!(cfg.general.show_storage_warning);
        assert!(!cfg.security.require_signed_themes);
        assert!(cfg.security.warn_unsigned_import);
    }

    /// V2 JSON → V2 復元。`changed == false` で永続化が不要。
    #[test]
    fn migrate_v2_returns_unchanged() {
        let v2_json = r#"{
            "schema_version": 2,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false,
                "show_apply_toast": false,
                "apply_shadow_control": false,
                "start_minimized": true,
                "show_storage_warning": false
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824,
                "require_signed_themes": true,
                "warn_unsigned_import": false
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
        let (cfg, changed) = ConfigManager::migrate(v2_json).expect("V2 → V2 成功");
        assert!(!changed, "V2 → V2 は changed=false");
        assert_eq!(cfg.schema_version, 2);
        assert!(!cfg.general.show_apply_toast, "ユーザー設定 false を保持");
        assert!(cfg.general.start_minimized);
        assert!(cfg.security.require_signed_themes);
        assert!(!cfg.security.warn_unsigned_import);
    }

    /// `schema_version` が CURRENT より新しい → エラー (現行挙動と同一)。
    #[test]
    fn migrate_future_version_returns_error() {
        let future_json = r#"{ "schema_version": 99 }"#;
        let result = ConfigManager::migrate(future_json);
        assert!(result.is_err(), "schema_version=99 はエラー");
    }

    /// `schema_version` 欠落 → エラー。
    #[test]
    fn migrate_missing_schema_version_returns_error() {
        let no_version = r#"{ "general": {} }"#;
        let result = ConfigManager::migrate(no_version);
        assert!(result.is_err(), "schema_version なしはエラー");
    }

    /// 不正 JSON (括弧不一致) → エラー。
    #[test]
    fn migrate_invalid_json_returns_error() {
        let result = ConfigManager::migrate("{ not valid json");
        assert!(result.is_err());
    }

    /// `load_or_initialize` が V1 ファイルを V2 に migrate して atomic_write で永続化する E2E 確認。
    #[test]
    fn load_or_initialize_v1_file_persists_as_v2() {
        let dir = make_tempdir("v1-migrate");
        let path = dir.join("config.json");

        // V1 JSON を手書きで配置 (= Wave 1A 移行を模擬)。
        let v1_json = r#"{
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
        fs::write(&path, v1_json).expect("V1 config.json 書き込み");

        // 起動を模擬: V1 ファイルを読み込んで V2 に migrate する。
        let mgr = ConfigManager::init_at(&path).expect("init_at");
        let cfg = mgr.get().unwrap();

        // 1. in-memory は V2
        assert_eq!(cfg.schema_version, 2);
        assert!(cfg.general.show_apply_toast);

        // 2. ディスクも V2 に書き戻されている (=次回以降は migrate 不要)
        let persisted = fs::read_to_string(&path).unwrap();
        assert!(persisted.contains("\"schema_version\": 2"));
        assert!(persisted.contains("\"show_apply_toast\": true"));
        assert!(persisted.contains("\"warn_unsigned_import\": true"));

        let _ = fs::remove_dir_all(&dir);
    }
}
