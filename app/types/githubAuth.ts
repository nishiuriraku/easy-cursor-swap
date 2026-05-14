/**
 * GitHub Device Flow / 自動提出関連の IPC ペイロード型。
 * Rust 側 (commands/marketplace_submit.rs / github/types.rs) と一致させること。
 */

/** `start_device_flow` IPC の戻り値。Rust: StartFlowResult #[serde(rename_all = "camelCase")] */
export interface StartFlowResult {
  userCode: string
  verificationUri: string
  expiresIn: number
  interval: number
}

/** `complete_device_flow` IPC の戻り値。Rust: CompleteFlowResult (#[serde(tag="status", rename_all="snake_case")]) */
export type CompleteFlowResult =
  | { status: 'pending' }
  | { status: 'slow_down' }
  | { status: 'expired' }
  | { status: 'denied' }
  | { status: 'ready'; login: string }

/** Rust 側 GithubAccount に対応。`get_config()` IPC で取得する AppConfig.github_account の型。
 *  Rust 側は snake_case でシリアライズされる (AppConfig は rename_all を持たない)。 */
export interface GithubAccount {
  login: string
  token_saved_at: string
}

/** `submit_theme_auto` IPC の戻り値。Rust: SubmitResult #[serde(rename_all = "camelCase")] */
export interface SubmitResult {
  prUrl: string
  prNumber: number
}

/** Tauri Event `submit:progress` の payload 値。Rust `emit_progress` の stage に対応。 */
export type SubmitStage =
  | 'build'
  | 'auth'
  | 'fork'
  | 'sync_fork'
  | 'branch'
  | 'upload_pack'
  | 'upload_entry'
  | 'open_pr'
