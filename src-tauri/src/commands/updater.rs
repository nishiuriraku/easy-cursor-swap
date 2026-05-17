//! Tauri Updater IPC コマンド (チャンネル切替対応)
//!
//! 既定の `@tauri-apps/plugin-updater` (JS 側) は `tauri.conf.json` の
//! `plugins.updater.endpoints` を起動時に確定して使うため、**runtime にチャンネル切替で
//! endpoint を差し替えることが出来ない**。
//!
//! その制約を回避するため、本モジュールでは Rust 側の `UpdaterExt::updater_builder()` を
//! 利用して channel ごとに endpoint を組み立てた `Updater` を都度生成し、check / download
//! を IPC として公開する。フロントは channel に応じてこちらを呼び分ける
//! (`useUpdater.ts` の wiring 参照)。
//!
//! ## チャンネル → endpoint 対応
//!
//! | channel | endpoint |
//! |---|---|
//! | `"stable"` | `releases/latest/download/latest.json` (= tauri.conf.json の default) |
//! | `"beta"`   | `releases/download/beta/latest.json` (固定タグ `beta` の Release) |
//!
//! beta endpoint に対応する parallel release pipeline は未配備のため、現状 beta は
//! HTTP 404 を返す。フロントは 404 を「beta リリースなし」として静かに扱う。

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;
use url::Url;

const STABLE_ENDPOINT: &str =
    "https://github.com/nishiuriraku/easy-cursor-swap/releases/latest/download/latest.json";
const BETA_ENDPOINT: &str =
    "https://github.com/nishiuriraku/easy-cursor-swap/releases/download/beta/latest.json";

/// `check_for_update_on_channel` の戻り値。
/// JS 側 Update 型のうち UI で必要な部分だけ抜粋。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMeta {
    /// 新バージョン (`latest.json` の version)
    pub version: String,
    /// 現バージョン (アプリの Cargo version)
    pub current_version: String,
    /// リリースノート本文 (省略可)
    pub body: Option<String>,
    /// リリース日 (省略可、RFC3339 想定)
    pub date: Option<String>,
}

/// `channel` 文字列 → endpoint URL 解決。未知値は安全側 stable に倒す。
fn resolve_endpoint(channel: &str) -> &'static str {
    match channel {
        "beta" => BETA_ENDPOINT,
        _ => STABLE_ENDPOINT,
    }
}

/// 指定チャンネルで update を check する。
///
/// 戻り値:
///   - `Ok(Some(meta))` — 新バージョンあり
///   - `Ok(None)`       — 最新 / リリース未配備 (404 含む)
///   - `Err(msg)`       — ネットワークエラー等の予期しない失敗
#[tauri::command]
pub async fn check_for_update_on_channel(
    app: AppHandle,
    channel: String,
) -> Result<Option<UpdateMeta>, String> {
    let endpoint = resolve_endpoint(&channel);
    let url = Url::parse(endpoint).map_err(|e| format!("endpoint URL parse 失敗: {e}"))?;

    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(|e| format!("endpoints 設定失敗: {e}"))?
        .build()
        .map_err(|e| format!("updater build 失敗: {e}"))?;

    match updater.check().await {
        Ok(Some(update)) => Ok(Some(UpdateMeta {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
            body: update.body.clone(),
            date: update.date.map(|d| d.to_string()),
        })),
        Ok(None) => Ok(None),
        Err(e) => {
            // beta channel が未配備 → 404 は「更新なし」として静かに返す。
            let msg = e.to_string();
            if channel == "beta" && msg.contains("404") {
                tracing::info!("beta channel 未配備 (404) — 更新なし扱い");
                return Ok(None);
            }
            tracing::warn!("updater.check failed (channel={}): {}", channel, msg);
            Err(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_endpoint_known_channels() {
        assert_eq!(resolve_endpoint("stable"), STABLE_ENDPOINT);
        assert_eq!(resolve_endpoint("beta"), BETA_ENDPOINT);
    }

    #[test]
    fn resolve_endpoint_unknown_falls_back_to_stable() {
        // 安全側: 不正値は stable と同じ endpoint を返す (panic させない)。
        assert_eq!(resolve_endpoint(""), STABLE_ENDPOINT);
        assert_eq!(resolve_endpoint("nightly"), STABLE_ENDPOINT);
        assert_eq!(resolve_endpoint("BETA"), STABLE_ENDPOINT); // 大文字違いも stable へ
    }
}
