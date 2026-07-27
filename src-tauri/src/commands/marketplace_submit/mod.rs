//! Marketplace 自動提出 IPC (Phase 10)。
//!
//! 5 個の `#[tauri::command]` を提供する:
//!  - `start_device_flow`     — GitHub Device Flow を開始し user_code を返す
//!  - `complete_device_flow`  — pending token を 1 回ポーリングする
//!  - `cancel_device_flow`    — pending を破棄
//!  - `submit_theme_auto`     — 自動 PR 作成 (token 必須)
//!  - `revoke_github_link`    — 保存済みトークンとアカウントメタを削除
//!
//! polling 間隔制御 (interval / slow_down 5s 加算) は **フロント側** で行う。
//! Rust 側はステートレスな 1 try IPC のみを提供する。
//!
//! `submit_theme_auto` の 9 ステージオーケストレーションは
//! [`stages::run_submit_pipeline`] に委譲する。本ファイルは前段 (UUID パース /
//! lineage ガード / タグ allow-list / pack build+sign / token load /
//! client 構築) までの **preflight adapter** に徹する。

use crate::commands::marketplace_submit::stages::{
    run_submit_pipeline, validate_tags, AppHandleSink, SubmitPreflight,
};
use crate::config::{ConfigManager, GithubAccount, DEFAULT_MAX_PACK_COMPRESSED_SIZE};
use crate::errors::AppError;
use crate::github::client::Client;
use crate::github::device_flow::{DeviceFlow, PollOutcome};
use crate::github::types::SubmitResult;
use crate::keystore::Keystore;
use serde::Serialize;
use std::sync::RwLock;
use tauri::{AppHandle, Emitter, State};

pub(crate) mod stages;

/// Device Flow 開始時に GitHub から受け取った値のうち、ポーリングに必要な分。
#[derive(Debug, Clone)]
pub struct PendingFlow {
    pub device_code: String,
    pub interval_secs: u64,
    pub expires_at_unix: u64,
}

/// `.manage()` で登録する Device Flow の pending 状態。
#[derive(Default)]
pub struct DeviceFlowState(RwLock<Option<PendingFlow>>);

impl DeviceFlowState {
    pub fn set(&self, p: PendingFlow) {
        if let Ok(mut g) = self.0.write() {
            *g = Some(p);
        }
    }

    pub fn snapshot(&self) -> Option<PendingFlow> {
        self.0.read().ok().and_then(|g| g.clone())
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.0.write() {
            *g = None;
        }
    }
}

/// `start_device_flow` の戻り値 (camelCase でフロントへ渡す)。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartFlowResult {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// `complete_device_flow` の戻り値。
/// `status` フィールドでフロントが分岐する (`pending` / `slow_down` / `expired` / `denied` / `ready`)。
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum CompleteFlowResult {
    Pending,
    SlowDown,
    Expired,
    Denied,
    Ready { login: String },
}

#[tauri::command]
pub async fn start_device_flow(
    state: State<'_, DeviceFlowState>,
) -> Result<StartFlowResult, AppError> {
    let cid = crate::github::client_id();
    if cid.is_empty() {
        return Err(AppError::Theme(
            "GitHub OAuth Client ID がビルド時に未設定です。手動モードを使用してください"
                .to_string(),
        ));
    }
    let dc = DeviceFlow::start(cid).await?;
    let expires_at_unix = current_unix_secs().saturating_add(dc.expires_in);
    state.set(PendingFlow {
        device_code: dc.device_code,
        interval_secs: dc.interval,
        expires_at_unix,
    });
    tracing::info!(
        "Device Flow 開始 (interval={}s, expires_in={}s)",
        dc.interval,
        dc.expires_in
    );
    Ok(StartFlowResult {
        user_code: dc.user_code,
        verification_uri: dc.verification_uri,
        expires_in: dc.expires_in,
        interval: dc.interval,
    })
}

