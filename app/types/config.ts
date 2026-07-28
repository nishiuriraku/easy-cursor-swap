/**
 * Rust 側の `AppConfig` (src-tauri/src/config.rs) と対応する型定義。
 *
 * Rust 構造体は `ts-rs` で `app/types/generated/` に自動生成される
 * (`cargo run --manifest-path src-tauri/Cargo.toml --features typegen --bin gen_types`)。
 * このファイルは **stable な再エクスポート面 (shim)** として機能し、
 * 生成物のファイル名/パスが変わっても既存 import
 * (`import type { AppConfig } from '~/types/config'`) を壊さない。
 *
 * ここで手書きするのは `submit_crash_reports` (Tauri command) のレスポンス型など、
 * Rust 側 serde derive の対象外のものに限る。
 */

export type {
  AppConfig,
  GeneralConfig,
  LoggingConfig,
  SecurityConfig,
  ThemeUsage,
} from './generated'

/**
 * `submit_crash_reports` (Tauri command) の戻り値。
 * 起動時の自動送信と UI ボタンから呼び出される。
 *
 * Rust 側に対応する公開 struct は無く、`commands::crash::submit_crash_reports`
 * が `Vec<ReportItem>` を返す先を集計した UI 専用レスポンス。
 */
export interface CrashSubmitSummary {
  /** 送信成功 → ローカル削除した件数 */
  sent: number
  /** 送信試行したが失敗した件数 (HTTP エラー / ネットワークエラー) */
  failed: number
  /** 件数上限などで今回送らなかった件数 (次回再試行) */
  skipped: number
}
