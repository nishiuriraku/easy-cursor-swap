//! GitHub API および Device Flow で扱う共通型。
//!
//! Task 5 (device_flow.rs) と Task 6 (client.rs) で実装する関数の入出力型を集約する。

use serde::{Deserialize, Serialize};

/// Device Flow 開始 (`POST /login/device/code`) のレスポンス。
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// Device Flow ポーリング (`POST /login/oauth/access_token`) のレスポンス。
///
/// 成功 (200 + `access_token`) と保留 (200 + `error`) を 1 つの enum に統合する。
/// `serde(untagged)` で `access_token` フィールドの有無によって分岐する。
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AccessTokenResponse {
    Success {
        access_token: String,
        token_type: String,
        scope: String,
    },
    Pending {
        error: String,
        #[serde(default)]
        error_description: Option<String>,
    },
}

/// 認証済みユーザー (`GET /user`) の最小レスポンス。
#[derive(Debug, Clone, Deserialize)]
pub struct AuthenticatedUser {
    pub login: String,
}

/// fork API (`POST /repos/{o}/{r}/forks`) レスポンスの抜粋。
#[derive(Debug, Clone, Deserialize)]
pub struct Repo {
    pub name: String,
    pub full_name: String,
    pub default_branch: String,
    pub owner: RepoOwner,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoOwner {
    pub login: String,
}

/// Pull Request 作成レスポンスの抜粋。
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub html_url: String,
    pub head: PrRef,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
}

/// IPC 戻り値: 自動提出結果。
/// フロント (`app/types/githubAuth.ts`) では camelCase を期待するため
/// `#[serde(rename_all = "camelCase")]` 必須。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitResult {
    pub pr_url: String,
    pub pr_number: u64,
}
