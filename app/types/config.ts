/**
 * Rust 側の `AppConfig` (src-tauri/src/config.rs) と対応する型定義。
 * snake_case フィールド名はそのまま (Tauri の serde 既定)。
 */

export interface GeneralConfig {
  auto_start: boolean
  auto_update: boolean
  language: string
  active_theme_id: string | null
  panic_hotkey: string
  /**
   * クラッシュレポート送信オプトイン (デフォルト false)。
   *
   * 送信先 URL / App Token はビルド時に環境変数
   * `EASY_CURSOR_SWAP_CRASH_REPORT_ENDPOINT` / `_APP_TOKEN` で埋め込まれる。
   * env 未設定でビルドされたアプリでは本フラグが true でも送信は行われない。
   */
  crash_reporting: boolean
  /** お気に入りテーマ ID (UUID 文字列) リスト。 */
  favorites?: string[]
  /** テーマ ID → 利用統計 の辞書。 */
  usage?: Record<string, ThemeUsage>

  // ===== Wave 1B: 6 つの UI 設定フィールド (Rust V2 と同期) =====

  /**
   * テーマ適用時のトースト通知を表示するか (Wave 1A で追加、default true)。
   *
   * false のとき apply 成功/失敗トーストを抑制。
   * Rust: `crate::config::GeneralConfig::show_apply_toast`
   */
  show_apply_toast?: boolean

  /**
   * カーソル影の ON/OFF 制御をアプリが行うか (Wave 1A で追加、default true)。
   *
   * false のとき `SPI_SETCURSORSHADOW` の制御をスキップ(= Windows 既定挙動)。
   * Rust: `crate::config::GeneralConfig::apply_shadow_control`
   */
  apply_shadow_control?: boolean

  /**
   * `--autostart` 起動時にウィンドウを最小化状態で起動するか (Wave 1A で追加、
   * default false)。手動起動は常にウィンドウ表示。
   * Rust: `crate::config::GeneralConfig::start_minimized`
   */
  start_minimized?: boolean

  /**
   * ストレージ使用量が閾値超過したときの警告トーストを表示するか
   * (Wave 1A で追加、default true)。
   * Rust: `crate::config::GeneralConfig::show_storage_warning`
   */
  show_storage_warning?: boolean
}

/**
 * テーマ利用統計 (Rust 側 `crate::config::ThemeUsage` と対応)。
 * 適用回数と最終適用日時 (RFC3339) を持つ。
 */
export interface ThemeUsage {
  apply_count: number
  last_applied_at: string | null
}

/**
 * `submit_crash_reports` (Tauri command) の戻り値。
 * 起動時の自動送信と UI ボタンから呼び出される。
 */
export interface CrashSubmitSummary {
  /** 送信成功 → ローカル削除した件数 */
  sent: number
  /** 送信試行したが失敗した件数 (HTTP エラー / ネットワークエラー) */
  failed: number
  /** 件数上限などで今回送らなかった件数 (次回再試行) */
  skipped: number
}

export interface SecurityConfig {
  max_pack_compressed_size: number
  max_pack_uncompressed_size: number
  max_image_file_size: number
  storage_warning_threshold: number

  // ===== Wave 1B: 署名設定フィールド (Rust V2 と同期) =====

  /**
   * 未署名 .cursorpack のインポートを Rust 境界で拒否するか
   * (Wave 1A で追加、default false)。
   *
   * true のときローカル `.cursorpack` import は theme.json の `signature` が
   * Some で marketplace の `verify_signature` 経路を通ったもののみ許可。
   * marketplace install は既存 Ed25519 検証があるため追加対応なし。
   * Rust: `crate::config::SecurityConfig::require_signed_themes`
   */
  require_signed_themes?: boolean

  /**
   * 未署名 .cursorpack インポート時に確認ダイアログを出すか
   * (Wave 1A で追加、default true)。
   *
   * require_signed_themes=false のときのみ意味を持つ。
   * Rust: `crate::config::SecurityConfig::warn_unsigned_import`
   */
  warn_unsigned_import?: boolean
}

export interface LoggingConfig {
  level: string
  retention_days: number
  max_total_size: number
}

import type { GithubAccount } from './githubAuth'

export interface AppConfig {
  schema_version: number
  general: GeneralConfig
  security: SecurityConfig
  logging: LoggingConfig
  /** GitHub 連携アカウント情報。未連携時は null / undefined。Rust: AppConfig.github_account (Option<GithubAccount>) */
  github_account?: GithubAccount | null
}
