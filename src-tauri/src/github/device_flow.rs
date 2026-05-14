//! GitHub OAuth Device Flow 実装 (Public client, scope: `public_repo`)。
//!
//! [GitHub 公式仕様](https://docs.github.com/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow)
//!
//! 単一の HTTP 呼び出しのみを行う。
//! ポーリング間隔・期限切れ判定は呼び出し側 (`commands/marketplace_submit.rs`) に任せ、
//! ここはステートレスに `start` と `poll` を 1 回ずつ提供する。

use crate::errors::{AppError, AppResult};
use crate::github::types::{AccessTokenResponse, DeviceCodeResponse};
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://github.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// 1 回ポーリングしたときの分岐。
#[derive(Debug)]
pub enum PollOutcome {
    /// 認可待ち。呼び出し側は次の `interval` 後に再ポーリングする。
    Pending,
    /// レートリミット警告。呼び出し側は `interval` を 5 秒延長する。
    SlowDown,
    /// `device_code` の有効期限切れ。Device Flow をやり直す必要がある。
    Expired,
    /// ユーザーが GitHub 側で認可を拒否した。
    Denied,
    /// 認可成功。`access_token` を keystore に保存して以降利用する。
    Ready { access_token: String, scope: String },
}

/// GitHub OAuth Device Flow の HTTP 呼び出しユーティリティ。
///
/// インスタンスを持たないステートレス構造体。
/// 実際の GitHub エンドポイントへの呼び出しは `start` / `poll` を使い、
/// テスト用にベース URL を差し込む場合は `start_at` / `poll_at` を使う。
pub struct DeviceFlow;

impl DeviceFlow {
    /// デバイスコードを GitHub に要求する。
    ///
    /// `https://github.com` に対して `POST /login/device/code` を送信する。
    pub async fn start(client_id: &str) -> AppResult<DeviceCodeResponse> {
        Self::start_at(DEFAULT_BASE_URL, client_id).await
    }

    /// デバイスコードを指定ベース URL に要求する (テスト用)。
    pub async fn start_at(base_url: &str, client_id: &str) -> AppResult<DeviceCodeResponse> {
        let url = format!("{}/login/device/code", base_url.trim_end_matches('/'));
        let client = http_client()?;
        let body: DeviceCodeResponse = client
            .post(&url)
            .header("Accept", "application/json")
            .form(&[("client_id", client_id), ("scope", "public_repo")])
            .send()
            .await
            .map_err(|e| AppError::Theme(format!("device flow start 失敗: {}", e)))?
            .error_for_status()
            .map_err(|e| AppError::Theme(format!("device flow start HTTP エラー: {}", e)))?
            .json()
            .await
            .map_err(|e| AppError::Theme(format!("device flow start JSON パース失敗: {}", e)))?;
        Ok(body)
    }

    /// アクセストークンをポーリングする (1 回のみ)。
    ///
    /// `https://github.com` に対して `POST /login/oauth/access_token` を送信する。
    pub async fn poll(client_id: &str, device_code: &str) -> AppResult<PollOutcome> {
        Self::poll_at(DEFAULT_BASE_URL, client_id, device_code).await
    }

    /// アクセストークンを指定ベース URL にポーリングする (テスト用)。
    pub async fn poll_at(
        base_url: &str,
        client_id: &str,
        device_code: &str,
    ) -> AppResult<PollOutcome> {
        let url = format!(
            "{}/login/oauth/access_token",
            base_url.trim_end_matches('/')
        );
        let client = http_client()?;
        let body: AccessTokenResponse = client
            .post(&url)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", client_id),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|e| AppError::Theme(format!("device flow poll 失敗: {}", e)))?
            .error_for_status()
            .map_err(|e| AppError::Theme(format!("device flow poll HTTP エラー: {}", e)))?
            .json()
            .await
            .map_err(|e| AppError::Theme(format!("device flow poll JSON パース失敗: {}", e)))?;
        Ok(match body {
            AccessTokenResponse::Success {
                access_token,
                scope,
                ..
            } => PollOutcome::Ready {
                access_token,
                scope,
            },
            AccessTokenResponse::Pending { error, .. } => match error.as_str() {
                "authorization_pending" => PollOutcome::Pending,
                "slow_down" => PollOutcome::SlowDown,
                "expired_token" => PollOutcome::Expired,
                "access_denied" => PollOutcome::Denied,
                other => {
                    return Err(AppError::Theme(format!(
                        "device flow 未知エラーコード: {}",
                        other
                    )))
                }
            },
        })
    }
}

/// タイムアウト付き reqwest クライアントを構築する。
fn http_client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(concat!("EasyCursorSwap/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::Theme(format!("HTTP クライアント初期化失敗: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn start_returns_user_code_from_github() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/login/device/code")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"device_code":"DC","user_code":"WDJB-MJHT","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#,
            )
            .create_async()
            .await;

        let dc = DeviceFlow::start_at(&server.url(), "client-xyz")
            .await
            .expect("start should succeed");
        assert_eq!(dc.user_code, "WDJB-MJHT");
        assert_eq!(dc.interval, 5);
        assert_eq!(dc.expires_in, 900);
    }

    #[tokio::test]
    async fn poll_returns_pending_when_authorization_pending() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/login/oauth/access_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":"authorization_pending","error_description":"..."}"#)
            .create_async()
            .await;

        let r = DeviceFlow::poll_at(&server.url(), "client-xyz", "DC")
            .await
            .unwrap();
        assert!(matches!(r, PollOutcome::Pending));
    }

    #[tokio::test]
    async fn poll_returns_slow_down() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/login/oauth/access_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":"slow_down"}"#)
            .create_async()
            .await;

        let r = DeviceFlow::poll_at(&server.url(), "client-xyz", "DC")
            .await
            .unwrap();
        assert!(matches!(r, PollOutcome::SlowDown));
    }

    #[tokio::test]
    async fn poll_returns_expired() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/login/oauth/access_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":"expired_token"}"#)
            .create_async()
            .await;

        let r = DeviceFlow::poll_at(&server.url(), "client-xyz", "DC")
            .await
            .unwrap();
        assert!(matches!(r, PollOutcome::Expired));
    }

    #[tokio::test]
    async fn poll_returns_denied() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/login/oauth/access_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":"access_denied"}"#)
            .create_async()
            .await;

        let r = DeviceFlow::poll_at(&server.url(), "client-xyz", "DC")
            .await
            .unwrap();
        assert!(matches!(r, PollOutcome::Denied));
    }

    #[tokio::test]
    async fn poll_returns_ready_with_token() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/login/oauth/access_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"access_token":"ghu_TOKEN","token_type":"bearer","scope":"public_repo"}"#,
            )
            .create_async()
            .await;

        let r = DeviceFlow::poll_at(&server.url(), "client-xyz", "DC")
            .await
            .unwrap();
        match r {
            PollOutcome::Ready {
                access_token,
                scope,
            } => {
                assert_eq!(access_token, "ghu_TOKEN");
                assert_eq!(scope, "public_repo");
            }
            other => panic!("expected Ready, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn poll_returns_error_for_unknown_error_code() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/login/oauth/access_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":"some_future_code"}"#)
            .create_async()
            .await;

        let r = DeviceFlow::poll_at(&server.url(), "client-xyz", "DC").await;
        assert!(r.is_err());
    }
}
