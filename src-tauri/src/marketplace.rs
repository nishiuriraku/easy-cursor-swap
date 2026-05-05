//! EasyCursorSwap 公式インデックス (Marketplace) クライアント
//!
//! GitHub 上の `easycursorswap/index` リポジトリから公開されているメタデータ
//! インデックス (`index.json`) を取得し、Ed25519 署名検証 + SHA-256 整合性
//! チェックを経てテーマをローカルにインストールするロジックを提供する。
//!
//! ## セキュリティ層
//!
//! 1. HTTPS + rustls (システム TLS スタックに依存しない)
//! 2. SHA-256 整合性チェック (ZIP バイト列)
//! 3. Ed25519 署名検証 (ZIP の SHA-256 → 著者公開鍵で署名)
//! 4. ZIP 展開時の Path traversal / シンボリックリンク / 累積サイズ防御
//!    (ThemeManager のインポート経路と同じ防御を再利用予定)
//!
//! ## Phase 9 で残るタスク
//! - `~/.custom_cursors/<UUID>/` への展開
//! - 公開鍵の `authors/{github_username}.json` 解決
//! - ETag / If-None-Match によるキャッシュ
//! - Rate-limit 対策 (User-Agent ヘッダー、再試行)

use crate::errors::{AppError, AppResult};
use base64::Engine as _;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

/// 公式インデックス JSON の URL。
pub const INDEX_URL: &str =
    "https://raw.githubusercontent.com/easycursorswap/index/main/index.json";

/// 公開鍵レジストリ (`authors/{github}.json`) のベース URL。
pub const PUBKEY_BASE_URL: &str =
    "https://raw.githubusercontent.com/easycursorswap/index/main/authors";

/// HTTP リクエストのタイムアウト。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// `.cursorpack` ダウンロードサイズの上限 (50 MB)。
/// `config.json` のセキュリティ閾値と同期させる予定。
const MAX_DOWNLOAD_BYTES: u64 = 50 * 1024 * 1024;

/// `index.json` のスキーマ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceIndex {
    pub schema_version: u32,
    pub commit: Option<String>,
    pub entries: Vec<MarketplaceEntry>,
}

/// 個別テーマのメタデータ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    pub id: uuid::Uuid,
    pub name: String,
    pub author: String,
    #[serde(rename = "author_github")]
    pub author_github: String,
    #[serde(rename = "author_pubkey_id")]
    pub author_pubkey_id: String,
    pub sha256: String,
    pub signature: String,
    #[serde(rename = "download_url")]
    pub download_url: String,
    pub version: String,
    #[serde(rename = "included_roles", default)]
    pub included_roles: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub homepage: Option<String>,
    #[serde(rename = "download_count", default)]
    pub download_count: u64,
}

/// 公開鍵レジストリ (`authors/{github}.json`) のスキーマ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorRecord {
    pub github_username: String,
    /// 現行公開鍵 (Base64)
    pub public_key: String,
    /// 過去鍵 (`key_id` → 公開鍵 Base64)。ローテーション時の旧署名検証用。
    #[serde(default)]
    pub historical_keys: std::collections::HashMap<String, String>,
}

/// インストール時のリクエスト (フロントエンドから渡される)。
/// JS 側は camelCase、Rust 側は snake_case で扱う。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceInstallRequest {
    pub download_url: String,
    pub sha256: String,
    pub signature: String,
    pub author_github: String,
    pub author_pubkey_id: String,
}

pub struct MarketplaceClient;

impl MarketplaceClient {
    /// 共有 HTTP クライアントを構築する。
    fn http() -> AppResult<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!("EasyCursorSwap/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| AppError::Theme(format!("HTTP クライアント初期化失敗: {}", e)))
    }

    /// 公式インデックスを取得する。
    pub async fn fetch_index() -> AppResult<MarketplaceIndex> {
        let client = Self::http()?;
        let body = client
            .get(INDEX_URL)
            .send()
            .await
            .map_err(|e| AppError::Theme(format!("インデックス取得失敗: {}", e)))?
            .error_for_status()
            .map_err(|e| AppError::Theme(format!("インデックス HTTP エラー: {}", e)))?
            .text()
            .await
            .map_err(|e| AppError::Theme(format!("レスポンス読み取り失敗: {}", e)))?;

        let index: MarketplaceIndex = serde_json::from_str(&body)?;
        Ok(index)
    }

    /// 著者の公開鍵レコードを取得する。
    pub async fn fetch_author_record(github_username: &str) -> AppResult<AuthorRecord> {
        let url = format!("{}/{}.json", PUBKEY_BASE_URL, github_username);
        let client = Self::http()?;
        let body = client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Theme(format!("著者鍵取得失敗: {}", e)))?
            .error_for_status()
            .map_err(|e| {
                AppError::Theme(format!(
                    "著者 {} の公開鍵が公式インデックスに存在しません: {}",
                    github_username, e
                ))
            })?
            .text()
            .await
            .map_err(|e| AppError::Theme(format!("公開鍵レスポンス読み取り失敗: {}", e)))?;

