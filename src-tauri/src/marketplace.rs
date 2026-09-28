//! EasyCursorSwap 公式インデックス (Marketplace) クライアント
//!
//! GitHub 上の `nishiuriraku/easy-cursor-swap-index` リポジトリから公開されているメタデータ
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
    "https://raw.githubusercontent.com/nishiuriraku/easy-cursor-swap-index/main/index.json";

/// 公開鍵レジストリ (`authors/{github}.json`) のベース URL。
pub const PUBKEY_BASE_URL: &str =
    "https://raw.githubusercontent.com/nishiuriraku/easy-cursor-swap-index/main/authors";

/// HTTP リクエストのタイムアウト。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// ダウンロード元として許可する GitHub 系ホスト (F-27)。
/// `index.json` の `download_url` をここ以外のホストへ向けさせない。
const ALLOWED_DOWNLOAD_HOSTS: &[&str] = &[
    "github.com",
    "objects.githubusercontent.com",
    "raw.githubusercontent.com",
    "codeload.github.com",
];

/// `index.json` のスキーマ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(export))]
pub struct MarketplaceIndex {
    pub schema_version: u32,
    pub commit: Option<String>,
    pub entries: Vec<MarketplaceEntry>,
}

/// `marketplace_fetch_index` IPC の戻り値 (P11)。
///
/// - `stale = false`: いま HTTPS で取得し、ディスクキャッシュも更新済み。
/// - `stale = true`: ネットワーク取得に失敗し、前回成功時のキャッシュを返している。
///   `error` に失敗理由 (ユーザー向け文字列)、`fetched_at` にキャッシュ取得時刻 (RFC3339)。
///
/// キャッシュは表示用メタデータに過ぎず、インストール時の SHA-256 / Ed25519 / 許可ホスト
/// 検証 (`MarketplaceClient::install`) はキャッシュ由来でも同じ経路で行われる。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(export, rename_all = "camelCase"))]
pub struct MarketplaceIndexResult {
    pub index: MarketplaceIndex,
    pub stale: bool,
    pub fetched_at: Option<String>,
    pub error: Option<String>,
}

/// ディスク上のキャッシュファイル形式 (`~/.custom_cursors/_marketplace_index_cache.json`).
///
/// `raw` には取得時のレスポンスボディをそのまま保存する (`MarketplaceIndex` の
/// serialize は camelCase だが deserialize は snake_case のため、パース済み型を
/// 往復させると読めなくなる。生 JSON なら読み直しが確実)。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedIndex {
    /// 将来フォーマットを変えたときの判別用。現在 1。
    cache_version: u32,
    /// 取得成功時刻 (UTC, RFC3339)
    fetched_at: String,
    /// 取得時の index.json 生ボディ
    raw: String,
}

const INDEX_CACHE_VERSION: u32 = 1;
const INDEX_CACHE_FILE: &str = "_marketplace_index_cache.json";

/// 個別テーマのメタデータ。
///
/// `name` は後方互換のため `LocalizedString` で受ける。これにより既存の
/// `"name": "EasyCursorSwap Mint"` (plain string) と将来の
/// `"name": {"ja": "ミント", "en": "Mint", "default": "EasyCursorSwap Mint"}`
/// (ロケールマップ) の両方を 1 つの struct で deserialize できる。
///
/// **シリアライズ規約**:
/// - **deserialize** は snake_case (公開 `index.json` のスキーマに合わせる)
/// - **serialize** は camelCase (フロントへ IPC で渡すとき、TS 側 camelCase 型と
///   1:1 で揃う。フロント側で `adaptEntry()` のような snake→camel リネームを行わずに
///   そのまま `MarketplaceEntry` に流し込める)
///
/// この非対称 (audit E1 / E2) は serde の directional `rename_all` で表現する。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(export, rename_all = "camelCase"))]
pub struct MarketplaceEntry {
    pub id: uuid::Uuid,
    pub name: crate::theme::LocalizedString,
    pub author: String,
    pub author_github: String,
    pub author_pubkey_id: String,
    pub sha256: String,
    pub signature: String,
    pub download_url: String,
    pub version: String,
    #[serde(default)]
    pub included_roles: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub homepage: Option<String>,
    #[serde(default)]
    pub download_count: u64,
    /// Featured 表示用ラベル (例: "new", "popular")。公式 index.json が
    /// 将来このフィールドを含める可能性があるため `serde(default)` で受け取る。
    #[serde(default)]
    pub highlight: Option<String>,
    /// 公式インデックス側 `previews/<uuid>/` のベース URL。
    /// `MarketplaceDetailModal` がここに `/<role>.png` を結合して PNG を取得する。
    /// 旧スキーマとの互換のため `serde(default)` で `None` フォールバック。
    #[serde(default)]
    pub preview_base_url: Option<String>,
}

