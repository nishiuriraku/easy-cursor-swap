//! 公式インデックス (Marketplace) IPC コマンド。
//!
//! `nishiuriraku/easy-cursor-swap-index` リポジトリで配布される署名済みテーマ一覧の
//! 取得とインストールを担当する。HTTPS + rustls / SHA-256 / Ed25519 で検証。

use crate::errors::AppError;
use crate::marketplace::{MarketplaceClient, MarketplaceIndex, MarketplaceInstallRequest};

/// 公式インデックス (Marketplace) のメタデータを取得する。
/// `nishiuriraku/easy-cursor-swap-index` リポジトリの `index.json` を HTTPS + rustls で取得。
#[tauri::command]
pub async fn marketplace_fetch_index() -> Result<MarketplaceIndex, AppError> {
    MarketplaceClient::fetch_index().await
}

/// 公式インデックスから指定エントリをダウンロード→検証→展開する。
/// (1) ダウンロード, (2) SHA-256 整合性, (3) Ed25519 署名検証, (4) ZIP 展開。
/// 戻り値はインポートしたテーマ ID (UUID 文字列)。
#[tauri::command]
pub async fn marketplace_install(req: MarketplaceInstallRequest) -> Result<String, AppError> {
    let id = MarketplaceClient::install(req).await?;
    Ok(id.to_string())
}

/// 公式インデックスから 1 ロール分のプレビュー PNG を取得する。
/// MarketplaceDetailModal で 6 ロール並列に呼ばれる。
#[tauri::command]
pub async fn marketplace_fetch_preview(
    preview_base_url: String,
    role: String,
) -> Result<Vec<u8>, AppError> {
    MarketplaceClient::fetch_preview(&preview_base_url, &role).await
}