#[tauri::command]
pub async fn complete_device_flow(
    state: State<'_, DeviceFlowState>,
    config: State<'_, ConfigManager>,
) -> Result<CompleteFlowResult, AppError> {
    let pending = state
        .snapshot()
        .ok_or_else(|| AppError::Theme("Device Flow が開始されていません".to_string()))?;
    let cid = crate::github::client_id();
    let outcome = DeviceFlow::poll(cid, &pending.device_code).await?;
    Ok(match outcome {
        PollOutcome::Pending => CompleteFlowResult::Pending,
        PollOutcome::SlowDown => CompleteFlowResult::SlowDown,
        PollOutcome::Expired => {
            state.clear();
            CompleteFlowResult::Expired
        }
        PollOutcome::Denied => {
            state.clear();
            CompleteFlowResult::Denied
        }
        PollOutcome::Ready { access_token, .. } => {
            Keystore::save_github_oauth_token(&access_token)?;
            let client = Client::new(access_token);
            let user = client.get_authenticated_user().await?;
            let now = chrono::Utc::now().to_rfc3339();
            config.update(|c| {
                c.github_account = Some(GithubAccount {
                    login: user.login.clone(),
                    token_saved_at: now.clone(),
                });
            })?;
            state.clear();
            tracing::info!("GitHub 連携完了");
            CompleteFlowResult::Ready { login: user.login }
        }
    })
}

#[tauri::command]
pub fn cancel_device_flow(state: State<'_, DeviceFlowState>) -> Result<(), AppError> {
    state.clear();
    Ok(())
}

#[tauri::command]
pub async fn revoke_github_link(
    state: State<'_, DeviceFlowState>,
    config: State<'_, ConfigManager>,
) -> Result<(), AppError> {
    Keystore::delete_github_oauth_token()?;
    config.update(|c| {
        c.github_account = None;
    })?;
    state.clear();
    tracing::info!("GitHub 連携を解除");
    Ok(())
}

/// `tags` はユーザーが提出ダイアログで入力したマーケットプレイス用タグ。
/// 空配列の場合は theme metadata 側の tags が使われる。
/// 提出時のみ反映 (theme metadata 自体は書き換えない)。
#[tauri::command]
pub async fn submit_theme_auto(
    app: AppHandle,
    theme_id: String,
    tags: Vec<String>,
) -> Result<SubmitResult, AppError> {
    // ── preflight (この IPC ハンドラの責務) ─────────────────────
    // UUID パース → lineage 拒否 → タグ allow-list 検証 → pack build+sign →
    // token load → client 構築 → `run_submit_pipeline` に委譲。
    // 9 ステージのオーケストレーションは `stages::run_submit_pipeline` に閉じている。
    let parsed_id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("テーマ ID パース失敗: {}", e)))?;

    // ── lineage ガード ────────────────────────────────────────
    // 公式インデックス由来テーマ自体 (source = Marketplace) は SubmitThemeDialog の
    // 提出可能一覧から既に弾かれているが、ユーザーが duplicate_theme で複製してから
    // 提出してきた場合に備え、Rust 側でも `cloned_from_marketplace_id` を確認する。
    // 複製してから何段ネストしても `duplicate_theme` が origin を引き継ぐので、
    // この 1 か所のチェックだけで再提出経路を全て塞げる。
    let lineage_meta = crate::theme::ThemeManager::load_metadata(parsed_id)?;
    if let Some(origin) = lineage_meta.cloned_from_marketplace_id {
        tracing::warn!(
            "marketplace 由来テーマの再提出を拒否: origin_short={}",
            crate::logging::short_hash(origin.to_string().as_bytes())
        );
        return Err(AppError::Theme(
            "公式インデックス由来テーマを複製したものは再提出できません".to_string(),
        ));
    }

    // ── タグ allow-list (ネットワーク呼び出し前に検証) ──────────
    let normalized_tags = validate_tags(tags)?;

    emit_progress(&app, "build");
    // pack build + SHA-256 + 署名は CPU 重めなので spawn_blocking で逃がす。
    // 実行スレッド自体は async executor を占有しないため、IPC 呼び出し中に他の
    // Tauri コマンド (cancel など) がフリーズしない。
    let parsed_id_for_blocking = parsed_id;
    let lineage_for_preflight = lineage_meta.cloned_from_marketplace_id;
    let preflight = tauri::async_runtime::spawn_blocking(move || -> Result<SubmitPreflight, AppError> {
        let pack_bytes = build_cursorpack_for_submit(parsed_id_for_blocking)?;
        if (pack_bytes.len() as u64) > DEFAULT_MAX_PACK_COMPRESSED_SIZE {
            return Err(AppError::Theme(format!(
                ".cursorpack が {}MB 超: 提出できません",
                DEFAULT_MAX_PACK_COMPRESSED_SIZE / 1024 / 1024
            )));
        }
        let sha256 = sha256_hex(&pack_bytes);
        // .cursorpack 全体の SHA-256 (16進文字列) を署名対象とする。
        // 公式 marketplace のインストール側も同じ規約を使う。
        let signature_b64 = Keystore::sign(sha256.as_bytes())?;
        let key_info = Keystore::info()?;
        let pubkey_id = key_info
            .key_id
            .ok_or_else(|| AppError::Theme("署名鍵が未生成です".to_string()))?;
        let meta = load_theme_meta_for_submit(parsed_id_for_blocking)?;
        Ok(SubmitPreflight {
            theme_id: parsed_id_for_blocking,
            theme_id_str: parsed_id_for_blocking.to_string(),
            tags: normalized_tags,
            pack_bytes,
            sha256,
            signature_b64,
            pubkey_id,
            meta,
            cloned_from_marketplace_id: lineage_for_preflight,
        })
    })
    .await
    .map_err(|e| AppError::Theme(format!("pack build join エラー: {}", e)))??;

    emit_progress(&app, "auth");
    let token = Keystore::load_github_oauth_token()?
        .ok_or_else(|| AppError::Theme("GitHub と未連携です".to_string()))?;
    let client = Client::new(token);

    // 9 ステージのネットワークオーケストレーションは stages.rs 側に完全に委譲する。
    // IPC ハンドラ側は ProgressSink adapter を作って submit:progress イベントを
    // 既存と同じ semantics で発火させる。
    let sink = AppHandleSink::new(&app);
    run_submit_pipeline(&sink, &client, preflight).await
}

