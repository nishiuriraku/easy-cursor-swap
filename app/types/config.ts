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
