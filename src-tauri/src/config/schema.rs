//! 設定スキーマ (Source of Truth 型) と既定値。
//!
//! `AppConfig` とその子型、`CURRENT_SCHEMA_VERSION`、サイズ上限 `DEFAULT_*` の SoT。
//! 永続化ロジックは `store.rs`、v1→v2 変換は `migrate.rs`、update 用 typed patch は `patch.rs`。
//! ts-rs (`--features typegen`) の生成対象は `AppConfig` / `BackupInfo` (export) と
//! その依存型。生成ファイル名は型名なので、本ファイルへの移動で TS 側は変わらない。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// オンボーディング (初回起動ガイド) の現行バージョン。
/// `GeneralConfig::onboarding_version < ONBOARDING_CURRENT_VERSION` のとき UI が
/// ガイドを表示し、完了 / スキップ時に `update_config` patch でこの値を書き込む。
/// ステップ内容を大きく変えたら 1 上げる (既存ユーザーに再表示される)。
/// 「設定 → 一般 → もう一度見る」は 0 に戻す。フロント側 `useOnboarding.ts` の
/// `ONBOARDING_VERSION` と同値を保つこと (TS 側テストで固定)。
pub const ONBOARDING_CURRENT_VERSION: u32 = 1;

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

    /// 初回起動オンボーディングの完了バージョン (P10)。0 = 未表示 / 再表示要求。
    /// `ONBOARDING_CURRENT_VERSION` 未満なら UI がガイドを表示する。
    /// 旧 JSON には存在しないため `serde(default)` で 0 フォールバック
    /// (schema_version は上げない — v1→v2 の 6 フィールド追加と同じ透過方式)。
    /// 消費側: `layouts/default.vue` → `useOnboarding.evaluate()`。
    #[serde(default)]
    pub onboarding_version: u32,
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
            onboarding_version: 0,
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
