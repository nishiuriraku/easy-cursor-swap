//! ts-rs ベースの Rust → TypeScript 型生成器。
//!
//! `cargo run --manifest-path src-tauri/Cargo.toml --features typegen --bin gen_types`
//! で起動し、`app/types/generated/` に以下の `.ts` ファイルを書き出す:
//! - `AppConfig.ts` / `BackupInfo.ts` / `GeneralConfig.ts` / `SecurityConfig.ts`
//!   / `LoggingConfig.ts` / `ThemeUsage.ts` / `GithubAccount.ts`
//! - `MarketplaceIndex.ts` / `MarketplaceEntry.ts` / `MarketplaceInstallRequest.ts`
//!
//! 出力先は `.cargo/config.toml` の `TS_RS_EXPORT_DIR` で固定 (リポジトリルートからの相対パス)。
//! 通常ビルド / 通常 `cargo test` では typegen feature が無効なので、`ts-rs` は一切
//! リンクされず proc-macro コストも発生しない。
//!
//! `MarketplaceEntry` / `MarketplaceInstallRequest` は serde 側で
//! `serialize = camelCase / deserialize = snake_case` の非対称を持つ。
//! ts-rs にも `rename_all = "camelCase"` を渡すことで、IPC シリアライズ境界の
//! ペイロード型 (TS 側 camelCase) と一致させる。

use app_lib::config::patch::AppConfigPatch;
use app_lib::config::{AppConfig, BackupInfo};
use app_lib::marketplace::{MarketplaceEntry, MarketplaceIndex, MarketplaceInstallRequest};
use ts_rs::{Config, TS};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 環境変数 TS_RS_EXPORT_DIR (`.cargo/config.toml` でリポジトリルート相対に固定) を
    // 読み取って出力ディレクトリを決定。指定なしなら `./bindings/` にフォールバック。
    let config = Config::from_env();
    AppConfig::export_all(&config)?;
    AppConfigPatch::export_all(&config)?;
    BackupInfo::export_all(&config)?;
    MarketplaceInstallRequest::export_all(&config)?;
    MarketplaceEntry::export_all(&config)?;
    MarketplaceIndex::export_all(&config)?;
    Ok(())
}
