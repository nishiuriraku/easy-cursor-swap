//! GitHub REST API クライアント (Device Flow で取得した token で叩く)。
//!
//! base_url は本番では `https://api.github.com`、テストは `mockito::Server::url()` を注入する。
//! - fork: POST `/repos/{o}/{r}/forks` (既存があれば既存を返す扱い、GitHub 側 idempotent)
//! - branch: GET base ref の sha → GET target ref が 404 なら POST、200 なら PATCH (`force: true`)
//! - contents: GET 既存 sha → PUT base64 (新規 or 上書き)
//! - PR: GET list (`state=open&head=...`) → 既存 PATCH or 新規 POST

use crate::errors::{AppError, AppResult};
use crate::github::types::{AuthenticatedUser, PullRequest, Repo};
use base64::Engine as _;
use serde_json::json;
use std::time::Duration;

/// 本番 GitHub REST API のベース URL。
const DEFAULT_API_BASE: &str = "https://api.github.com";

/// HTTP リクエストのタイムアウト時間。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// GitHub REST API クライアント。
///
/// `token` は Device Flow で取得した OAuth アクセストークン。
/// `base_url` はテスト時に mockito のサーバー URL へ差し替えられる。
pub struct Client {
    /// API のベース URL (末尾スラッシュなし)。
    base_url: String,
    /// Bearer 認証トークン。
    token: String,
    /// 共有 HTTP クライアント (タイムアウト・UA 設定済み)。
    http: reqwest::Client,
}

impl Client {
    /// 本番 GitHub API (`https://api.github.com`) に接続するクライアントを生成する。
    pub fn new(token: String) -> Self {
        Self::new_at(DEFAULT_API_BASE, token)
    }

