//! Marketplace 自動提出フローの 9 ステージを実行するネットワークオーケストレータ。
//!
//! 責務分離:
//! - IPC ハンドラ (`submit_theme_auto` in `mod.rs`) は UUID パース / lineage ガード /
//!   pack build+sign / token load / client 構築までの **preflight** に専念する。
//! - 本モジュールは preflight で揃えた `SubmitPreflight` と `GithubGateway` を受け取り、
//!   `auth → fork → sync_fork → branch → upload_pack → upload_previews →
//!    upload_entry → open_pr` の順でステージを進める。
//!
//! 設計上の選択:
//! - `GithubGateway` は Rust 1.82 の async-in-trait を使った trait (dyn ではない)。
//!   `Client` を含む任意の impl をジェネリック引数として受け取る。
//! - 進捗通知は `ProgressSink` trait 経由で受け取る。`AppHandle` を直接渡さないので
//!   テストでは実 Tauri ランタイム無しで本パイプラインを駆動できる。
//! - `sync_fork_with_upstream` / `upload_previews_from_pack` は失敗しても致命ではない
//!   (= 提出全体を止めない) ため、結果を warn ログだけで継続する。

use crate::commands::marketplace_submit::ThemeMetaForSubmit;
use crate::config::DEFAULT_MAX_IMAGE_FILE_SIZE;
use crate::errors::{AppError, AppResult};
use crate::github::client::GithubGateway;
use crate::github::types::SubmitResult;
use std::collections::HashSet;
use std::io::Read;
use tauri::AppHandle;
use uuid::Uuid;

/// upstream の GitHub owner (Marketplace 公式インデックス)。
pub(crate) const UPSTREAM_OWNER: &str = "nishiuriraku";

/// upstream の GitHub repo (Marketplace 公式インデックス)。
pub(crate) const UPSTREAM_REPO: &str = "easy-cursor-swap-index";

/// 公式インデックスが受理するタグの allow-list。
///
/// Source of truth: `easy-cursor-swap-index/schemas/index-entry.json#tags.items.enum`
/// フロントエンド (`app/types/marketplace.ts` の `ALLOWED_MARKETPLACE_TAGS`) と同期すること。
pub(crate) const ALLOWED_MARKETPLACE_TAGS: &[&str] = &[
    "pixel", "minimal", "animated", "dark", "light", "anime", "retro", "neon",
];

/// preflight で確定した提出ペイロード。パイプラインの入力。
///
/// `cloned_from_marketplace_id` は公式インデックス由来テーマの再提出防止用。
/// IPC preflight が `load_metadata` から取り込んだ値をそのまま運ぶ。
/// テストでは任意に設定でき、本パイプラインは値が `Some` ならネットワーク呼び出しを
/// 一切行わずに拒否する。
#[allow(dead_code)] // theme_id は API symmetry のため保持 (派生処理でも参照可能)
pub(crate) struct SubmitPreflight {
    pub theme_id: Uuid,
    pub theme_id_str: String,
    pub tags: Vec<String>,
    pub pack_bytes: Vec<u8>,
    pub sha256: String,
    pub signature_b64: String,
    pub pubkey_id: String,
    pub meta: ThemeMetaForSubmit,
    pub cloned_from_marketplace_id: Option<Uuid>,
}

/// 進捗通知用の sink。
///
/// `AppHandle` を直接渡せない経路 (ユニットテスト等) でもパイプラインが動くように
/// interface を切っている。本番 IPC ハンドラは `AppHandle` を `&AppHandle` で渡す
/// ための adapter (`AppHandleSink`) を提供する。
pub(crate) trait ProgressSink {
    /// `submit:progress` イベント相当の stage 文字列を発行する。
    fn emit(&self, stage: &str);
}

/// `AppHandle` を `ProgressSink` として使うための薄い adapter。
pub(crate) struct AppHandleSink<'a> {
    app: &'a AppHandle,
}

impl<'a> AppHandleSink<'a> {
    pub(crate) fn new(app: &'a AppHandle) -> Self {
        Self { app }
    }
}

impl<'a> ProgressSink for AppHandleSink<'a> {
    fn emit(&self, stage: &str) {
        use tauri::Emitter;
        if let Err(e) = self.app.emit("submit:progress", stage) {
            tracing::warn!("submit:progress emit 失敗 ({}): {}", stage, e);
        }
    }
}

