//! GitHub REST API クライアント (Marketplace 自動提出フロー用)。
//!
//! - [`device_flow`][]: OAuth Device Flow (Public client, scope: `public_repo`)
//! - [`client`][]: REST API ラッパー (fork / branch / contents / PR)
//! - [`types`][]: 共通レスポンス型
//!
//! `client_id` は build 時に環境変数 `GITHUB_OAUTH_CLIENT_ID` を `option_env!` で
//! 注入する。未設定 (CI / 開発時) は空文字列が返り、上位 IPC は AuthRequired 系の
//! エラーで失敗する。`client_secret` は Device Flow では発行されないため不要。

pub mod client;
pub mod device_flow;
pub mod types;

/// build 時に注入された GitHub OAuth App の Client ID。
/// 未設定の場合は空文字列を返す。
pub fn client_id() -> &'static str {
    option_env!("GITHUB_OAUTH_CLIENT_ID").unwrap_or("")
}

#[cfg(test)]
mod tests {
    #[test]
    fn client_id_returns_empty_string_or_value() {
        // CI/開発環境では未設定なので空文字列、ローカル開発で設定済みなら非空。
        // どちらでも panic しないことを確認。
        let id = super::client_id();
        // 文字列なので長さは常に取得できる。
        let _ = id.len();
    }
}