// ── helpers ────────────────────────────────────────────────────────────

fn emit_progress(app: &AppHandle, stage: &str) {
    if let Err(e) = app.emit("submit:progress", stage) {
        tracing::warn!("submit:progress emit 失敗 ({}): {}", stage, e);
    }
}

fn current_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// `submit_theme_auto` IPC が組み立てて `stages::run_submit_pipeline` に渡す
/// theme.json のサブセット。`build_entry_json` の入力となる。
///
/// `pub(crate)` で公開し、`stages` モジュールからも参照する。
/// `Clone` を derive しているのは `stages` 側で `meta.clone()` するため
/// (= ユーザー入力 tags 適用時に in-place 変更するため)。
#[derive(Clone)]
pub(crate) struct ThemeMetaForSubmit {
    /// `theme.json` の name をそのまま (LocalizedString のまま) 保持。
    /// `build_entry_json` で `index.json` に出力する際は serde untagged で
    /// plain string またはロケールマップとして書き出される。
    name: crate::theme::LocalizedString,
    /// PR タイトル / ログ等の単一文字列が必要な箇所用の表示名。
    /// fallback: en → ja → "default" → first → "Untitled"。
    display_name: String,
    /// `theme.json` の `author` フィールド (= 作者クレジット, plain string)。
    /// 公式インデックスの `entry.author` はこの値を優先し、未設定 (None) の
    /// 場合のみ提出者の GitHub username にフォールバックする。
    /// 提出者識別 (`author_github` / authors/<gh>.json 紐付け / 署名検証) には
    /// `me.login` を使うため、この 2 概念を取り違えないこと。
    author: Option<String>,
    version: String,
    included_roles: Vec<String>,
    tags: Vec<String>,
}

/// `LocalizedString` を index.json 用に JSON Value 化する。
///
/// 公式インデックスのスキーマ (`schemas/index-entry.json`) は name について
/// `oneOf: [string, { required: ["ja", "en"] }]` を要求している。
/// したがって object 形式は **ja と en の両方が必須**。Creator UI では en は
/// 任意なので、ユーザーが ja だけ入力したテーマは object 形式のままだと
/// validate を通らない。
///
/// 解決策: `Localized(map)` でキーが 1 個以下なら、その値を plain string として
/// 書き出す (schema の string 分岐で通過する)。これにより:
///   - 「ja のみ」「en のみ」のテーマも提出が通る
///   - 表示側 (`pickLocalizedName`) は plain string なら全ロケールで同じ値を返すので
///     データロスゼロ
///   - 「ja=en と同値で重複入力した」ケースは object のまま残す (UI で明示的に
///     2 ロケール指定したと判断)
fn name_value_for_index(
    name: &crate::theme::LocalizedString,
) -> Result<serde_json::Value, AppError> {
    use crate::theme::LocalizedString;
    match name {
        LocalizedString::Simple(s) => Ok(serde_json::Value::String(s.clone())),
        LocalizedString::Localized(map) => {
            // 0 個 (異常系: hand-edited theme.json) も 1 個も plain string に潰す。
            // 0 個のときは空文字列 — schema は minLength を課していないので形式上は通る。
            if map.len() <= 1 {
                return Ok(serde_json::Value::String(
                    map.values().next().cloned().unwrap_or_default(),
                ));
            }
            Ok(serde_json::to_value(name)?)
        }
    }
}