/// 9 ステージを実行して PR 作成まで完了させ、結果を返す。
///
/// ジェネリック境界で `S: ProgressSink` / `C: GithubGateway` を取り、
/// async-in-trait で dyn dispatch を避けて monomorphize する。
/// テストでは mockito ベースの `Client` または録音用 fake gateway を注入できる。
pub(crate) async fn run_submit_pipeline<S, C>(
    sink: &S,
    gateway: &C,
    preflight: SubmitPreflight,
) -> AppResult<SubmitResult>
where
    S: ProgressSink,
    C: GithubGateway,
{
    // ── lineage ガード ────────────────────────────────────────
    // 公式インデックス由来テーマ自体 (`source = Marketplace`) は SubmitThemeDialog の
    // 提出可能一覧から既に弾かれているが、ユーザーが `duplicate_theme` で複製してから
    // 提出してきた場合に備え、Rust 側でも `cloned_from_marketplace_id` を確認する。
    // 複製してから何段ネストしても `duplicate_theme` が origin を引き継ぐので、
    // この 1 か所のチェックだけで再提出経路を全て塞げる。
    // ネットワーク呼び出しよりも **先に** 検査する (テスト `lineage_rejected_before_gateway_calls` で固定)。
    if let Some(origin) = preflight.cloned_from_marketplace_id {
        tracing::warn!(
            "marketplace 由来テーマの再提出を拒否: origin_short={}",
            crate::logging::short_hash(origin.to_string().as_bytes())
        );
        return Err(AppError::Theme(
            "公式インデックス由来テーマを複製したものは再提出できません".to_string(),
        ));
    }

    // ── stage 1: auth ────────────────────────────────────────
    sink.emit("auth");
    let me = gateway.get_authenticated_user().await?;

    // ── stage 2: fork ────────────────────────────────────────
    sink.emit("fork");
    let fork = gateway.ensure_fork(UPSTREAM_OWNER, UPSTREAM_REPO).await?;
    // GitHub は「upstream owner 本人」が fork を作ろうとすると、新規 fork ではなく
    // upstream 本体 (= 同じ owner / 同じ repo) を返す。この場合 `merge-upstream` も
    // 「自分の main を自分の main に merge」になり 422 で落ちる。
    // 自前の fork でない (= upstream owner 本人) なら sync をスキップする。
    let is_self_owned = fork.owner.login == UPSTREAM_OWNER && fork.name == UPSTREAM_REPO;

    // ── stage 3: sync_fork ───────────────────────────────────
    sink.emit("sync_fork");
    if is_self_owned {
        tracing::info!("upstream owner 本人のため fork sync をスキップ");
    } else if let Err(e) = gateway
        .sync_fork_with_upstream(&fork.owner.login, &fork.name, &fork.default_branch)
        .await
    {
        // fork sync は失敗しても致命ではない (新規 fork なら upstream と一致しているはず)。
        tracing::warn!("fork sync 失敗 (続行): {}", e);
    }

    // ── stage 4: branch ──────────────────────────────────────
    let branch = format!("submit/{}", preflight.theme_id_str);
    sink.emit("branch");
    gateway
        .create_or_reset_branch(&fork.owner.login, &fork.name, &branch, &fork.default_branch)
        .await?;

    // ── stage 5: upload_pack ─────────────────────────────────
    sink.emit("upload_pack");
    gateway
        .put_contents(
            &fork.owner.login,
            &fork.name,
            &branch,
            &format!("themes/{}.cursorpack", preflight.theme_id_str),
            &preflight.pack_bytes,
            &format!("feat: add cursorpack for {}", preflight.theme_id_str),
        )
        .await?;

    // ── stage 6: upload_previews ─────────────────────────────
    sink.emit("upload_previews");
    // .cursorpack 内の `previews/<role>.png` を抽出し、
    // `previews/<theme_id>/<role>.png` として upstream に置けるように fork へアップロードする。
    // 失敗しても提出全体は止めず警告のみ (entry の preview_base_url は省略)。
    //
    // ZIP 展開は CPU 重め (= zip bomb / 巨大 entry decode) なので、抽出部のみ
    // `spawn_blocking` で逃がし async executor を占有しない。アップロード自体は
    // GitHub への HTTP 呼び出しなので async のまま (= tokio runtime を使う)。
    let uploaded_previews = upload_previews_from_pack(
        gateway,
        &fork.owner.login,
        &fork.name,
        &branch,
        &preflight.theme_id_str,
        preflight.pack_bytes.clone(),
    )
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("preview アップロード失敗 (続行): {}", e);
        false
    });

    // ── stage 7: upload_entry ────────────────────────────────
    sink.emit("upload_entry");
    let mut meta = preflight.meta.clone();
    // 提出ダイアログで入力された tags があれば metadata より優先する。
    // 空配列の場合は metadata の tags をそのまま使う。
    if !preflight.tags.is_empty() {
        meta.tags = preflight.tags.clone();
    }
    // セキュリティ不変条件: `meta.tags` はネットワーク送信前に必ず allow-list
    // を通すこと。`preflight.tags` (= ユーザー入力) は IPC preflight で検証済み
    // だが、`preflight.meta.tags` は theme.json のメタデータをそのまま反映するため
    // allow-list 作成前の古いテーマでも非許容タグを持ち得る。
    // 不正タグが混入したまま network write に到達すると、upstream index の
    // `schemas/index-entry.json#tags.items.enum` 検証で PR が落ちる (= token 浪費)。
    // 早期に拒否するため、最終的な `meta.tags` を再度 `validate_tags` に通す。
    if !meta.tags.is_empty() {
        meta.tags = validate_tags(meta.tags.clone())?;
    }
    let download_url = format!(
        "https://raw.githubusercontent.com/{}/{}/main/themes/{}.cursorpack",
        UPSTREAM_OWNER, UPSTREAM_REPO, preflight.theme_id_str
    );
    let preview_base_url = if uploaded_previews {
        Some(format!(
            "https://raw.githubusercontent.com/{}/{}/main/previews/{}",
            UPSTREAM_OWNER, UPSTREAM_REPO, preflight.theme_id_str
        ))
    } else {
        None
    };
    let entry_json = crate::commands::marketplace_submit::build_entry_json(
        &preflight.theme_id_str,
        &meta,
        &me.login,
        &preflight.pubkey_id,
        &preflight.sha256,
        &preflight.signature_b64,
        &download_url,
        preview_base_url.as_deref(),
    )?;
    gateway
        .put_contents(
            &fork.owner.login,
            &fork.name,
            &branch,
            &format!("entries/{}.json", preflight.theme_id_str),
            entry_json.as_bytes(),
            &format!("feat: add entry for {}", preflight.theme_id_str),
        )
        .await?;

    // ── stage 8: open_pr ──────────────────────────────────────
    sink.emit("open_pr");
    let head = format!("{}:{}", fork.owner.login, branch);
    let title = format!("submit: {} v{}", meta.display_name, meta.version);
    let body = crate::commands::marketplace_submit::render_pr_body(
        &preflight.theme_id_str,
        &meta.display_name,
        &meta.version,
        &me.login,
        &preflight.sha256,
        &preflight.signature_b64,
    );
    let pr = gateway
        .open_or_update_pr(UPSTREAM_OWNER, UPSTREAM_REPO, &head, "main", &title, &body)
        .await?;

    tracing::info!("Marketplace 自動提出完了: PR #{}", pr.number);
    Ok(SubmitResult {
        pr_url: pr.html_url,
        pr_number: pr.number,
    })
}