/// 公開鍵レジストリ (`authors/{github}.json`) のスキーマ。
///
/// 実サーバーは現在 `"github"` キーで author username を返すが、過去 `"github_username"`
/// だった経緯と将来の rename 余地を残すため `serde(alias)` で両対応する。`github_username`
/// 自体は install ロジックでは未使用 (作者識別は `MarketplaceInstallRequest.author_github`
/// で完結する) のため `serde(default)` で欠落も許容する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorRecord {
    /// 作者の GitHub username。server JSON のキーは `"github"`。
    /// 旧スキーマ (`"github_username"`) も alias で受け入れる。
    #[serde(default, alias = "github")]
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
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(export, rename_all = "camelCase"))]
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
        Self::fetch_index_from(INDEX_URL).await
    }

    /// キャッシュファイルのパス (`ConfigManager::cursors_dir()` 配下)。
    fn index_cache_path() -> AppResult<std::path::PathBuf> {
        Ok(crate::config::ConfigManager::cursors_dir()?.join(INDEX_CACHE_FILE))
    }

    /// 公式インデックスを取得し、成功時はキャッシュを更新、失敗時はキャッシュを stale で返す。
    pub async fn fetch_index_cached() -> AppResult<MarketplaceIndexResult> {
        let cache = Self::index_cache_path()?;
        Self::fetch_index_with_cache_from(INDEX_URL, &cache).await
    }

    /// テスト可能な本体。`url` は mockito、`cache_path` は tempdir を注入する。
    pub(crate) async fn fetch_index_with_cache_from(
        url: &str,
        cache_path: &std::path::Path,
    ) -> AppResult<MarketplaceIndexResult> {
        match Self::fetch_index_body(url).await {
            Ok(body) => {
                let mut index: MarketplaceIndex = serde_json::from_str(&body)?;
                for entry in &mut index.entries {
                    entry.homepage = Self::sanitize_homepage(entry.homepage.take());
                }
                let fetched_at = chrono::Utc::now().to_rfc3339();
                if let Err(e) = Self::write_index_cache(cache_path, &body, &fetched_at) {
                    // キャッシュ書込失敗は表示を止めない (次回は再取得すればよい)
                    tracing::warn!(
                        "marketplace index cache write failed ({}): {}",
                        crate::logging::redact_path(cache_path),
                        e
                    );
                }
                Ok(MarketplaceIndexResult {
                    index,
                    stale: false,
                    fetched_at: Some(fetched_at),
                    error: None,
                })
            }
            Err(net_err) => match Self::read_index_cache(cache_path) {
                Ok(Some(cached)) => {
                    let index = match Self::parse_cached_index(&cached) {
                        Ok(index) => index,
                        Err(cache_err) => {
                            tracing::warn!(
                                "marketplace index cache unreadable, ignoring: {}",
                                cache_err
                            );
                            return Err(net_err);
                        }
                    };
                    tracing::warn!(
                        "marketplace index fetch failed, serving cache from {}: {}",
                        cached.fetched_at,
                        net_err
                    );
                    Ok(MarketplaceIndexResult {
                        index,
                        stale: true,
                        fetched_at: Some(cached.fetched_at),
                        error: Some(net_err.to_string()),
                    })
                }
                Ok(None) => Err(net_err),
                Err(cache_err) => {
                    tracing::warn!(
                        "marketplace index cache unreadable, ignoring: {}",
                        cache_err
                    );
                    Err(net_err)
                }
            },
        }
    }

    fn write_index_cache(
        path: &std::path::Path,
        raw_body: &str,
        fetched_at: &str,
    ) -> AppResult<()> {
        let payload = CachedIndex {
            cache_version: INDEX_CACHE_VERSION,
            fetched_at: fetched_at.to_string(),
            raw: raw_body.to_string(),
        };
        let content = serde_json::to_string(&payload)?;
        crate::config::atomic_write(path, &content)
    }

    /// `Ok(None)` = ファイル無し。`Err` = 破損 / 版不一致 (呼び出し側は無視してネットエラーを返す)。
    fn read_index_cache(path: &std::path::Path) -> AppResult<Option<CachedIndex>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        let cached: CachedIndex = serde_json::from_str(&content)?;
        if cached.cache_version != INDEX_CACHE_VERSION {
            return Err(AppError::Theme(format!(
                "index cache version mismatch: {}",
                cached.cache_version
            )));
        }
        Ok(Some(cached))
    }

    /// キャッシュの生ボディをパースして返す (homepage 再健全化つき)。
    fn parse_cached_index(cached: &CachedIndex) -> AppResult<MarketplaceIndex> {
        let mut index: MarketplaceIndex = serde_json::from_str(&cached.raw)?;
        // 読み込み時も homepage を再健全化 (キャッシュ改変への防御線)
        for entry in &mut index.entries {
            entry.homepage = Self::sanitize_homepage(entry.homepage.take());
        }
        Ok(index)
    }

    /// 任意の URL からインデックスを取得する (テストでは mockito の URL を注入する)。
    ///
    /// `fetch_index` の本体実装をここに置き、本番経路は `INDEX_URL` を渡す薄い
    /// ラッパーにすることで、公開 API シグネチャを変えずに HTTP I/O を差し替え
    /// 可能にする。`pub(crate)` に閉じているので、フロントエンドに無用な公開
    /// 表面は増えない。
    pub(crate) async fn fetch_index_from(url: &str) -> AppResult<MarketplaceIndex> {
        let body = Self::fetch_index_body(url).await?;
        let mut index: MarketplaceIndex = serde_json::from_str(&body)?;
        // F-17: 各エントリの homepage を健全化してからフロントへ返す。
        for entry in &mut index.entries {
            entry.homepage = Self::sanitize_homepage(entry.homepage.take());
        }
        Ok(index)
    }

    /// HTTP GET の生ボディを返す (キャッシュ保存用にパース前文字列が必要な経路向け)。
    async fn fetch_index_body(url: &str) -> AppResult<String> {
        let client = Self::http()?;
        let body = client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Theme(format!("インデックス取得失敗: {}", e)))?
            .error_for_status()
            .map_err(|e| AppError::Theme(format!("インデックス HTTP エラー: {}", e)))?
            .text()
            .await
            .map_err(|e| AppError::Theme(format!("レスポンス読み取り失敗: {}", e)))?;
        Ok(body)
    }

    /// 著者の公開鍵レコードを取得する。
    pub async fn fetch_author_record(github_username: &str) -> AppResult<AuthorRecord> {
        // F-28: URL 連結前に username を検証 (パストラバーサル / 別パス参照防止)。
        Self::validate_github_username(github_username)?;
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
        // 0. download_url を検証 (F-27)。署名 / SHA-256 検証より前に GET するため、
        //    許可ホスト + https のみに絞ってフィッシング / 任意 URL 取得を防ぐ。
        Self::validate_download_url(&req.download_url)?;

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

        // 3. ZIP をダウンロード (サイズ上限つき。圧縮サイズ上限の SoT は config/schema.rs)
        use crate::config::DEFAULT_MAX_PACK_COMPRESSED_SIZE;
        let bytes =
            Self::download_with_limit(&req.download_url, DEFAULT_MAX_PACK_COMPRESSED_SIZE).await?;

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

        // マーケットプレース由来であることを theme.json に記録する。
        // repackage_theme IPC と UI が編集 / エクスポートをガードするための土台。
        let cursors_dir = crate::config::ConfigManager::cursors_dir()?;
        let theme_dir = cursors_dir.join(theme_id.to_string());
        crate::theme::set_metadata_source(
            &theme_dir,
            crate::theme::types::ThemeSource::Marketplace,
        )?;

        tracing::info!("marketplace install completed: theme_id={}", theme_id);
        Ok(theme_id)
    }

    /// プレビュー画像サイズの上限 (500 KB)。
    const MAX_PREVIEW_BYTES: u64 = 500 * 1024;

    /// homepage URL を健全化する (F-17 第一防御線)。
    ///
    /// `index.json` の `homepage` はフロントの外部リンクに直結する。`https://` で
    /// 始まらない (javascript: / http: 等) もの、`..` を含むものは `None` に落として
    /// フロントへ渡さない。フロント側の useExternalUrl → open_url IPC でも
    /// `is_allowed_url_scheme` が再検証するが、ここが最初の関門。
    pub fn sanitize_homepage(url: Option<String>) -> Option<String> {
        url.filter(|u| u.starts_with("https://") && !u.contains(".."))
    }

    /// preview_base_url が https:// で始まり ".." を含まないことを検証。
    pub fn validate_preview_url(url: &str) -> AppResult<()> {
        if !url.starts_with("https://") {
            return Err(AppError::Theme(
                "preview_base_url は https:// で始まる必要があります".to_string(),
            ));
        }
        if url.contains("..") {
            return Err(AppError::Theme(
                "preview_base_url に .. を含めることはできません".to_string(),
            ));
        }
        Ok(())
    }

    /// role 識別子が ASCII 英数字 + アンダースコアのみで構成されることを検証。
    /// パストラバーサル / 拡張子付与 / クエリ文字列を排除する。
    pub fn validate_role_name(role: &str) -> AppResult<()> {
        if role.is_empty() || role.len() > 32 {
            return Err(AppError::Theme("role 名が不正です".to_string()));
        }
        if !role.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(AppError::Theme(format!("role 名に不正な文字: {}", role)));
        }
        Ok(())
    }

    /// download_url が https:// かつ許可ホストであることを検証する (F-27)。
    ///
    /// `index.json` の `download_url` は署名 / SHA-256 検証の前に GET される。
    /// 許可リスト外ホストや非 https を弾くことで、悪意ある index でフィッシング
    /// サイトや任意 URL を叩かせる経路を塞ぐ。`url` crate を増やさず手書きで
    /// host 部だけ取り出す (Cargo.lock 不変)。
    pub fn validate_download_url(url: &str) -> AppResult<()> {
        let rest = url.strip_prefix("https://").ok_or_else(|| {
            AppError::Theme("download_url は https:// である必要があります".into())
        })?;
        // authority 部分は最初の `/`, `?`, `#` まで。さらに `:` でポートを切り落とす。
        let host = rest
            .split(['/', '?', '#'])
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("");
        let ok = ALLOWED_DOWNLOAD_HOSTS
            .iter()
            .any(|h| host == *h || host.ends_with(&format!(".{h}")));
        if !ok {
            return Err(AppError::Theme(format!(
                "許可されていないダウンロード元ホスト: {}",
                host
            )));
        }
        Ok(())
    }

    /// github_username が GitHub の命名規則に沿う安全な値かを検証する (F-28)。
    ///
    /// `fetch_author_record` で `PUBKEY_BASE_URL/{username}.json` に直接連結される
    /// ため、`../` や `/` 等を含む値はパストラバーサル / 別パス参照を招く。
    /// GitHub の規則: 1〜39 文字、英数字とハイフンのみ、先頭末尾ハイフン不可、
    /// 連続ハイフン不可。
    pub fn validate_github_username(name: &str) -> AppResult<()> {
        if name.is_empty() || name.len() > 39 {
            return Err(AppError::Theme(
                "github_username の長さが不正です".to_string(),
            ));
        }
        if name.starts_with('-') || name.ends_with('-') {
            return Err(AppError::Theme(
                "github_username の先頭・末尾にハイフンは使えません".to_string(),
            ));
        }
        if name.contains("--") {
            return Err(AppError::Theme(
                "github_username に連続ハイフンは使えません".to_string(),
            ));
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(AppError::Theme(format!(
                "github_username に不正な文字: {}",
                name
            )));
        }
        Ok(())
    }

    /// 公式インデックスから 1 ロール分のプレビュー PNG を取得する。
    /// MarketplaceDetailModal で 6 ロール並列に呼ばれる。
    pub async fn fetch_preview(preview_base_url: &str, role: &str) -> AppResult<Vec<u8>> {
        Self::validate_preview_url(preview_base_url)?;
        Self::validate_role_name(role)?;
        let url = format!("{}/{}.png", preview_base_url.trim_end_matches('/'), role);
        Self::download_with_limit(&url, Self::MAX_PREVIEW_BYTES).await
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
///
/// Wave 2B / Task 5: 旧 `marketplace::compute_key_id` 実装は `keystore::compute_key_id`
/// と逐語重複だったため正準を `keystore.rs` に統一し、このラッパで再エクスポート
/// する。`marketplace` 内では `keystore::compute_key_id` を直接呼び出す。
pub use crate::keystore::compute_key_id;

fn decode_verifying_key(pubkey_b64: &str) -> AppResult<VerifyingKey> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(pubkey_b64)
        .map_err(|e| AppError::Theme(format!("公開鍵 Base64 デコード失敗: {}", e)))?;
    let bytes: [u8; 32] = raw
        .as_slice()
        .try_into()
        .map_err(|_| AppError::Theme(format!("公開鍵長が不正: {} bytes (32 必要)", raw.len())))?;
    VerifyingKey::from_bytes(&bytes)
        .map_err(|e| AppError::Theme(format!("公開鍵パース失敗: {}", e)))
}