fn load_theme_meta_for_submit(theme_id: uuid::Uuid) -> Result<ThemeMetaForSubmit, AppError> {
    let meta = crate::theme::ThemeManager::load_metadata(theme_id)?;
    let display_name = match &meta.name {
        crate::theme::LocalizedString::Simple(s) => s.clone(),
        crate::theme::LocalizedString::Localized(m) => m
            .get("en")
            .or_else(|| m.get("ja"))
            .or_else(|| m.get("default"))
            .cloned()
            .or_else(|| m.values().next().cloned())
            .unwrap_or_else(|| "Untitled".to_string()),
    };
    Ok(ThemeMetaForSubmit {
        name: meta.name,
        display_name,
        author: meta.author,
        version: meta.version,
        included_roles: meta.cursors.keys().cloned().collect(),
        tags: meta.tags,
    })
}

fn build_cursorpack_for_submit(theme_id: uuid::Uuid) -> Result<Vec<u8>, AppError> {
    let cursors_dir = ConfigManager::cursors_dir()?;
    let theme_dir = cursors_dir.join(theme_id.to_string());
    let mut metadata = crate::theme::ThemeManager::load_metadata(theme_id)?;
    let mut cursors: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();
    for (role, def) in &metadata.cursors {
        let p = theme_dir.join(&def.file);
        let bin = std::fs::read(&p).map_err(|e| {
            AppError::Theme(format!(
                "カーソル {} の読込失敗 ({}): {}",
                role,
                crate::logging::redact_path(&p),
                e
            ))
        })?;
        cursors.insert(role.clone(), bin);
    }
    crate::theme::ThemeManager::write_cursorpack_to_buffer(&mut metadata, &cursors)
}

/// `index.json` 用のエントリ JSON 文字列を組み立てる。
///
/// `stages::run_submit_pipeline` から呼ばれるため `pub(crate)`。
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_entry_json(
    theme_id: &str,
    meta: &ThemeMetaForSubmit,
    author_github: &str,
    author_pubkey_id: &str,
    sha256: &str,
    signature_b64: &str,
    download_url: &str,
    preview_base_url: Option<&str>,
) -> Result<String, AppError> {
    // name は LocalizedString のまま untagged で書き出す。
    // - Simple("Foo") なら "name": "Foo"
    // - Localized({"ja": "ミント", "en": "Mint"}) なら "name": {"ja": "ミント", "en": "Mint"}
    // - Localized({"ja": "X"}) のように 1 ロケールしか無い場合は plain string に降格して
    //   "name": "X" として出力 (index 側 schemas/index-entry.json は object 形式の場合
    //   `required: ["ja", "en"]` を要求するため、片方欠落の object は validate を通らない)。
    let name_value = name_value_for_index(&meta.name)?;
    // `author` は作者クレジット = theme.json の `author` を最優先。
    // theme.json で未設定の場合のみ提出者 (`author_github` = `me.login`) にフォールバック。
    // この分離が無いと、別人 (例: コミュニティ) が代理提出したテーマで本来の作者名が
    // 上書きされてしまう (旧バグ: index entry `f2d3825c` の author が `"無ナ"` ではなく
    // `"nishiuriraku"` で書き出されていた事例)。
    let author_credit = meta.author.as_deref().unwrap_or(author_github);
    let mut entry = serde_json::json!({
        "id": theme_id,
        "name": name_value,
        "author": author_credit,
        "author_github": author_github,
        "author_pubkey_id": author_pubkey_id,
        "sha256": sha256,
        "signature": signature_b64,
        "download_url": download_url,
        "version": meta.version,
        "included_roles": meta.included_roles,
        "tags": meta.tags.clone()
    });
    if let Some(base) = preview_base_url {
        entry["preview_base_url"] = serde_json::Value::String(base.to_string());
    }
    Ok(serde_json::to_string_pretty(&entry)?)
}