        let record: AuthorRecord = serde_json::from_str(&body)?;
        Ok(record)
    }

    /// 指定エントリをダウンロードして検証 + 展開する。
    ///
    /// フロー:
    ///  1. 著者公開鍵レコード取得 + key_id 一致確認 (ローテーション対応)
    ///  2. `.cursorpack` ダウンロード (サイズ上限つき)
    ///  3. SHA-256 整合性チェック
    ///  4. Ed25519 署名検証 (SHA-256 16進文字列を署名対象)
    ///  5. `ThemeManager::import_cursorpack_bytes` で展開
    ///     (Path traversal / Zip 爆弾 / シンボリックリンク防御を再利用)
    pub async fn install(req: MarketplaceInstallRequest) -> AppResult<uuid::Uuid> {
        // 1. 著者の公開鍵レコードを取得
        let author = Self::fetch_author_record(&req.author_github).await?;

        // 2. `key_id` 一致確認 (現行 or 過去鍵)
        let pubkey_b64 = if compute_key_id(&author.public_key)? == req.author_pubkey_id {
            &author.public_key
        } else if let Some(historical) = author.historical_keys.get(&req.author_pubkey_id) {
            historical
        } else {
            return Err(AppError::Theme(format!(
                "key_id {} が著者 {} の登録鍵と一致しません",
                req.author_pubkey_id, req.author_github
            )));
        };

        let verifying_key = decode_verifying_key(pubkey_b64)?;

        // 3. ZIP をダウンロード (サイズ上限つき)
        let bytes = Self::download_with_limit(&req.download_url, MAX_DOWNLOAD_BYTES).await?;

        // 4. SHA-256 整合性チェック
        let actual_sha256 = hex::encode(Sha256::digest(&bytes));
        if actual_sha256 != req.sha256.to_lowercase() {
            return Err(AppError::Theme(format!(
                "SHA-256 が一致しません (expected={} actual={})",
                req.sha256, actual_sha256
            )));
        }

        // 5. Ed25519 署名検証 (ZIP の SHA-256 を署名対象とする)
        let signature = decode_signature(&req.signature)?;
        verifying_key
            .verify(actual_sha256.as_bytes(), &signature)
            .map_err(|e| AppError::Theme(format!("Ed25519 署名検証に失敗: {}", e)))?;

        // url は短縮ハッシュにして、フィッシング先の追跡経路を直接残さない。
        // sha256 は前段で本物と確認済みなので 16 文字短縮版を残す。
        tracing::info!(
            "marketplace install verified: url_hash={} sha256_short={} key_id={}",
            crate::logging::short_hash(req.download_url.as_bytes()),
            &actual_sha256[..16],
            req.author_pubkey_id,
        );

        // 6. ThemeManager に展開を委譲 (Path traversal / Zip 爆弾 / Symlink 防御を共有)
        let theme_id = crate::theme::ThemeManager::import_cursorpack_bytes(&bytes)?;

        tracing::info!("marketplace install completed: theme_id={}", theme_id);
        Ok(theme_id)
    }

    /// 上限サイズ付きでバイト列をダウンロードする (Zip 爆弾対策の第一歩)。
    async fn download_with_limit(url: &str, limit: u64) -> AppResult<Vec<u8>> {
        let client = Self::http()?;
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Theme(format!("ダウンロード失敗: {}", e)))?
            .error_for_status()
            .map_err(|e| AppError::Theme(format!("ダウンロード HTTP エラー: {}", e)))?;

        if let Some(len) = resp.content_length() {
            if len > limit {
                return Err(AppError::Theme(format!(
                    "ダウンロードサイズ {} bytes が上限 {} bytes を超えています",
                    len, limit
                )));
            }
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::Theme(format!("ボディ読み取り失敗: {}", e)))?;

        if bytes.len() as u64 > limit {
            return Err(AppError::Theme(format!(
                "受信バイト数 {} が上限 {} を超えています",
                bytes.len(),
                limit
            )));
        }

        Ok(bytes.to_vec())
    }
}

/// Base64 公開鍵から `key_id` (公開鍵 SHA-256 の先頭 16 文字) を計算する。
pub fn compute_key_id(pubkey_b64: &str) -> AppResult<String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(pubkey_b64)
        .map_err(|e| AppError::Theme(format!("公開鍵 Base64 デコード失敗: {}", e)))?;
    Ok(hex::encode(Sha256::digest(&raw))[..16].to_string())
}

fn decode_verifying_key(pubkey_b64: &str) -> AppResult<VerifyingKey> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(pubkey_b64)
        .map_err(|e| AppError::Theme(format!("公開鍵 Base64 デコード失敗: {}", e)))?;
    let bytes: [u8; 32] = raw.as_slice().try_into().map_err(|_| {
        AppError::Theme(format!("公開鍵長が不正: {} bytes (32 必要)", raw.len()))
    })?;
    VerifyingKey::from_bytes(&bytes)
        .map_err(|e| AppError::Theme(format!("公開鍵パース失敗: {}", e)))
}

fn decode_signature(sig_b64: &str) -> AppResult<Signature> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(sig_b64)
        .map_err(|e| AppError::Theme(format!("署名 Base64 デコード失敗: {}", e)))?;
    let bytes: [u8; 64] = raw.as_slice().try_into().map_err(|_| {
        AppError::Theme(format!("署名長が不正: {} bytes (64 必要)", raw.len()))
    })?;
    Ok(Signature::from_bytes(&bytes))
}