fn decode_signature(sig_b64: &str) -> AppResult<Signature> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(sig_b64)
        .map_err(|e| AppError::Theme(format!("署名 Base64 デコード失敗: {}", e)))?;
    let bytes: [u8; 64] = raw
        .as_slice()
        .try_into()
        .map_err(|_| AppError::Theme(format!("署名長が不正: {} bytes (64 必要)", raw.len())))?;
    Ok(Signature::from_bytes(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// 32 byte の固定パターン公開鍵生バイト列を Base64 で表現したもの。
    /// SHA-256 の値が固定なので key_id を直接アサートできる。
    fn fixed_pubkey_b64() -> String {
        // Repeated 0x42 = 66 byte = 'B' x 32 -> 一意で再現性のあるテストベクトル
        let raw = [0x42u8; 32];
        base64::engine::general_purpose::STANDARD.encode(raw)
    }

    #[test]
    fn compute_key_id_returns_16_hex_chars() {
        let id = compute_key_id(&fixed_pubkey_b64()).unwrap();
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn compute_key_id_is_deterministic() {
        // 同一公開鍵で何度呼んでも同じ key_id
        let pk = fixed_pubkey_b64();
        assert_eq!(compute_key_id(&pk).unwrap(), compute_key_id(&pk).unwrap());
    }

    #[test]
    fn compute_key_id_differs_for_different_pubkeys() {
        let a = base64::engine::general_purpose::STANDARD.encode([0x01u8; 32]);
        let b = base64::engine::general_purpose::STANDARD.encode([0x02u8; 32]);
        assert_ne!(compute_key_id(&a).unwrap(), compute_key_id(&b).unwrap());
    }

    #[test]
    fn compute_key_id_known_vector() {
        // SHA-256 of (0x42 repeated 32 times) =
        //   8a48f1ad7d99b8d6e2c4127ec97a99ce92efb46aa6e0c9c5ad8e83eb6e9f1f1d (例)
        // 実値は実際に計算して固定する。
        // [0x42; 32] -> SHA-256 hex prefix の最初の 16 文字を実値計算で確認:
        // hex(sha256([0x42; 32])) = "26ac9a3a36cdb6acdc24fa6f9d92ee7c..." (これは仮)
        // 実値は決定的なのでテスト失敗で初回のみ更新する。
        let id = compute_key_id(&fixed_pubkey_b64()).unwrap();
        // SHA-256 of [0x42 x 32]: 計算済み値
        let raw = [0x42u8; 32];
        let expected = hex::encode(Sha256::digest(raw))[..16].to_string();
        assert_eq!(id, expected);
    }

    #[test]
    fn compute_key_id_rejects_invalid_base64() {
        // `marketplace::compute_key_id` は Task 5a で `keystore::compute_key_id`
        // の re-export に統一済み。鍵ペア Base64 デコード失敗は暗号学的失敗
        // なので `AppError::Crypto` (= `crypto: <理由>`) に分類される (Task 10)。
        let err = compute_key_id("not-valid-base64-!!!").unwrap_err();
        assert!(matches!(err, AppError::Crypto(_)));
    }

    #[test]
    fn decode_verifying_key_rejects_wrong_length() {
        // 16 bytes (= short) → 32 必要なのでエラー
        let short_pk = base64::engine::general_purpose::STANDARD.encode([0u8; 16]);
        let err = decode_verifying_key(&short_pk).unwrap_err();
        assert!(matches!(err, AppError::Theme(_)));
    }

    #[test]
    fn decode_verifying_key_accepts_valid_keypair() {
        // 実 Ed25519 鍵ペアを生成 → Base64 → デコード往復
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let pk_bytes = signing.verifying_key().to_bytes();
        let pk_b64 = base64::engine::general_purpose::STANDARD.encode(pk_bytes);
        let decoded = decode_verifying_key(&pk_b64).unwrap();
        assert_eq!(decoded.to_bytes(), pk_bytes);
    }

    #[test]
    fn decode_signature_rejects_wrong_length() {
        // 32 bytes -> 64 必要なのでエラー
        let short_sig = base64::engine::general_purpose::STANDARD.encode([0u8; 32]);
        let err = decode_signature(&short_sig).unwrap_err();
        assert!(matches!(err, AppError::Theme(_)));
    }

    #[test]
    fn install_writes_marketplace_source_via_helper() {
        // marketplace::install の "set_metadata_source 呼出" 部分を契約として固定するテスト。
        // 実 install は HTTP / Ed25519 が要るので、ヘルパだけ単体で動くことを確認する。
        use crate::theme::types::ThemeSource;
        let temp = tempfile::TempDir::new().unwrap();
        let theme_dir = temp.path();
        std::fs::write(
            theme_dir.join("theme.json"),
            r#"{
                "schema_version":1,
                "id":"00000000-0000-0000-0000-000000000000",
                "name":"Imported",
                "version":"1.0.0",
                "created_at":"2026-05-14T00:00:00Z",
                "requires_os_shadow":false,
                "cursors":{}
            }"#,
        )
        .unwrap();

        // install() が最後にやる呼び出しと同じパラメータ
        crate::theme::set_metadata_source(theme_dir, ThemeSource::Marketplace).unwrap();

        let content = std::fs::read_to_string(theme_dir.join("theme.json")).unwrap();
        assert!(content.contains(r#""source""#));
        assert!(content.contains(r#""marketplace""#));
    }

    #[test]
    fn preview_url_rejects_non_https() {
        let r = MarketplaceClient::validate_preview_url("http://evil.com/preview");
        assert!(r.is_err(), "http:// は拒否されるべき");
    }

    #[test]
    fn preview_url_accepts_https() {
        let r = MarketplaceClient::validate_preview_url(
            "https://raw.githubusercontent.com/x/y/main/previews/abc",
        );
        assert!(r.is_ok());
    }

    #[test]
    fn preview_role_rejects_path_traversal() {
        assert!(MarketplaceClient::validate_role_name("../escape").is_err());
        assert!(MarketplaceClient::validate_role_name("Arrow/extra").is_err());
        assert!(MarketplaceClient::validate_role_name("Arrow.png").is_err());
    }

    #[test]
    fn preview_role_accepts_canonical_role_names() {
        for name in ["Arrow", "Help", "AppStarting", "Wait", "Crosshair", "IBeam"] {
            assert!(
                MarketplaceClient::validate_role_name(name).is_ok(),
                "{} should be valid",
                name
            );
        }
    }

    #[test]
    fn download_url_accepts_allowed_github_hosts() {
        // F-27: 許可された GitHub 系ホストの https URL は通る。
        assert!(MarketplaceClient::validate_download_url(
            "https://github.com/owner/repo/releases/download/v1/p.cursorpack"
        )
        .is_ok());
        assert!(MarketplaceClient::validate_download_url(
            "https://objects.githubusercontent.com/github-production-release-asset/abc"
        )
        .is_ok());
        // サブドメインも許可 (`.github.com` で終わるもの)。
        assert!(MarketplaceClient::validate_download_url(
            "https://codeload.github.com/owner/repo/zip"
        )
        .is_ok());
    }

    #[test]
    fn download_url_rejects_non_https_and_foreign_hosts() {
        // F-27: http / 別ホスト / 末尾偽装 / file スキーム / 空はすべて拒否。
        assert!(
            MarketplaceClient::validate_download_url("http://github.com/p.cursorpack").is_err()
        );
        assert!(MarketplaceClient::validate_download_url("https://evil.com/p.cursorpack").is_err());
        // "github.com.evil.com" は github.com で「終わらない」ので拒否されるべき。
        assert!(MarketplaceClient::validate_download_url(
            "https://github.com.evil.com/p.cursorpack"
        )
        .is_err());
        // 前方一致偽装 "github.com" + ".evil" -> ends_with(".github.com") に該当しない。
        assert!(
            MarketplaceClient::validate_download_url("https://notgithub.com/p.cursorpack").is_err()
        );
        assert!(MarketplaceClient::validate_download_url("file:///etc/passwd").is_err());
        assert!(MarketplaceClient::validate_download_url("").is_err());
    }

    #[test]
    fn github_username_accepts_valid_names() {
        // F-28: GitHub 規則に沿う username は通る。
        assert!(MarketplaceClient::validate_github_username("nishiuriraku").is_ok());
        assert!(MarketplaceClient::validate_github_username("a").is_ok());
        assert!(MarketplaceClient::validate_github_username("foo-bar").is_ok());
        // 39 文字ちょうどは上限内。
        assert!(MarketplaceClient::validate_github_username(&"a".repeat(39)).is_ok());
    }

    #[test]
    fn github_username_rejects_invalid_names() {
        // F-28: パストラバーサル / 不正文字 / 長さ違反 / ハイフン位置違反を拒否。
        assert!(MarketplaceClient::validate_github_username("").is_err());
        assert!(MarketplaceClient::validate_github_username(&"a".repeat(40)).is_err());
        assert!(MarketplaceClient::validate_github_username("../escape").is_err());
        assert!(MarketplaceClient::validate_github_username("foo/bar").is_err());
        assert!(MarketplaceClient::validate_github_username("foo.bar").is_err());
        assert!(MarketplaceClient::validate_github_username("-lead").is_err());
        assert!(MarketplaceClient::validate_github_username("trail-").is_err());
        assert!(MarketplaceClient::validate_github_username("日本語").is_err());
    }

    #[test]
    fn sanitize_homepage_keeps_https_and_drops_unsafe() {
        // F-17: https のみ通し、http / javascript / .. / None は None に落とす。
        assert_eq!(
            MarketplaceClient::sanitize_homepage(Some("https://example.com".to_string())),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            MarketplaceClient::sanitize_homepage(Some("http://x".to_string())),
            None
        );
        assert_eq!(
            MarketplaceClient::sanitize_homepage(Some("javascript:alert(1)".to_string())),
            None
        );
        assert_eq!(
            MarketplaceClient::sanitize_homepage(Some("https://x/../y".to_string())),
            None
        );
        assert_eq!(MarketplaceClient::sanitize_homepage(None), None);
    }

    #[test]
    fn signature_roundtrip_verifies_with_correct_message() {
        // 「鍵ペア生成 → メッセージに署名 → 公開鍵で検証」の往復テスト。
        // marketplace install と同じ署名フォーマット (バイト列) でうまく動くことを確認。
        let signing = SigningKey::from_bytes(&[42u8; 32]);
        let message = b"sha256-of-zip-payload";
        let sig = signing.sign(message);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let pk_b64 =
            base64::engine::general_purpose::STANDARD.encode(signing.verifying_key().to_bytes());
        let vkey = decode_verifying_key(&pk_b64).unwrap();
        let decoded_sig = decode_signature(&sig_b64).unwrap();

        // 正しいメッセージで検証 → OK
        assert!(vkey.verify(message, &decoded_sig).is_ok());
        // 改竄されたメッセージ → 失敗
        assert!(vkey
            .verify(b"sha256-of-tampered-payload", &decoded_sig)
            .is_err());
    }

    #[test]
    fn entry_deserializes_preview_base_url() {
        // 実 index.json で配信される形 (preview_base_url を含む) が正しく struct に入ること。
        let json = r#"{
            "id": "6d364941-c605-4def-801a-14ebb401936f",
            "name": "Mint",
            "author": "x",
            "author_github": "x",
            "author_pubkey_id": "abcd",
            "sha256": "00",
            "signature": "AA==",
            "download_url": "https://example.com/pack",
            "version": "1.0.0",
            "preview_base_url": "https://raw.githubusercontent.com/x/y/main/previews/6d364941"
        }"#;
        let entry: MarketplaceEntry = serde_json::from_str(json).unwrap();
        assert_eq!(
            entry.preview_base_url.as_deref(),
            Some("https://raw.githubusercontent.com/x/y/main/previews/6d364941")
        );
    }

    #[test]
    fn entry_deserializes_localized_name_object() {
        // 新スキーマ: name がロケールマップで来た場合。LocalizedString::Localized で受ける。
        // フロント側はこの形式が来たら useI18n().locale に応じて表示を切り替える。
        let json = r#"{
            "id": "6d364941-c605-4def-801a-14ebb401936f",
            "name": {"ja": "ミント", "en": "Mint", "default": "EasyCursorSwap Mint"},
            "author": "x",
            "author_github": "x",
            "author_pubkey_id": "abcd",
            "sha256": "00",
            "signature": "AA==",
            "download_url": "https://example.com/pack",
            "version": "1.0.0"
        }"#;
        let entry: MarketplaceEntry = serde_json::from_str(json).unwrap();
        // LocalizedString::get で fallback chain (locale → default → en → first) が効くこと
        assert_eq!(entry.name.get("ja"), "ミント");
        assert_eq!(entry.name.get("en"), "Mint");
        // 未知ロケールは default にフォールバック
        assert_eq!(entry.name.get("zh"), "EasyCursorSwap Mint");
    }

    #[test]
    fn entry_deserializes_plain_string_name_for_backward_compat() {
        // 既存スキーマ: name が文字列のままの場合。LocalizedString::Simple で受け、
        // どのロケールを問い合わせても同じ値を返す (既存挙動を維持)。
        let json = r#"{
            "id": "6d364941-c605-4def-801a-14ebb401936f",
            "name": "EasyCursorSwap Mint",
            "author": "x",
            "author_github": "x",
            "author_pubkey_id": "abcd",
            "sha256": "00",
            "signature": "AA==",
            "download_url": "https://example.com/pack",
            "version": "1.0.0"
        }"#;
        let entry: MarketplaceEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.name.get("ja"), "EasyCursorSwap Mint");
        assert_eq!(entry.name.get("en"), "EasyCursorSwap Mint");
    }

    #[test]
    fn entry_omits_preview_base_url_when_none() {
        // 旧スキーマ互換: preview_base_url 不在の JSON でも None で受けられる。
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000000",
            "name": "Legacy",
            "author": "x",
            "author_github": "x",
            "author_pubkey_id": "abcd",
            "sha256": "00",
            "signature": "AA==",
            "download_url": "https://example.com/pack",
            "version": "1.0.0"
        }"#;
        let entry: MarketplaceEntry = serde_json::from_str(json).unwrap();
        assert!(entry.preview_base_url.is_none());
    }

    #[test]
    fn author_record_accepts_github_key_alias() {
        // 実サーバーが返す JSON 形式: `github` キー + `display_name` を含む (display_name は捨てられて OK)
        let json = r#"{
            "github": "nishiuriraku",
            "display_name": "nishiuriraku",
            "public_key": "0k3mqDQtxdbY9LN7VX9n9vDO8QTB5fySZBDbJqBwfaQ="
        }"#;
        let record: AuthorRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.github_username, "nishiuriraku");
        assert_eq!(
            record.public_key,
            "0k3mqDQtxdbY9LN7VX9n9vDO8QTB5fySZBDbJqBwfaQ="
        );
        assert!(record.historical_keys.is_empty());
    }

    #[test]
    fn author_record_accepts_legacy_github_username_key() {
        // 旧スキーマ互換: `github_username` キーでも引き続き受け入れる
        let json = r#"{
            "github_username": "alice",
            "public_key": "AAAA"
        }"#;
        let record: AuthorRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.github_username, "alice");
    }

    #[test]
    fn author_record_defaults_github_username_when_missing() {
        // 両方のキーが欠落していても deserialize できる (empty string にフォールバック)
        let json = r#"{
            "public_key": "AAAA"
        }"#;
        let record: AuthorRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.github_username, "");
    }

    /// `fetch_index_from` のハッピーパス。200 + 有効な index.json を返す mockito
    /// サーバーを立て、レスポンスが `MarketplaceIndex` に正しく deserialize される
    /// ことを確認する。reqwest 0.13 + rustls feature wiring の最低限の通電試験。
    #[tokio::test]
    async fn fetch_index_parses_minimal_valid_payload() {
        let mut server = mockito::Server::new_async().await;
        let body = r#"{
            "schema_version": 1,
            "commit": "deadbeef",
            "entries": [
                {
                    "id": "6d364941-c605-4def-801a-14ebb401936f",
                    "name": "Mint",
                    "author": "alice",
                    "author_github": "alice",
                    "author_pubkey_id": "abcd",
                    "sha256": "00",
                    "signature": "AA==",
                    "download_url": "https://example.com/pack",
                    "version": "1.0.0"
                }
            ]
        }"#;
        let _m = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;

        let url = format!("{}/index.json", server.url());
        let index = MarketplaceClient::fetch_index_from(&url).await.unwrap();
        assert_eq!(index.schema_version, 1);
        assert_eq!(index.commit.as_deref(), Some("deadbeef"));
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].author_github, "alice");
    }

    /// HTTP 500 が返ったときに `AppError::Theme` でエラー伝播することを確認する。
    /// `error_for_status` の経路が正しく `AppError` にマッピングされているか
    /// (= サイレントに成功扱いになっていないか) の回帰防御。
    #[tokio::test]
    async fn fetch_index_returns_error_on_5xx() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/index.json")
            .with_status(500)
            .with_body("internal error")
            .create_async()
            .await;

        let url = format!("{}/index.json", server.url());
        let err = MarketplaceClient::fetch_index_from(&url).await.unwrap_err();
        assert!(
            matches!(err, AppError::Theme(_)),
            "5xx は AppError::Theme で返るべき: {:?}",
            err
        );
    }

    /// レスポンス本文が壊れた JSON の場合、`serde_json` 側のエラーがそのまま
    /// `AppError` に変換されて返ること。本番では `?` 経由で `From<serde_json::Error>`
    /// が走るので、テスト側でも同じパスを通す。
    #[tokio::test]
    async fn fetch_index_returns_error_on_malformed_json() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_body("{ not json")
            .create_async()
            .await;

        let url = format!("{}/index.json", server.url());
        let err = MarketplaceClient::fetch_index_from(&url).await.unwrap_err();
        // serde_json::Error → AppError は `?` で実装されている経路。具体的な
        // variant は実装に依存するが、Ok ではないことが回帰防御として重要。
        let _ = err;
    }

    fn sample_index_json() -> String {
        r#"{
            "schema_version": 1,
            "commit": "deadbeef",
            "entries": [
                {
                    "id": "6d364941-c605-4def-801a-14ebb401936f",
                    "name": "Mint",
                    "author": "alice",
                    "author_github": "alice",
                    "author_pubkey_id": "abcd",
                    "sha256": "00",
                    "signature": "AA==",
                    "download_url": "https://example.com/pack",
                    "version": "1.0.0"
                }
            ]
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn cached_fetch_success_writes_cache_and_is_not_stale() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_body(sample_index_json())
            .create_async()
            .await;
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("_marketplace_index_cache.json");
        let r = MarketplaceClient::fetch_index_with_cache_from(
            &format!("{}/index.json", server.url()),
            &cache,
        )
        .await
        .unwrap();
        assert!(!r.stale && r.error.is_none() && r.fetched_at.is_some());
        assert!(cache.exists());
        assert_eq!(r.index.entries.len(), 1);
    }

    #[tokio::test]
    async fn cached_fetch_failure_serves_cache_as_stale() {
        // 1 回目 200 でキャッシュを作り、2 回目 500 で stale=true になる
        let mut server = mockito::Server::new_async().await;
        let _m_ok = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_body(sample_index_json())
            .create_async()
            .await;
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("_marketplace_index_cache.json");
        let url = format!("{}/index.json", server.url());
        MarketplaceClient::fetch_index_with_cache_from(&url, &cache)
            .await
            .unwrap();
        drop(_m_ok);
        let _m_err = server
            .mock("GET", "/index.json")
            .with_status(500)
            .with_body("oops")
            .create_async()
            .await;
        let r = MarketplaceClient::fetch_index_with_cache_from(&url, &cache)
            .await
            .unwrap();
        assert!(r.stale);
        assert!(r.error.is_some());
        assert_eq!(r.index.entries.len(), 1);
    }

    #[tokio::test]
    async fn cached_fetch_failure_without_cache_is_error() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/index.json")
            .with_status(500)
            .with_body("oops")
            .create_async()
            .await;
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("_marketplace_index_cache.json");
        let url = format!("{}/index.json", server.url());
        assert!(MarketplaceClient::fetch_index_with_cache_from(&url, &cache)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn corrupt_cache_is_ignored_and_network_error_propagates() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/index.json")
            .with_status(500)
            .with_body("oops")
            .create_async()
            .await;
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("_marketplace_index_cache.json");
        std::fs::write(&cache, "{").unwrap();
        let url = format!("{}/index.json", server.url());
        assert!(MarketplaceClient::fetch_index_with_cache_from(&url, &cache)
            .await
            .is_err());
    }

    #[test]
    fn read_index_cache_resanitizes_homepage() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("_marketplace_index_cache.json");
        // homepage に javascript: を含む生ボディ → 読込時に None へ健全化
        let mut raw: serde_json::Value = serde_json::from_str(&sample_index_json()).unwrap();
        raw["entries"][0]["homepage"] = serde_json::Value::String("javascript:x".to_string());
        let payload = serde_json::json!({
            "cache_version": 1,
            "fetched_at": "2026-09-28T00:00:00Z",
            "raw": serde_json::to_string(&raw).unwrap(),
        });
        std::fs::write(&cache, serde_json::to_string(&payload).unwrap()).unwrap();
        let cached = MarketplaceClient::read_index_cache(&cache)
            .unwrap()
            .expect("cache readable");
        let index = MarketplaceClient::parse_cached_index(&cached).unwrap();
        assert_eq!(index.entries[0].homepage, None);
    }
}