/// ユーザー入力の tags 文字列を allow-list で検証し、空白除去 / 重複排除した上で返す。
///
/// `submit_theme_auto` の IPC preflight からネットワーク呼び出しの前に呼ぶ。
/// `Ok(vec)` の長さは 0 のこともある (= 全要素が空 / 重複だった場合)。
pub(crate) fn validate_tags(tags: Vec<String>) -> AppResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(tags.len());
    for raw in tags {
        let t = raw.trim().to_string();
        if t.is_empty() {
            continue;
        }
        if !ALLOWED_MARKETPLACE_TAGS.contains(&t.as_str()) {
            return Err(AppError::Theme(format!(
                "未対応のマーケットプレイスタグ: {} (許可: {})",
                t,
                ALLOWED_MARKETPLACE_TAGS.join(", ")
            )));
        }
        if seen.insert(t.clone()) {
            out.push(t);
        }
    }
    Ok(out)
}

/// `.cursorpack` (ZIP) から `previews/<role>.png` を抽出し、
/// fork branch の `previews/<theme_id>/<role>.png` として upload する。
///
/// 戻り値は「1 件以上 PNG を upload できたか」。すべて失敗 / 該当ファイル無しの
/// 場合は `false` を返し、呼び出し側は entry の `preview_base_url` を省略する。
///
/// ZIP 展開は CPU 重め (= zip bomb / 巨大 entry decode) なので、抽出部のみ
/// `tokio::task::spawn_blocking` で逃がし async executor を占有しない。
/// アップロード自体は GitHub への HTTP 呼び出しなので async のまま
/// (= tokio runtime を使う) にする。
async fn upload_previews_from_pack<C: GithubGateway>(
    gateway: &C,
    owner: &str,
    repo: &str,
    branch: &str,
    theme_id: &str,
    pack_bytes: Vec<u8>,
) -> AppResult<bool> {
    // ── Phase 1: ZIP 展開 (blocking) ──
    // `pack_bytes` は zip decoder に消費されるので、抽出結果は所有権ごと渡される。
    let extracts: Vec<(String, Vec<u8>)> =
        tauri::async_runtime::spawn_blocking(move || extract_preview_pngs_from_pack(&pack_bytes))
            .await
            .map_err(|e| AppError::Theme(format!("preview extract join エラー: {}", e)))??;

    if extracts.is_empty() {
        tracing::warn!("preview PNG が cursorpack に含まれていません");
        return Ok(false);
    }

    // ── Phase 2: HTTP upload (async) ──
    let mut success = 0usize;
    for (role, bytes) in &extracts {
        let path = format!("previews/{}/{}.png", theme_id, role);
        let msg = format!("feat: add preview {} for {}", role, theme_id);
        if let Err(e) = gateway
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

/// `.cursorpack` 内の `previews/<role>.png` を同期処理で (role, bytes) のリストへ展開する。
///
/// 防御 (defense-in-depth):
/// - role 名: ASCII 英数字 + アンダースコアのみ (1〜32 文字)。
/// - 各 preview のサイズは `DEFAULT_MAX_IMAGE_FILE_SIZE` (10 MB) で上限カット。
///   zip の `entry.size()` は信頼できない (= zip bomb) ので `take(MAX+1)` で
///   実ストリーム長を読んで判定する。
fn extract_preview_pngs_from_pack(pack_bytes: &[u8]) -> AppResult<Vec<(String, Vec<u8>)>> {
    let reader = std::io::Cursor::new(pack_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| AppError::Theme(format!(".cursorpack ZIP オープン失敗: {}", e)))?;

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
        // `.take(MAX+1)` で上限超過を実ストリーム長で検出 (zip bomb 対策)。
        let mut limited = (&mut f).take(DEFAULT_MAX_IMAGE_FILE_SIZE + 1);
        let mut bytes = Vec::new();
        limited
            .read_to_end(&mut bytes)
            .map_err(|e| AppError::Theme(format!("preview 読込失敗: {}", e)))?;
        if (bytes.len() as u64) > DEFAULT_MAX_IMAGE_FILE_SIZE {
            // 巨大すぎる preview はスキップ (= 提出全体が止まらない / 既存の
            // 「warn ログだけで続行」soft-fail semantics を保つ)。
            tracing::warn!(
                "preview {} が上限 {} bytes を超過、スキップ",
                role,
                DEFAULT_MAX_IMAGE_FILE_SIZE
            );
            continue;
        }
        uploads.push((role.to_string(), bytes));
    }
    Ok(uploads)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::types::{AuthenticatedUser, PrRef, PullRequest, Repo, RepoOwner};
    use std::sync::Mutex;
    use std::sync::MutexGuard;

    /// テスト用 gateway。呼ばれたメソッド名 (`'static str`) を順に記録し、
    /// 決められた canned 値を返す。
    ///
    /// `calls` は `Mutex<Vec<&'static str>>` でラップしているため、
    /// `&self` の借用を `.await` をまたいで保持しても Send 境界を満たせる。
    struct RecordingGateway {
        calls: Mutex<Vec<&'static str>>,
    }

    impl RecordingGateway {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }

        fn lock_calls(&self) -> MutexGuard<'_, Vec<&'static str>> {
            self.calls.lock().expect("recording gateway mutex poisoned")
        }

        fn order(&self) -> Vec<&'static str> {
            self.lock_calls().clone()
        }

        fn record(&self, name: &'static str) {
            self.lock_calls().push(name);
        }
    }

    impl GithubGateway for RecordingGateway {
        async fn get_authenticated_user(&self) -> AppResult<AuthenticatedUser> {
            self.record("get_authenticated_user");
            Ok(AuthenticatedUser {
                login: "octocat".into(),
            })
        }

        async fn ensure_fork(&self, _owner: &str, _repo: &str) -> AppResult<Repo> {
            self.record("ensure_fork");
            Ok(Repo {
                name: UPSTREAM_REPO.into(),
                full_name: format!("octocat/{}", UPSTREAM_REPO),
                default_branch: "main".into(),
                owner: RepoOwner {
                    login: "octocat".into(),
                },
            })
        }

        async fn sync_fork_with_upstream(
            &self,
            _owner: &str,
            _repo: &str,
            _branch: &str,
        ) -> AppResult<()> {
            self.record("sync_fork_with_upstream");
            Ok(())
        }

        async fn create_or_reset_branch(
            &self,
            _owner: &str,
            _repo: &str,
            _branch: &str,
            _base: &str,
        ) -> AppResult<()> {
            self.record("create_or_reset_branch");
            Ok(())
        }

        async fn put_contents(
            &self,
            _owner: &str,
            _repo: &str,
            _branch: &str,
            _path: &str,
            _bytes: &[u8],
            _message: &str,
        ) -> AppResult<()> {
            self.record("put_contents");
            Ok(())
        }

        async fn open_or_update_pr(
            &self,
            _owner: &str,
            _repo: &str,
            _head: &str,
            _base: &str,
            _title: &str,
            _body: &str,
        ) -> AppResult<PullRequest> {
            self.record("open_or_update_pr");
            Ok(PullRequest {
                number: 42,
                html_url: format!(
                    "https://github.com/{}/{}/pull/42",
                    UPSTREAM_OWNER, UPSTREAM_REPO
                ),
                head: PrRef {
                    ref_name: "submit/abc".into(),
                },
            })
        }
    }

    /// 任意のメソッドが呼ばれたら Err を返す厳格な gateway。
    /// 「呼ばれてはいけない」経路を検証するために使う (= パニックではなく
    /// テスト失敗で扱いやすくする)。
    struct StrictGateway {
        calls: Mutex<Vec<&'static str>>,
    }

    impl StrictGateway {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
        fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    impl GithubGateway for StrictGateway {
        async fn get_authenticated_user(&self) -> AppResult<AuthenticatedUser> {
            self.calls.lock().unwrap().push("get_authenticated_user");
            Err(AppError::Theme(
                "StrictGateway: get_authenticated_user should not be called".into(),
            ))
        }
        async fn ensure_fork(&self, _: &str, _: &str) -> AppResult<Repo> {
            self.calls.lock().unwrap().push("ensure_fork");
            Err(AppError::Theme(
                "StrictGateway: ensure_fork should not be called".into(),
            ))
        }
        async fn sync_fork_with_upstream(&self, _: &str, _: &str, _: &str) -> AppResult<()> {
            self.calls.lock().unwrap().push("sync_fork_with_upstream");
            Err(AppError::Theme(
                "StrictGateway: sync_fork_with_upstream should not be called".into(),
            ))
        }
        async fn create_or_reset_branch(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
        ) -> AppResult<()> {
            self.calls.lock().unwrap().push("create_or_reset_branch");
            Err(AppError::Theme(
                "StrictGateway: create_or_reset_branch should not be called".into(),
            ))
        }
        async fn put_contents(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &[u8],
            _: &str,
        ) -> AppResult<()> {
            self.calls.lock().unwrap().push("put_contents");
            Err(AppError::Theme(
                "StrictGateway: put_contents should not be called".into(),
            ))
        }
        async fn open_or_update_pr(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
        ) -> AppResult<PullRequest> {
            self.calls.lock().unwrap().push("open_or_update_pr");
            Err(AppError::Theme(
                "StrictGateway: open_or_update_pr should not be called".into(),
            ))
        }
    }

    /// sync_fork だけ失敗してそれ以外は成功する gateway。
    /// 非 fatal であることを検証するために使う。
    struct SyncFailGateway {
        calls: Mutex<Vec<&'static str>>,
    }

    impl SyncFailGateway {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl GithubGateway for SyncFailGateway {
        async fn get_authenticated_user(&self) -> AppResult<AuthenticatedUser> {
            self.calls.lock().unwrap().push("get_authenticated_user");
            Ok(AuthenticatedUser {
                login: "octocat".into(),
            })
        }
        async fn ensure_fork(&self, _: &str, _: &str) -> AppResult<Repo> {
            self.calls.lock().unwrap().push("ensure_fork");
            Ok(Repo {
                name: UPSTREAM_REPO.into(),
                full_name: format!("octocat/{}", UPSTREAM_REPO),
                default_branch: "main".into(),
                owner: RepoOwner {
                    login: "octocat".into(),
                },
            })
        }
        async fn sync_fork_with_upstream(&self, _: &str, _: &str, _: &str) -> AppResult<()> {
            self.calls.lock().unwrap().push("sync_fork_with_upstream");
            Err(AppError::Theme("synthetic sync failure".into()))
        }
        async fn create_or_reset_branch(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
        ) -> AppResult<()> {
            self.calls.lock().unwrap().push("create_or_reset_branch");
            Ok(())
        }
        async fn put_contents(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &[u8],
            _: &str,
        ) -> AppResult<()> {
            self.calls.lock().unwrap().push("put_contents");
            Ok(())
        }
        async fn open_or_update_pr(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
        ) -> AppResult<PullRequest> {
            self.calls.lock().unwrap().push("open_or_update_pr");
            Ok(PullRequest {
                number: 7,
                html_url: format!(
                    "https://github.com/{}/{}/pull/7",
                    UPSTREAM_OWNER, UPSTREAM_REPO
                ),
                head: PrRef {
                    ref_name: "submit/abc".into(),
                },
            })
        }
    }

    /// テスト用 progress sink。何もしない。
    struct NoopSink;
    impl ProgressSink for NoopSink {
        fn emit(&self, _stage: &str) {}
    }

    /// recording progress sink。emit された stage を順番に記録する。
    struct RecordingSink {
        stages: Mutex<Vec<String>>,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self {
                stages: Mutex::new(Vec::new()),
            }
        }
        fn order(&self) -> Vec<String> {
            self.stages.lock().unwrap().clone()
        }
    }

    impl ProgressSink for RecordingSink {
        fn emit(&self, stage: &str) {
            self.stages.lock().unwrap().push(stage.to_string());
        }
    }

    /// テスト用 preflight を組み立てるヘルパー。
    fn make_preflight(theme_id: Uuid) -> SubmitPreflight {
        SubmitPreflight {
            theme_id,
            theme_id_str: theme_id.to_string(),
            tags: vec![],
            // 1 件の preview PNG を含む ZIP。
            pack_bytes: minimal_pack_with_preview_bytes(),
            sha256: "deadbeef".repeat(8),
            signature_b64: "AA==".to_string(),
            pubkey_id: "deadbeef".to_string(),
            meta: ThemeMetaForSubmit {
                name: crate::theme::LocalizedString::Simple("Test".into()),
                display_name: "Test".into(),
                author: None,
                version: "1.0.0".into(),
                included_roles: vec![],
                tags: vec![],
            },
            cloned_from_marketplace_id: None,
        }
    }

    /// `previews/Arrow.png` を 1 個だけ含む最小限の ZIP バイト列。
    ///
    /// `upload_previews_from_pack` の挙動 (= PNG を 1 件抽出して gateway を呼ぶ) を
    /// テスト内で再現できる最小データ。
    fn minimal_pack_with_preview_bytes() -> Vec<u8> {
        use std::io::Write;
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut zip = zip::ZipWriter::new(cursor);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("previews/Arrow.png", opts).unwrap();
        // 1x1 透明 PNG の固定バイト列。中身はテストでは参照しないので中身は固定で良い。
        let png = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // signature
            0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR length + name
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, // bit depth etc + crc
            0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, // IDAT
            0x54, 0x78, 0x9c, 0x62, 0x00, 0x01, 0x00, 0x00, // zlib stream
            0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, // adler32 etc
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, // IEND
            0x42, 0x60, 0x82,
        ];
        zip.write_all(&png).unwrap();
        let cursor = zip.finish().expect("zip finish");
        cursor.into_inner()
    }

    #[tokio::test]
    async fn pipeline_calls_stages_in_order() {
        let gw = RecordingGateway::new();
        let theme_id = Uuid::parse_str("00000000-0000-0000-0000-000000000abc").unwrap();
        let preflight = make_preflight(theme_id);

        let sink = RecordingSink::new();
        let result = run_submit_pipeline(&sink, &gw, preflight).await.unwrap();
        assert_eq!(result.pr_number, 42);
        assert_eq!(
            result.pr_url,
            format!(
                "https://github.com/{}/{}/pull/42",
                UPSTREAM_OWNER, UPSTREAM_REPO
            )
        );

        // 期待呼び出し順: auth → fork → sync_fork → branch →
        //                 put_contents (upload_pack) → put_contents (upload_previews) →
        //                 put_contents (upload_entry) → open_pr
        // upload_previews は previews/Arrow.png を 1 個含む pack から 1 件 put_contents を発行する。
        let order = gw.order();
        assert_eq!(
            order,
            vec![
                "get_authenticated_user",
                "ensure_fork",
                "sync_fork_with_upstream",
                "create_or_reset_branch",
                "put_contents", // upload_pack
                "put_contents", // upload_previews (1 PNG)
                "put_contents", // upload_entry
                "open_or_update_pr",
            ]
        );

        // 進捗イベントも同順で発火している。
        let expected_events: &[&str] = &[
            "auth",
            "fork",
            "sync_fork",
            "branch",
            "upload_pack",
            "upload_previews",
            "upload_entry",
            "open_pr",
        ];
        let expected_events_owned: Vec<String> =
            expected_events.iter().map(|s| s.to_string()).collect();
        assert_eq!(sink.order(), expected_events_owned);
    }

    #[tokio::test]
    async fn lineage_rejected_before_gateway_calls() {
        let gw = StrictGateway::new();
        let theme_id = Uuid::parse_str("00000000-0000-0000-0000-000000000abc").unwrap();
        let mut preflight = make_preflight(theme_id);
        // 公式インデックス由来テーマを複製したもの。
        preflight.cloned_from_marketplace_id = Some(theme_id);

        let sink = NoopSink;
        let result = run_submit_pipeline(&sink, &gw, preflight).await;
        assert!(matches!(result, Err(AppError::Theme(_))));
        // ネットワーク呼び出しは一切発生していない (= 0 件)。
        assert_eq!(
            gw.call_count(),
            0,
            "lineage 拒否より前に gateway は呼ばれてはいけない"
        );
    }

    #[tokio::test]
    async fn sync_fork_failure_does_not_abort() {
        let gw = SyncFailGateway::new();
        let theme_id = Uuid::parse_str("00000000-0000-0000-0000-000000000abc").unwrap();
        let preflight = make_preflight(theme_id);

        let sink = NoopSink;
        let result = run_submit_pipeline(&sink, &gw, preflight).await.unwrap();
        // sync_fork が失敗しても PR 作成まで到達している。
        assert_eq!(result.pr_number, 7);
        // (内部 Mutex<Vec> は private field のため直接読めない;
        //  動作上「Err を握り潰して続行」していることを result で確認する。)
    }

    #[tokio::test]
    async fn tag_allow_list_accepts_known_tags() {
        let tags = vec!["pixel".into(), "minimal".into(), "dark".into()];
        let out = validate_tags(tags).unwrap();
        assert_eq!(out, vec!["pixel", "minimal", "dark"]);
    }

    #[tokio::test]
    async fn tag_allow_list_rejects_unknown_tag() {
        let tags = vec!["unknown".into()];
        let err = validate_tags(tags).unwrap_err();
        match err {
            AppError::Theme(msg) => assert!(msg.contains("unknown")),
            other => panic!("expected Theme error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn tag_allow_list_dedupes_and_trims() {
        let tags = vec![" pixel ".into(), "pixel".into(), "  ".into(), "dark".into()];
        let out = validate_tags(tags).unwrap();
        assert_eq!(out, vec!["pixel", "dark"]);
    }

    #[tokio::test]
    async fn tag_allow_list_matches_frontend_allow_list() {
        // frontend `ALLOWED_MARKETPLACE_TAGS` (app/types/marketplace.ts) と
        // 同じ文字列集合を返していることを固定する。
        let expected: &[&str] = &[
            "pixel", "minimal", "animated", "dark", "light", "anime", "retro", "neon",
        ];
        assert_eq!(ALLOWED_MARKETPLACE_TAGS, expected);
    }

    /// Task 9 review finding #4: `preflight.tags` が空 (= ユーザー入力なし)
    /// のとき `meta.tags` (= theme.json の tags) が未検証のまま entry JSON に
    /// 入り、network write まで到達してはいけない。`upload_entry` 段階で
    /// `validate_tags` が再適用され、不正タグ混入時に `run_submit_pipeline` が
    /// `Err(AppError::Theme(_))` を返すことを固定する。
    #[tokio::test]
    async fn meta_tags_with_unknown_tag_rejected_before_network_write() {
        let gw = RecordingGateway::new();
        let theme_id = Uuid::parse_str("00000000-0000-0000-0000-000000000abc").unwrap();
        let mut preflight = make_preflight(theme_id);
        // ユーザー入力は空 (= IPC 側 allow-list を通っていない)。
        preflight.tags = vec![];
        // theme.json 側に不正タグ (allow-list 外) が混入しているケース。
        preflight.meta.tags = vec!["unknown_tag".into()];
        // preview pack 内の PNG はクリア (= upload_previews を素通りさせる)。
        preflight.pack_bytes = empty_pack_bytes();

        let sink = NoopSink;
        let result = run_submit_pipeline(&sink, &gw, preflight).await;
        match result {
            Err(AppError::Theme(msg)) => {
                assert!(
                    msg.contains("unknown_tag"),
                    "error should mention the bad tag, got: {msg}"
                );
            }
            other => panic!("expected Theme error, got {other:?}"),
        }
        // 不正タグ検出時点で gate: open_pr / upload_entry の put_contents は
        // 呼ばれていないこと (= network write に到達していない)。
        let order = gw.order();
        assert!(
            !order.contains(&"open_or_update_pr"),
            "open_pr should not have been called, but got order {order:?}"
        );
        assert!(
            !order
                .iter()
                .any(|c| *c == "put_contents" && order.iter().take_while(|x| *x != c).count() == 6),
            "upload_entry (3rd put_contents) should not have been called, got order {order:?}"
        );
    }

    /// Task 9 review finding #4: `meta.tags` が全て allow-list 内のとき
    /// (= 既存テーマで正常) は再検証を通過して PR 作成まで進む。
    #[tokio::test]
    async fn meta_tags_with_valid_tags_passes_through() {
        let gw = RecordingGateway::new();
        let theme_id = Uuid::parse_str("00000000-0000-0000-0000-000000000abc").unwrap();
        let mut preflight = make_preflight(theme_id);
        preflight.tags = vec![];
        preflight.meta.tags = vec!["pixel".into(), "dark".into()];
        preflight.pack_bytes = empty_pack_bytes();

        let sink = NoopSink;
        let result = run_submit_pipeline(&sink, &gw, preflight).await.unwrap();
        assert_eq!(result.pr_number, 42);
    }

    /// `previews/<role>.png` を含まない空 pack を返すヘルパー。
    fn empty_pack_bytes() -> Vec<u8> {
        use std::io::Write;
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut zip = zip::ZipWriter::new(cursor);
        let opts = zip::write::SimpleFileOptions::default();
        // 役割名だけ (中身は空) のディレクトリ相当エントリ。
        zip.start_file("manifest.json", opts).unwrap();
        zip.write_all(b"{}").unwrap();
        zip.finish().expect("zip finish").into_inner()
    }

    /// Task 9 review finding #6: 10 MB を超える preview は defense-in-depth
    /// としてスキップ (= 提出全体は止めない) され、サイズ上限内のものは抽出される。
    #[test]
    fn extract_preview_pngs_skips_oversize_entries() {
        use crate::config::DEFAULT_MAX_IMAGE_FILE_SIZE;
        use std::io::Write;
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut zip = zip::ZipWriter::new(cursor);
        let opts = zip::write::SimpleFileOptions::default();
        // 正常サイズ。
        zip.start_file("previews/Arrow.png", opts).unwrap();
        zip.write_all(b"OK").unwrap();
        // 上限 + 1 バイト (= スキップされるべき)。
        zip.start_file("previews/Big.png", opts).unwrap();
        let big = vec![0u8; (DEFAULT_MAX_IMAGE_FILE_SIZE + 1) as usize];
        zip.write_all(&big).unwrap();
        let pack_bytes = zip.finish().expect("zip finish").into_inner();

        let out = extract_preview_pngs_from_pack(&pack_bytes).unwrap();
        let roles: Vec<&str> = out.iter().map(|(r, _)| r.as_str()).collect();
        assert!(
            roles.contains(&"Arrow"),
            "正常サイズの preview は残るべき; got {roles:?}"
        );
        assert!(
            !roles.contains(&"Big"),
            "10MB 超の preview はスキップされるべき; got {roles:?}"
        );
        // 返した sizes 自体が上限内に収まっていることを確認。
        for (role, bytes) in &out {
            assert!(
                (bytes.len() as u64) <= DEFAULT_MAX_IMAGE_FILE_SIZE,
                "{role} のサイズが上限を超えている"
            );
        }
    }
}