    /// 任意のベース URL に接続するクライアントを生成する (テスト用)。
    ///
    /// # Panics
    /// `reqwest::Client` の構築に失敗した場合 (通常起こりえない)。
    pub fn new_at(base_url: &str, token: String) -> Self {
        // reqwest::Client 構築は設定値が不正でなければ失敗しない。
        // ここで expect するのは「TLS スタックの初期化に根本的な問題がある場合」のみ。
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!("EasyCursorSwap/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest クライアントの構築に失敗 (TLS スタックの異常)");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            http,
        }
    }

    /// リクエストビルダーに認証ヘッダーを付与する。
    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    /// 認証済みユーザー情報を取得する (`GET /user`)。
    pub async fn get_authenticated_user(&self) -> AppResult<AuthenticatedUser> {
        let url = format!("{}/user", self.base_url);
        self.auth(self.http.get(&url))
            .send()
            .await
            .map_err(err_ctx("GET /user"))?
            .error_for_status()
            .map_err(err_ctx("GET /user status"))?
            .json()
            .await
            .map_err(err_ctx("GET /user json"))
    }

    /// upstream リポジトリを自アカウントへ fork する (`POST /repos/{o}/{r}/forks`)。
    ///
    /// GitHub API はべき等なので、既に fork 済みの場合も 202 を返す。
    pub async fn ensure_fork(&self, upstream_owner: &str, upstream_repo: &str) -> AppResult<Repo> {
        let url = format!(
            "{}/repos/{}/{}/forks",
            self.base_url, upstream_owner, upstream_repo
        );
        self.auth(self.http.post(&url))
            .send()
            .await
            .map_err(err_ctx("POST forks"))?
            .error_for_status()
            .map_err(err_ctx("POST forks status"))?
            .json()
            .await
            .map_err(err_ctx("POST forks json"))
    }

    /// fork を upstream の最新状態に同期する (`POST /repos/{o}/{r}/merge-upstream`)。
    pub async fn sync_fork_with_upstream(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> AppResult<()> {
        let url = format!("{}/repos/{}/{}/merge-upstream", self.base_url, owner, repo);
        self.auth(self.http.post(&url))
            .json(&json!({ "branch": branch }))
            .send()
            .await
            .map_err(err_ctx("merge-upstream"))?
            .error_for_status()
            .map_err(err_ctx("merge-upstream status"))?;
        Ok(())
    }

    /// ブランチを作成または base ブランチの HEAD へ強制リセットする。
    ///
    /// 1. `base_branch` の SHA を取得する。
    /// 2. `branch` の ref を GET する。
    ///    - 200: PATCH で `force: true` リセット
    ///    - 404: POST で新規作成
    ///    - その他: エラー
    pub async fn create_or_reset_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        base_branch: &str,
    ) -> AppResult<()> {
        // ベースブランチの SHA を取得する。
        let base_sha = self.get_branch_sha(owner, repo, base_branch).await?;

        let ref_url = format!(
            "{}/repos/{}/{}/git/refs/heads/{}",
            self.base_url, owner, repo, branch
        );
        let head_resp = self
            .auth(self.http.get(&ref_url))
            .send()
            .await
            .map_err(err_ctx("GET target ref"))?;

        if head_resp.status().is_success() {
            // 既存ブランチを force リセットする。
            self.auth(self.http.patch(&ref_url))
                .json(&json!({ "sha": base_sha, "force": true }))
                .send()
                .await
                .map_err(err_ctx("PATCH ref"))?
                .error_for_status()
                .map_err(err_ctx("PATCH ref status"))?;
        } else if head_resp.status().as_u16() == 404 {
            // 新規ブランチを作成する。
            let create_url = format!("{}/repos/{}/{}/git/refs", self.base_url, owner, repo);
            self.auth(self.http.post(&create_url))
                .json(&json!({
                    "ref": format!("refs/heads/{}", branch),
                    "sha": base_sha,
                }))
                .send()
                .await
                .map_err(err_ctx("POST ref"))?
                .error_for_status()
                .map_err(err_ctx("POST ref status"))?;
        } else {
            return Err(AppError::Theme(format!(
                "GET ref 予期せぬステータス: {}",
                head_resp.status()
            )));
        }
        Ok(())
    }

    /// 指定ブランチの HEAD SHA を取得する (内部ヘルパー)。
    async fn get_branch_sha(&self, owner: &str, repo: &str, branch: &str) -> AppResult<String> {
        let url = format!(
            "{}/repos/{}/{}/git/refs/heads/{}",
            self.base_url, owner, repo, branch
        );
        let v: serde_json::Value = self
            .auth(self.http.get(&url))
            .send()
            .await
            .map_err(err_ctx("GET base ref"))?
            .error_for_status()
            .map_err(err_ctx("GET base ref status"))?
            .json()
            .await
            .map_err(err_ctx("GET base ref json"))?;
        v["object"]["sha"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Theme("base ref に sha なし".to_string()))
    }

    /// ファイルを作成または更新する (`PUT /repos/{o}/{r}/contents/{path}`)。
    ///
    /// `bytes` を Base64 エンコードして PUT する。
    /// 既存ファイルの更新には現在の sha が必要なため、まず GET で確認する。
    pub async fn put_contents(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
        bytes: &[u8],
        message: &str,
    ) -> AppResult<()> {
        let url = format!(
            "{}/repos/{}/{}/contents/{}",
            self.base_url, owner, repo, path
        );

        // 既存ファイルの sha を取得する (なければ None)。
        let existing_sha = {
            let resp = self
                .auth(self.http.get(&url).query(&[("ref", branch)]))
                .send()
                .await
                .map_err(err_ctx("GET contents"))?;
            if resp.status().is_success() {
                let v: serde_json::Value =
                    resp.json().await.map_err(err_ctx("GET contents json"))?;
                v["sha"].as_str().map(|s| s.to_string())
            } else {
                None
            }
        };

        // bytes を Base64 エンコードする。
        let content_b64 = base64::engine::general_purpose::STANDARD.encode(bytes);

        // PUT ボディを組み立てる。既存ファイルがある場合は sha を含める。
        let mut body = serde_json::Map::new();
        body.insert("message".into(), json!(message));
        body.insert("content".into(), json!(content_b64));
        body.insert("branch".into(), json!(branch));
        if let Some(sha) = existing_sha {
            body.insert("sha".into(), json!(sha));
        }

        self.auth(self.http.put(&url))
            .json(&serde_json::Value::Object(body))
            .send()
            .await
            .map_err(err_ctx("PUT contents"))?
            .error_for_status()
            .map_err(err_ctx("PUT contents status"))?;
        Ok(())
    }

    /// Pull Request を作成または既存 PR を更新する。
    ///
    /// `state=open` かつ `head` が一致する PR を検索し、
    /// - 既存あり: PATCH でタイトル・本文を更新する。
    /// - 既存なし: POST で新規作成する。
    pub async fn open_or_update_pr(
        &self,
        upstream_owner: &str,
        upstream_repo: &str,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> AppResult<PullRequest> {
        let list_url = format!(
            "{}/repos/{}/{}/pulls",
            self.base_url, upstream_owner, upstream_repo
        );

        // 既存の open PR を検索する。
        let existing: Vec<PullRequest> = self
            .auth(self.http.get(&list_url))
            .query(&[("state", "open"), ("head", head)])
            .send()
            .await
            .map_err(err_ctx("GET pulls"))?
            .error_for_status()
            .map_err(err_ctx("GET pulls status"))?
            .json()
            .await
            .map_err(err_ctx("GET pulls json"))?;

        if let Some(pr) = existing.into_iter().next() {
            // 既存 PR を更新する。
            let patch_url = format!(
                "{}/repos/{}/{}/pulls/{}",
                self.base_url, upstream_owner, upstream_repo, pr.number
            );
            let updated: PullRequest = self
                .auth(self.http.patch(&patch_url))
                .json(&json!({ "title": title, "body": body }))
                .send()
                .await
                .map_err(err_ctx("PATCH pr"))?
                .error_for_status()
                .map_err(err_ctx("PATCH pr status"))?
                .json()
                .await
                .map_err(err_ctx("PATCH pr json"))?;
            return Ok(updated);
        }

        // 新規 PR を作成する。
        self.auth(self.http.post(&list_url))
            .json(&json!({
                "title": title,
                "body": body,
                "head": head,
                "base": base,
            }))
            .send()
            .await
            .map_err(err_ctx("POST pr"))?
            .error_for_status()
            .map_err(err_ctx("POST pr status"))?
            .json()
            .await
            .map_err(err_ctx("POST pr json"))
    }
}