/// PR body の Markdown を組み立てる。
///
/// `stages::run_submit_pipeline` から呼ばれるため `pub(crate)`。
pub(crate) fn render_pr_body(
    theme_id: &str,
    name: &str,
    version: &str,
    author_github: &str,
    sha256: &str,
    signature_b64: &str,
) -> String {
    format!(
        "## Auto-submitted via EasyCursorSwap\n\n\
         - **Theme:** {name} (v{version})\n\
         - **ID:** `{theme_id}`\n\
         - **Author:** @{author_github}\n\
         - **SHA-256:** `{sha256}`\n\
         - **Signature (Ed25519, b64):** `{signature_b64}`\n\n\
         This PR was generated automatically by the app. \
         The maintainer should wait for CI (`marketplace-validate.yml`) before merging.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_flow_state_round_trip() {
        let state = DeviceFlowState::default();
        assert!(state.snapshot().is_none());
        state.set(PendingFlow {
            device_code: "DC".to_string(),
            interval_secs: 5,
            expires_at_unix: 1_700_000_000,
        });
        let s = state.snapshot().expect("must be set");
        assert_eq!(s.device_code, "DC");
        assert_eq!(s.interval_secs, 5);
        assert_eq!(s.expires_at_unix, 1_700_000_000);
        state.clear();
        assert!(state.snapshot().is_none());
    }

    #[test]
    fn render_pr_body_contains_all_metadata() {
        let body = render_pr_body(
            "abc-123",
            "My Theme",
            "1.0.0",
            "octocat",
            "0123abcd0123abcd",
            "SIGNATURE_B64",
        );
        assert!(body.contains("abc-123"));
        assert!(body.contains("My Theme"));
        assert!(body.contains("1.0.0"));
        assert!(body.contains("0123abcd0123abcd"));
        assert!(body.contains("SIGNATURE_B64"));
        assert!(body.contains("octocat"));
        assert!(body.contains("Auto-submitted"));
    }

    #[test]
    fn build_entry_json_preserves_localized_name() {
        // theme.json が LocalizedString::Localized を持っている場合、
        // index.json にも同じロケールマップが書き出されることを固定する。
        // この挙動が「JA モードでも EN 名が表示される」バグの根治パスになる。
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("ja".to_string(), "ミント".to_string());
        map.insert("en".to_string(), "Mint".to_string());
        map.insert("default".to_string(), "EasyCursorSwap Mint".to_string());

        let meta = ThemeMetaForSubmit {
            name: crate::theme::LocalizedString::Localized(map),
            display_name: "Mint".to_string(),
            author: None,
            version: "1.0.0".to_string(),
            included_roles: vec!["Arrow".to_string()],
            tags: vec!["minimal".to_string()],
        };
        let json = build_entry_json(
            "00000000-0000-0000-0000-000000000000",
            &meta,
            "octocat",
            "deadbeef",
            "00",
            "AA==",
            "https://example.com/pack",
            None,
        )
        .unwrap();
        // 出力 JSON を parse して name フィールドがオブジェクトかつ ja キーを持つことを確認
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let name = &v["name"];
        assert!(name.is_object(), "name should be an object, got: {name}");
        assert_eq!(name["ja"], "ミント");
        assert_eq!(name["en"], "Mint");
        assert_eq!(name["default"], "EasyCursorSwap Mint");
    }

    #[test]
    fn build_entry_json_downgrades_localized_with_single_locale_to_plain_string() {
        // Creator UI で「名前 (英語)」を空のまま提出した場合、theme.json は
        // Localized({"ja": "ハムチマウスカーソル"}) になる。このままだと
        // index schema は required: ["ja", "en"] で弾くので、plain string に
        // 降格させて schema の string 分岐で通すのが正しい。
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("ja".to_string(), "ハムチマウスカーソル".to_string());

        let meta = ThemeMetaForSubmit {
            name: crate::theme::LocalizedString::Localized(map),
            display_name: "ハムチマウスカーソル".to_string(),
            author: None,
            version: "1.0.0".to_string(),
            included_roles: vec!["Arrow".to_string()],
            tags: vec![],
        };
        let json = build_entry_json(
            "00000000-0000-0000-0000-000000000000",
            &meta,
            "octocat",
            "deadbeef",
            "00",
            "AA==",
            "https://example.com/pack",
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        // 期待: plain string に降格
        assert_eq!(v["name"], "ハムチマウスカーソル");
        assert!(
            v["name"].is_string(),
            "single-locale name should downgrade to plain string"
        );
    }

    #[test]
    fn build_entry_json_downgrades_en_only_to_plain_string() {
        // 対称ケース: en だけのテーマも plain string に降格。
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("en".to_string(), "EnglishOnly".to_string());

        let meta = ThemeMetaForSubmit {
            name: crate::theme::LocalizedString::Localized(map),
            display_name: "EnglishOnly".to_string(),
            author: None,
            version: "1.0.0".to_string(),
            included_roles: vec![],
            tags: vec![],
        };
        let json = build_entry_json(
            "00000000-0000-0000-0000-000000000000",
            &meta,
            "octocat",
            "deadbeef",
            "00",
            "AA==",
            "https://example.com/pack",
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["name"], "EnglishOnly");
    }

    #[test]
    fn build_entry_json_keeps_plain_name_when_simple() {
        // theme.json が LocalizedString::Simple ("Foo") の場合は index.json も
        // 文字列のまま出力される (既存の curated index と完全互換)。
        let meta = ThemeMetaForSubmit {
            name: crate::theme::LocalizedString::Simple("Plain Name".to_string()),
            display_name: "Plain Name".to_string(),
            author: None,
            version: "1.0.0".to_string(),
            included_roles: vec![],
            tags: vec![],
        };
        let json = build_entry_json(
            "00000000-0000-0000-0000-000000000000",
            &meta,
            "octocat",
            "deadbeef",
            "00",
            "AA==",
            "https://example.com/pack",
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["name"], "Plain Name");
    }

    #[test]
    fn build_entry_json_prefers_theme_author_over_github_login() {
        // 回帰テスト: theme.json に `author` が書かれていれば、index entry の `author` は
        // 必ずそれを採用する (提出者 GitHub username = `author_github` で上書きしない)。
        //
        // 旧バグ: 自動提出 (`submit_theme_auto`) が `author` と `author_github` の両方に
        // `me.login` を入れていたため、第三者が代理提出すると本来の作者クレジットが消えた。
        // 実例: cursorpack `f2d3825c` (theme.author = "無ナ") の index entry が
        // `author: "nishiuriraku"` (= 提出者) で書き出されていた。
        let meta = ThemeMetaForSubmit {
            name: crate::theme::LocalizedString::Simple("Hamuchi Mouse Cursor".to_string()),
            display_name: "Hamuchi Mouse Cursor".to_string(),
            author: Some("無ナ".to_string()),
            version: "1.0.0".to_string(),
            included_roles: vec!["Arrow".to_string()],
            tags: vec![],
        };
        let json = build_entry_json(
            "00000000-0000-0000-0000-000000000000",
            &meta,
            "nishiuriraku",
            "deadbeef",
            "00",
            "AA==",
            "https://example.com/pack",
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            v["author"], "無ナ",
            "entry.author は theme.author を採用すべき"
        );
        assert_eq!(
            v["author_github"], "nishiuriraku",
            "entry.author_github は提出者 GitHub username であるべき"
        );
    }

    #[test]
    fn build_entry_json_falls_back_to_github_when_theme_author_missing() {
        // theme.json に author が無い (Option::None) 場合は、index entry の `author` を
        // 提出者の GitHub username にフォールバックする (Manual タブの
        // `th.author ?? githubUsername.value` と同じ挙動)。
        // schema が `author` を required string にしているので空文字列にはせず、
        // 必ず非空の値を入れる必要がある。
        let meta = ThemeMetaForSubmit {
            name: crate::theme::LocalizedString::Simple("No Author".to_string()),
            display_name: "No Author".to_string(),
            author: None,
            version: "1.0.0".to_string(),
            included_roles: vec![],
            tags: vec![],
        };
        let json = build_entry_json(
            "00000000-0000-0000-0000-000000000000",
            &meta,
            "octocat",
            "deadbeef",
            "00",
            "AA==",
            "https://example.com/pack",
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["author"], "octocat");
        assert_eq!(v["author_github"], "octocat");
    }

    #[test]
    fn sha256_hex_is_deterministic_64_chars() {
        let h1 = sha256_hex(b"hello");
        let h2 = sha256_hex(b"hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }
}