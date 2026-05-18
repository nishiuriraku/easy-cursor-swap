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

/// `tags` はユーザーが提出ダイアログで入力したマーケットプレイス用タグ。
/// 空配列の場合は theme metadata 側の tags が使われる。
/// 提出時のみ反映 (theme metadata 自体は書き換えない)。
#[tauri::command]
pub async fn submit_theme_auto(
    app: AppHandle,
    theme_id: String,
    tags: Vec<String>,
) -> Result<SubmitResult, AppError> {
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
    // GitHub は「upstream owner 本人」が fork を作ろうとすると、新規 fork ではなく
    // upstream 本体 (= 同じ owner / 同じ repo) を返す。この場合 `merge-upstream` も
    // 「自分の main を自分の main に merge」になり 422 で落ちる。
    // 自前の fork でない (= upstream owner 本人) なら sync をスキップする。
    let is_self_owned = fork.owner.login == UPSTREAM_OWNER && fork.name == UPSTREAM_REPO;

    emit_progress(&app, "sync_fork");
    if is_self_owned {
        tracing::info!("upstream owner 本人のため fork sync をスキップ");
    } else if let Err(e) = gh
        .sync_fork_with_upstream(&fork.owner.login, &fork.name, &fork.default_branch)
        .await
    {
        // fork sync は失敗しても致命ではない (新規 fork なら upstream と一致しているはず)。
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

    emit_progress(&app, "upload_previews");
    // .cursorpack 内の `previews/<role>.png` を抽出し、
    // `previews/<theme_id>/<role>.png` として upstream に置けるように fork へアップロードする。
    // 失敗しても提出全体は止めず警告のみ (entry の preview_base_url は省略)。
    let uploaded_previews = upload_previews_from_pack(
        &gh,
        &fork.owner.login,
        &fork.name,
        &branch,
        &theme_id,
        &pack_bytes,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("preview アップロード失敗 (続行): {}", e);
        false
    });

    emit_progress(&app, "upload_entry");
    let mut meta = load_theme_meta_for_submit(parsed_id)?;
    // 提出ダイアログで入力された tags があれば metadata より優先する。
    // 空配列の場合は metadata の tags をそのまま使う。
    if !tags.is_empty() {
        meta.tags = tags
            .into_iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
    }
    let download_url = format!(
        "https://raw.githubusercontent.com/{}/{}/main/themes/{}.cursorpack",
        UPSTREAM_OWNER, UPSTREAM_REPO, theme_id
    );
    let preview_base_url = if uploaded_previews {
        Some(format!(
            "https://raw.githubusercontent.com/{}/{}/main/previews/{}",
            UPSTREAM_OWNER, UPSTREAM_REPO, theme_id
        ))
    } else {
        None
    };
    let entry_json = build_entry_json(
        &theme_id,
        &meta,
        &me.login,
        &pubkey_id,
        &sha256,
        &signature_b64,
        &download_url,
        preview_base_url.as_deref(),
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
    /// `theme.json` の name をそのまま (LocalizedString のまま) 保持。
    /// `build_entry_json` で `index.json` に出力する際は serde untagged で
    /// plain string またはロケールマップとして書き出される。
    name: crate::theme::LocalizedString,
    /// PR タイトル / ログ等の単一文字列が必要な箇所用の表示名。
    /// fallback: en → ja → "default" → first → "Untitled"。
    display_name: String,
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

#[allow(clippy::too_many_arguments)]
fn build_entry_json(
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
    let mut entry = serde_json::json!({
        "id": theme_id,
        "name": name_value,
        "author": author_github,
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

/// `.cursorpack` (ZIP) から `previews/<role>.png` を抽出し、
/// fork branch の `previews/<theme_id>/<role>.png` として upload する。
///
/// 戻り値は「1 件以上 PNG を upload できたか」。すべて失敗 / 該当ファイル無しの
/// 場合は `false` を返し、呼び出し側は entry の `preview_base_url` を省略する。
async fn upload_previews_from_pack(
    gh: &Client,
    owner: &str,
    repo: &str,
    branch: &str,
    theme_id: &str,
    pack_bytes: &[u8],
) -> Result<bool, AppError> {
    use std::io::Read;
    let reader = std::io::Cursor::new(pack_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| AppError::Theme(format!(".cursorpack ZIP オープン失敗: {}", e)))?;

    // ZIP 内エントリ名から (role, bytes) を抽出。`previews/<role>.png` 形式のみ採用し、
    // 役割名は ASCII 英数字 + アンダースコアに正規化されたものに限定する。
    let mut uploads: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut f = archive
            .by_index(i)
            .map_err(|e| AppError::Theme(format!(".cursorpack エントリ取得失敗: {}", e)))?;
        let name = f.name().to_string();
        let role = match name
            .strip_prefix("previews/")
            .and_then(|s| s.strip_suffix(".png"))
        {
            Some(r) => r,
            None => continue,
        };
        if role.is_empty()
            || role.len() > 32
            || !role.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)
            .map_err(|e| AppError::Theme(format!("preview 読込失敗: {}", e)))?;
        uploads.push((role.to_string(), bytes));
    }

    if uploads.is_empty() {
        tracing::warn!("preview PNG が cursorpack に含まれていません");
        return Ok(false);
    }

    let mut success = 0usize;
    for (role, bytes) in &uploads {
        let path = format!("previews/{}/{}.png", theme_id, role);
        let msg = format!("feat: add preview {} for {}", role, theme_id);
        if let Err(e) = gh
            .put_contents(owner, repo, branch, &path, bytes, &msg)
            .await
        {
            tracing::warn!("preview upload 失敗 ({}): {}", role, e);
            continue;
        }
        success += 1;
    }
    Ok(success > 0)
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
    fn sha256_hex_is_deterministic_64_chars() {
        let h1 = sha256_hex(b"hello");
        let h2 = sha256_hex(b"hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