/// reqwest エラーを `AppError::Theme` に変換するクロージャを生成する。
fn err_ctx(ctx: &'static str) -> impl Fn(reqwest::Error) -> AppError {
    move |e| AppError::Theme(format!("GitHub API {}: {}", ctx, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    /// テスト用クライアントを生成する (mockito サーバー URL を注入)。
    fn client_for(url: &str) -> Client {
        Client::new_at(url, "TOKEN".to_string())
    }

    #[tokio::test]
    async fn get_authenticated_user_returns_login() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/user")
            .match_header("authorization", "Bearer TOKEN")
            .with_status(200)
            .with_body(r#"{"login":"octocat"}"#)
            .create_async()
            .await;
        let c = client_for(&server.url());
        let u = c.get_authenticated_user().await.unwrap();
        assert_eq!(u.login, "octocat");
    }

    #[tokio::test]
    async fn ensure_fork_creates_when_missing() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/repos/upstream-owner/easy-cursor-swap-index/forks")
            .with_status(202)
            .with_body(r#"{"name":"easy-cursor-swap-index","full_name":"octocat/easy-cursor-swap-index","default_branch":"main","owner":{"login":"octocat"}}"#)
            .create_async()
            .await;
        let c = client_for(&server.url());
        let f = c
            .ensure_fork("upstream-owner", "easy-cursor-swap-index")
            .await
            .unwrap();
        assert_eq!(f.owner.login, "octocat");
        assert_eq!(f.default_branch, "main");
    }

    #[tokio::test]
    async fn sync_fork_calls_merge_upstream() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock(
                "POST",
                "/repos/octocat/easy-cursor-swap-index/merge-upstream",
            )
            .with_status(200)
            .with_body(r#"{"merged":true,"message":"ok"}"#)
            .create_async()
            .await;
        let c = client_for(&server.url());
        c.sync_fork_with_upstream("octocat", "easy-cursor-swap-index", "main")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_or_reset_branch_creates_when_missing() {
        let mut server = Server::new_async().await;
        // GET base ref → BASESHA
        let _m_base = server
            .mock(
                "GET",
                "/repos/octocat/easy-cursor-swap-index/git/refs/heads/main",
            )
            .with_status(200)
            .with_body(r#"{"object":{"sha":"BASESHA"}}"#)
            .create_async()
            .await;
        // GET target ref → 404
        let _m_missing = server
            .mock(
                "GET",
                "/repos/octocat/easy-cursor-swap-index/git/refs/heads/submit/abc",
            )
            .with_status(404)
            .with_body(r#"{"message":"Not Found"}"#)
            .create_async()
            .await;
        // POST new ref
        let _m_post = server
            .mock("POST", "/repos/octocat/easy-cursor-swap-index/git/refs")
            .with_status(201)
            .with_body(r#"{"ref":"refs/heads/submit/abc","object":{"sha":"BASESHA"}}"#)
            .create_async()
            .await;

        let c = client_for(&server.url());
        c.create_or_reset_branch("octocat", "easy-cursor-swap-index", "submit/abc", "main")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_or_reset_branch_force_resets_when_existing() {
        let mut server = Server::new_async().await;
        let _m_base = server
            .mock(
                "GET",
                "/repos/octocat/easy-cursor-swap-index/git/refs/heads/main",
            )
            .with_status(200)
            .with_body(r#"{"object":{"sha":"BASESHA"}}"#)
            .create_async()
            .await;
        let _m_head = server
            .mock(
                "GET",
                "/repos/octocat/easy-cursor-swap-index/git/refs/heads/submit/abc",
            )
            .with_status(200)
            .with_body(r#"{"object":{"sha":"OLDSHA"}}"#)
            .create_async()
            .await;
        let _m_patch = server
            .mock(
                "PATCH",
                "/repos/octocat/easy-cursor-swap-index/git/refs/heads/submit/abc",
            )
            .with_status(200)
            .with_body(r#"{"object":{"sha":"BASESHA"}}"#)
            .create_async()
            .await;
        let c = client_for(&server.url());
        c.create_or_reset_branch("octocat", "easy-cursor-swap-index", "submit/abc", "main")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn put_contents_base64_encodes_and_posts() {
        let mut server = Server::new_async().await;
        // GET existing → 404 (先行コンテンツなし)
        let _m_get = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/repos/octocat/easy-cursor-swap-index/contents/entries/abc\.json.*"
                        .to_string(),
                ),
            )
            .with_status(404)
            .create_async()
            .await;
        // PUT new
        let _m_put = server
            .mock(
                "PUT",
                "/repos/octocat/easy-cursor-swap-index/contents/entries/abc.json",
            )
            .with_status(201)
            .with_body(r#"{"content":{"sha":"NEWSHA"},"commit":{"sha":"COMMITSHA"}}"#)
            .create_async()
            .await;

        let c = client_for(&server.url());
        c.put_contents(
            "octocat",
            "easy-cursor-swap-index",
            "submit/abc",
            "entries/abc.json",
            b"{}",
            "feat: add",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn open_or_update_pr_creates_when_no_existing() {
        let mut server = Server::new_async().await;
        let _m_list = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/repos/upstream-owner/easy-cursor-swap-index/pulls.*".to_string(),
                ),
            )
            .with_status(200)
            .with_body("[]")
            .create_async()
            .await;
        let _m_post = server
            .mock(
                "POST",
                "/repos/upstream-owner/easy-cursor-swap-index/pulls",
            )
            .with_status(201)
            .with_body(r#"{"number":42,"html_url":"https://github.com/upstream-owner/easy-cursor-swap-index/pull/42","head":{"ref":"submit/abc"}}"#)
            .create_async()
            .await;

        let c = client_for(&server.url());
        let pr = c
            .open_or_update_pr(
                "upstream-owner",
                "easy-cursor-swap-index",
                "octocat:submit/abc",
                "main",
                "submit: x",
                "body",
            )
            .await
            .unwrap();
        assert_eq!(pr.number, 42);
        assert_eq!(
            pr.html_url,
            "https://github.com/upstream-owner/easy-cursor-swap-index/pull/42"
        );
    }

    #[tokio::test]
    async fn open_or_update_pr_patches_when_existing() {
        let mut server = Server::new_async().await;
        let _m_list = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/repos/upstream-owner/easy-cursor-swap-index/pulls.*".to_string(),
                ),
            )
            .with_status(200)
            .with_body(
                r#"[{"number":7,"html_url":"https://github.com/u/r/pull/7","head":{"ref":"submit/abc"}}]"#,
            )
            .create_async()
            .await;
        let _m_patch = server
            .mock(
                "PATCH",
                "/repos/upstream-owner/easy-cursor-swap-index/pulls/7",
            )
            .with_status(200)
            .with_body(
                r#"{"number":7,"html_url":"https://github.com/u/r/pull/7","head":{"ref":"submit/abc"}}"#,
            )
            .create_async()
            .await;
        let c = client_for(&server.url());
        let pr = c
            .open_or_update_pr(
                "upstream-owner",
                "easy-cursor-swap-index",
                "octocat:submit/abc",
                "main",
                "t",
                "b",
            )
            .await
            .unwrap();
        assert_eq!(pr.number, 7);
    }
}
