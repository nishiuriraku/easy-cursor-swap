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

use crate::config::{ConfigManager, GithubAccount};
use crate::errors::AppError;
use crate::github::client::Client;
use crate::github::device_flow::{DeviceFlow, PollOutcome};
use crate::github::types::SubmitResult;
use crate::keystore::Keystore;
use serde::Serialize;
use std::sync::RwLock;
use tauri::{AppHandle, Emitter, State};

const UPSTREAM_OWNER: &str = "nishiuriraku";
const UPSTREAM_REPO: &str = "easy-cursor-swap-index";
const MAX_PACK_SIZE: usize = 50 * 1024 * 1024;

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

#[tauri::command]
pub async fn submit_theme_auto(app: AppHandle, theme_id: String) -> Result<SubmitResult, AppError> {
    let parsed_id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("テーマ ID パース失敗: {}", e)))?;

    emit_progress(&app, "build");
    let pack_bytes = build_cursorpack_for_submit(parsed_id)?;
    if pack_bytes.len() > MAX_PACK_SIZE {
        return Err(AppError::Theme(format!(
            ".cursorpack が {}MB 超: 提出できません",
            MAX_PACK_SIZE / 1024 / 1024
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

    emit_progress(&app, "auth");
    let token = Keystore::load_github_oauth_token()?
        .ok_or_else(|| AppError::Theme("GitHub と未連携です".to_string()))?;
    let gh = Client::new(token);
    let me = gh.get_authenticated_user().await?;

    emit_progress(&app, "fork");
    let fork = gh.ensure_fork(UPSTREAM_OWNER, UPSTREAM_REPO).await?;

    emit_progress(&app, "sync_fork");
    // fork sync は失敗しても致命ではない (新規 fork なら upstream と一致しているはず)。
    // 失敗時は warn だけ出して branch 作成に進む。
    if let Err(e) = gh
        .sync_fork_with_upstream(&fork.owner.login, &fork.name, &fork.default_branch)
        .await
    {
        tracing::warn!("fork sync 失敗 (続行): {}", e);
    }

    let branch = format!("submit/{}", theme_id);
    emit_progress(&app, "branch");
    gh.create_or_reset_branch(&fork.owner.login, &fork.name, &branch, &fork.default_branch)
        .await?;

    emit_progress(&app, "upload_pack");
    gh.put_contents(
        &fork.owner.login,
        &fork.name,
        &branch,
        &format!("themes/{}.cursorpack", theme_id),
        &pack_bytes,
        &format!("feat: add cursorpack for {}", theme_id),
    )
    .await?;

    emit_progress(&app, "upload_entry");
    let meta = load_theme_meta_for_submit(parsed_id)?;
    let download_url = format!(
        "https://raw.githubusercontent.com/{}/{}/main/themes/{}.cursorpack",
        UPSTREAM_OWNER, UPSTREAM_REPO, theme_id
    );
    let entry_json = build_entry_json(
        &theme_id,
        &meta,
        &me.login,
        &pubkey_id,
        &sha256,
        &signature_b64,
        &download_url,
    )?;
    gh.put_contents(
        &fork.owner.login,
        &fork.name,
        &branch,
        &format!("entries/{}.json", theme_id),
        entry_json.as_bytes(),
        &format!("feat: add entry for {}", theme_id),
    )
    .await?;

    emit_progress(&app, "open_pr");
    let head = format!("{}:{}", fork.owner.login, branch);
    let title = format!("submit: {} v{}", meta.display_name, meta.version);
    let body = render_pr_body(
        &theme_id,
        &meta.display_name,
        &meta.version,
        &me.login,
        &sha256,
        &signature_b64,
    );
    let pr = gh
        .open_or_update_pr(UPSTREAM_OWNER, UPSTREAM_REPO, &head, "main", &title, &body)
        .await?;

    tracing::info!("Marketplace 自動提出完了: PR #{}", pr.number);
    Ok(SubmitResult {
        pr_url: pr.html_url,
        pr_number: pr.number,
    })
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

struct ThemeMetaForSubmit {
    display_name: String,
    version: String,
    included_roles: Vec<String>,
}

fn load_theme_meta_for_submit(theme_id: uuid::Uuid) -> Result<ThemeMetaForSubmit, AppError> {
    let meta = crate::theme::ThemeManager::load_metadata(theme_id)?;
    let display_name = match &meta.name {
        crate::theme::LocalizedString::Simple(s) => s.clone(),
        crate::theme::LocalizedString::Localized(m) => m
            .get("en")
            .or_else(|| m.get("ja"))
            .cloned()
            .unwrap_or_else(|| "Untitled".to_string()),
    };
    Ok(ThemeMetaForSubmit {
        display_name,
        version: meta.version,
        included_roles: meta.cursors.keys().cloned().collect(),
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

fn build_entry_json(
    theme_id: &str,
    meta: &ThemeMetaForSubmit,
    author_github: &str,
    author_pubkey_id: &str,
    sha256: &str,
    signature_b64: &str,
    download_url: &str,
) -> Result<String, AppError> {
    let entry = serde_json::json!({
        "id": theme_id,
        "name": meta.display_name,
        "author": author_github,
        "author_github": author_github,
        "author_pubkey_id": author_pubkey_id,
        "sha256": sha256,
        "signature": signature_b64,
        "download_url": download_url,
        "version": meta.version,
        "included_roles": meta.included_roles,
        "tags": []
    });
    Ok(serde_json::to_string_pretty(&entry)?)
}

fn render_pr_body(
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
    fn sha256_hex_is_deterministic_64_chars() {
        let h1 = sha256_hex(b"hello");
        let h2 = sha256_hex(b"hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
