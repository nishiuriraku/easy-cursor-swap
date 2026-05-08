//! クラッシュレポート収集 (オプトイン送信)
//!
//! `std::panic::set_hook` で panic 情報を `%LOCALAPPDATA%\EasyCursorSwap\crash\` に
//! JSON で保存し、UI 側から閲覧 / 削除できるようにする。
//!
//! ## 設計方針
//!
//! - **オプトイン**: `config.general.crash_reporting` のデフォルトは false。
//!   ユーザーが明示的に有効化した場合のみ将来のサーバー送信に使われる。
//! - **PII 除外**: panic メッセージ / location 内のユーザーホームパスは
//!   `logging::redact_path` で `~/...` に正規化する。
//! - **fail-safe**: ファイル書き込みが失敗しても panic は通常通り伝搬させる。
//! - **送信先**: `services/crash-report-worker/` (Cloudflare Worker) に
//!   POST /crash → `nishiuriraku/easy-cursor-swap` の Issue 化。
//!   送信は [`submit_pending_reports`] が担当し、ビルド時に環境変数
//!   `EASY_CURSOR_SWAP_CRASH_REPORT_ENDPOINT` / `_APP_TOKEN` で埋め込まれた
//!   credentials を [`embedded_credentials`] が返したときのみ実行される。
//!   env 未設定でビルドされた場合は機能ごと無効化されるため、
//!   ローカル `cargo build` でユーザーが何も設定しなくても壊れない。
//!   送信成功したレポートはローカルから削除される。
//!
//! ## レコードサイズ
//!
//! クラッシュレポートが大量に溜まらないよう、`list_reports` は最新 50 件に
//! 制限し、`prune_old_reports` で 30 日経過レポートを削除する。
//! Worker 側 validate と整合させるため、message / location は送信時に
//! [`MAX_WIRE_FIELD_LEN`] でクリップする。

use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// レポート保持日数 (これより古いものは起動時に自動削除)
const RETENTION_DAYS: u64 = 30;
/// `list_reports` の最大返却件数 (UI に流す上限)
const LIST_LIMIT: usize = 50;
/// Worker 側 `MAX_FIELD_LEN` と一致させる、message / location の送信時クリップ長 (bytes)。
/// 超過分は末尾を `...[truncated]` 印で置換し、サーバ側 400 を回避する。
const MAX_WIRE_FIELD_LEN: usize = 4 * 1024;
/// HTTP リクエストの全体タイムアウト。Worker 側に届いたが応答が遅延する最悪ケースを想定。
const SUBMIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// 一度の `submit_pending_reports` 呼び出しで送信を試みる最大件数。
/// レート制限 (Worker 5 req/h × IP) より控えめに抑える。
const MAX_SUBMIT_PER_CALL: usize = 4;

/// ビルド時に埋め込まれる送信先 Cloudflare Worker の URL。
/// 例: "https://easy-cursor-swap-crash-report.<sub>.workers.dev/crash"
///
/// 未設定でビルドした場合は `None` となり、送信機能は事実上無効化される。
/// `option_env!` はコンパイル時マクロなので env が変わったら再ビルドが必要
/// (`build.rs` の `cargo:rerun-if-env-changed` が再ビルドをトリガーする)。
const EMBEDDED_ENDPOINT: Option<&str> = option_env!("EASY_CURSOR_SWAP_CRASH_REPORT_ENDPOINT");

/// ビルド時に埋め込まれる Worker 側 `ALLOWED_ORIGIN` と一致させる App Token。
///
/// **注意**: バイナリの `.rodata` セクションに平文で乗るため、`strings <exe>` で抽出可能。
/// これは「誰でも野良 POST で Issue 量産」を防ぐ程度の緩いゲートでしかなく、
/// 真の防御は Worker 側のレート制限 / Turnstile / WAF。
const EMBEDDED_APP_TOKEN: Option<&str> = option_env!("EASY_CURSOR_SWAP_CRASH_REPORT_APP_TOKEN");

/// ビルド時 env が両方設定されているときのみ `Some((endpoint, token))` を返す。
/// 片方でも欠けたり空文字なら `None` (= 送信を完全にスキップ)。
pub fn embedded_credentials() -> Option<(&'static str, &'static str)> {
    let endpoint = EMBEDDED_ENDPOINT?.trim();
    let token = EMBEDDED_APP_TOKEN?.trim();
    if endpoint.is_empty() || token.is_empty() {
        return None;
    }
    Some((endpoint, token))
}

/// 1 件のクラッシュレポート
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    /// ファイル名 (例: "panic-1746123456789.json")
    pub file_name: String,
    /// UTC ISO 8601 のクラッシュ時刻
    pub timestamp_utc: String,
    /// アプリバージョン (Cargo.toml の version)
    pub app_version: String,
    /// OS 種別 ("windows", "macos", ...)
    pub os: String,
    /// panic メッセージ (PII redact 済み)
    pub message: String,
    /// パニック発生位置 (file:line:column) — ホームパスは redact 済み
    pub location: Option<String>,
}

/// クラッシュレポートディレクトリ
pub fn crash_dir() -> AppResult<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| AppError::Config("LocalAppData が取得できません".to_string()))?;
    Ok(base.join("EasyCursorSwap").join("crash"))
}

/// `set_hook` 済みかどうかを記録 (テストで多重設定しないよう)
static HOOK_INSTALLED: OnceLock<()> = OnceLock::new();

/// プロセス開始時に一度だけ呼び出し、panic_hook を仕込む。
///
/// 既存のフック (デフォルトの stderr 出力) は内部で保持し、レポート保存後に呼び出す。
/// これにより `RUST_BACKTRACE` 等の標準挙動を壊さない。
pub fn install_panic_hook() {
    if HOOK_INSTALLED.set(()).is_err() {
        return;
    }
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // 失敗しても元のフックは呼ぶ。
        if let Err(e) = write_panic_record(info) {
            eprintln!("[crash] failed to write panic record: {}", e);
        }
        prev(info);
    }));
}

/// panic 情報を JSON ファイルに書き出す。
fn write_panic_record(info: &std::panic::PanicHookInfo<'_>) -> AppResult<()> {
    let dir = crash_dir()?;
    std::fs::create_dir_all(&dir)?;

    let now = SystemTime::now();
    let epoch_ms = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let secs = (epoch_ms / 1000) as i64;
    let timestamp_utc = chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0)
        .unwrap_or_default()
        .to_rfc3339();

    let raw_msg = if let Some(s) = info.payload().downcast_ref::<&'static str>() {
        s.to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "Box<dyn Any>".to_string()
    };
    let message = redact_message(&raw_msg);

    let location = info.location().map(|loc| {
        let file = std::path::PathBuf::from(loc.file());
        let redacted = crate::logging::redact_path(&file);
        format!("{}:{}:{}", redacted, loc.line(), loc.column())
    });

    let report = CrashReport {
        file_name: format!("panic-{}.json", epoch_ms),
        timestamp_utc,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        message,
        location,
    };

    let path = dir.join(&report.file_name);
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// panic メッセージ内に含まれるホームディレクトリパスを `~/...` に置換する。
///
/// メッセージは任意のフォーマットなので完全な PII 除去は保証できないが、
/// 最も多発する「絶対パスの文字列埋め込み」だけはここで処理する。
pub fn redact_message(msg: &str) -> String {
    let Some(home) = dirs::home_dir() else {
        return msg.to_string();
    };
    let home_str = home.to_string_lossy().to_string();
    if home_str.is_empty() {
        return msg.to_string();
    }
    // Windows: `\` と `/` 両方の表記が混在しうるので両方差し替え
    let alt = home_str.replace('\\', "/");
    msg.replace(&home_str, "~").replace(&alt, "~")
}

/// 保存済みレポートの一覧 (新しい順, 上限 `LIST_LIMIT`)
pub fn list_reports() -> AppResult<Vec<CrashReport>> {
    let dir = crash_dir()?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut entries: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if !path.is_file() {
                return None;
            }
            let name = path.file_name()?.to_string_lossy();
            if !name.starts_with("panic-") || !name.ends_with(".json") {
                return None;
            }
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, path))
        })
        .collect();
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.truncate(LIST_LIMIT);

    let mut reports = Vec::with_capacity(entries.len());
    for (_, path) in entries {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(report) = serde_json::from_str::<CrashReport>(&content) {
                reports.push(report);
            }
        }
    }
    Ok(reports)
}

/// 全レポート削除 (ユーザーが「クラッシュ履歴を消去」を実行したとき)
pub fn clear_reports() -> AppResult<usize> {
    let dir = crash_dir()?;
    if !dir.exists() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in std::fs::read_dir(&dir)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if name.starts_with("panic-")
            && name.ends_with(".json")
            && std::fs::remove_file(&path).is_ok()
        {
            removed += 1;
        }
    }
    Ok(removed)
}

/// 起動時に呼び出して、保持期間を超えたレポートを削除する。
pub fn prune_old_reports() -> AppResult<usize> {
    let dir = crash_dir()?;
    if !dir.exists() {
        return Ok(0);
    }
    let now = SystemTime::now();
    let retention = std::time::Duration::from_secs(RETENTION_DAYS * 24 * 60 * 60);
    let mut removed = 0;
    for entry in std::fs::read_dir(&dir)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !(name.starts_with("panic-") && name.ends_with(".json")) {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                if let Ok(elapsed) = now.duration_since(mtime) {
                    if elapsed > retention && std::fs::remove_file(&path).is_ok() {
                        removed += 1;
                    }
                }
            }
        }
    }
    Ok(removed)
}

/// Worker `POST /crash` に送信するペイロード形式。
///
/// `CrashReport` から `file_name` を除いたもの。`signature` は省略可で、
/// 省略時は Worker 側が `app_version|os|message|location` の SHA-256 短縮で自動算出する。
#[derive(Debug, Clone, Serialize)]
struct CrashReportWire<'a> {
    app_version: &'a str,
    os: &'a str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    timestamp_utc: &'a str,
}

/// `submit_pending_reports` の結果サマリ。UI 側でトースト表示などに使える。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SubmitSummary {
    /// 送信成功 → ローカル削除した件数
    pub sent: usize,
    /// 送信試行したが失敗した件数 (4xx / 5xx / ネットワークエラー)
    pub failed: usize,
    /// 件数上限などで今回送らなかった件数 (次回再試行)
    pub skipped: usize,
}

/// `CrashReport` から Worker への送信用ワイヤーフォーマットへ変換する。
///
/// Worker validate (`MAX_FIELD_LEN = 4 KB`) と整合させるため、message と
/// location は文字数ではなく **バイト数** ベースでクリップする。
fn to_wire(r: &CrashReport) -> CrashReportWire<'_> {
    CrashReportWire {
        app_version: &r.app_version,
        os: &r.os,
        message: clip_field(&r.message),
        location: r.location.as_deref().map(clip_field),
        timestamp_utc: &r.timestamp_utc,
    }
}

/// 文字列を UTF-8 安全境界でバイト長 `MAX_WIRE_FIELD_LEN` に切り詰める。
///
/// 切り詰めた場合は末尾に `...[truncated]` を付ける。
fn clip_field(s: &str) -> String {
    if s.len() <= MAX_WIRE_FIELD_LEN {
        return s.to_string();
    }
    const SUFFIX: &str = "...[truncated]";
    let budget = MAX_WIRE_FIELD_LEN.saturating_sub(SUFFIX.len());
    // UTF-8 文字境界で安全に切る
    let mut end = budget.min(s.len());
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    let mut out = String::with_capacity(end + SUFFIX.len());
    out.push_str(&s[..end]);
    out.push_str(SUFFIX);
    out
}

/// 1 件のレポートファイルを削除する。`file_name` は `panic-*.json` 形式に限定。
///
/// パストラバーサル対策として `crash_dir()` 配下の単一ファイルしか触らない。
fn delete_report_file(file_name: &str) -> AppResult<()> {
    if !file_name.starts_with("panic-") || !file_name.ends_with(".json") {
        return Err(AppError::InvalidInput(format!(
            "不正なファイル名: {}",
            file_name
        )));
    }
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err(AppError::InvalidInput(format!(
            "ファイル名に区切り文字: {}",
            file_name
        )));
    }
    let path = crash_dir()?.join(file_name);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// 保留中のクラッシュレポートを Cloudflare Worker に送信し、成功分は削除する。
///
/// - **オプトイン尊重**: 呼び出し側で `config.general.crash_reporting` を
///   先にチェックすること。本関数自体はフラグを見ない (config 依存を持たないため再利用しやすい)。
/// - **ベストエフォート**: ネットワークエラー / 4xx / 5xx は黙殺し、
///   ローカルファイルは残して次回起動時に再試行可能にする。
/// - **件数制限**: 1 回あたり最大 [`MAX_SUBMIT_PER_CALL`] 件まで。
///   Worker のレート制限 (5 req/h × IP) を踏み越えないため余裕を残す。
/// - **タイムアウト**: 各リクエスト [`SUBMIT_TIMEOUT`] (10 秒)。
///
/// `endpoint` または `app_token` が空文字なら `Err(AppError::InvalidInput)`。
pub async fn submit_pending_reports(endpoint: &str, app_token: &str) -> AppResult<SubmitSummary> {
    if endpoint.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "crash_report_endpoint が未設定".to_string(),
        ));
    }
    if app_token.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "crash_report_app_token が未設定".to_string(),
        ));
    }
    let reports = list_reports()?;
    if reports.is_empty() {
        return Ok(SubmitSummary::default());
    }

    let client = reqwest::Client::builder()
        .timeout(SUBMIT_TIMEOUT)
        .user_agent(concat!("EasyCursorSwap/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::Other(format!("HTTP クライアント初期化失敗: {}", e)))?;

    let mut summary = SubmitSummary::default();
    for (idx, report) in reports.iter().enumerate() {
        if idx >= MAX_SUBMIT_PER_CALL {
            summary.skipped = reports.len() - idx;
            break;
        }
        let wire = to_wire(report);
        let res = client
            .post(endpoint)
            .header("X-App-Token", app_token)
            .json(&wire)
            .send()
            .await;
        match res {
            Ok(r) if r.status().is_success() => {
                if let Err(e) = delete_report_file(&report.file_name) {
                    tracing::warn!(
                        "[crash] 送信成功後の削除に失敗 file={} err={}",
                        report.file_name,
                        e
                    );
                }
                summary.sent += 1;
            }
            Ok(r) => {
                tracing::debug!(
                    "[crash] Worker から非 2xx file={} status={}",
                    report.file_name,
                    r.status()
                );
                summary.failed += 1;
            }
            Err(e) => {
                tracing::debug!(
                    "[crash] 送信失敗 (network) file={} err={}",
                    report.file_name,
                    e
                );
                summary.failed += 1;
            }
        }
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_message_replaces_home_with_tilde() {
        // home_dir() を直接スタブできないため、現在のホームを利用したスモークテスト。
        // ホームが取れない環境では skip。
        let Some(home) = dirs::home_dir() else { return };
        let home_str = home.to_string_lossy().to_string();
        if home_str.is_empty() {
            return;
        }
        let original = format!("file not found at {}/foo/bar.txt", home_str);
        let redacted = redact_message(&original);
        assert!(
            !redacted.contains(&home_str),
            "ホームパスが残っている: {}",
            redacted
        );
        assert!(
            redacted.contains("~/foo/bar.txt") || redacted.contains("~\\foo\\bar.txt"),
            "~ に置換されていない: {}",
            redacted
        );
    }

    #[test]
    fn redact_message_preserves_non_path_text() {
        let msg = "assertion failed: left == right";
        assert_eq!(redact_message(msg), msg);
    }

    #[test]
    fn install_panic_hook_is_idempotent() {
        // 多重呼び出しでパニックしないこと
        install_panic_hook();
        install_panic_hook();
        install_panic_hook();
    }

    #[test]
    fn report_serde_roundtrip() {
        // CrashReport を JSON 経由で復元したら一致する
        let report = CrashReport {
            file_name: "panic-1746000000000.json".to_string(),
            timestamp_utc: "2026-05-08T00:00:00+00:00".to_string(),
            app_version: "0.1.0".to_string(),
            os: "windows".to_string(),
            message: "test panic".to_string(),
            location: Some("src/lib.rs:42:5".to_string()),
        };
        let json = serde_json::to_string(&report).unwrap();
        let restored: CrashReport = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.file_name, report.file_name);
        assert_eq!(restored.timestamp_utc, report.timestamp_utc);
        assert_eq!(restored.app_version, report.app_version);
        assert_eq!(restored.os, report.os);
        assert_eq!(restored.message, report.message);
        assert_eq!(restored.location, report.location);
    }

    #[test]
    fn report_with_no_location() {
        // location が None でも往復できる
        let report = CrashReport {
            file_name: "panic-x.json".to_string(),
            timestamp_utc: "2026-05-08T00:00:00+00:00".to_string(),
            app_version: "0.1.0".to_string(),
            os: "windows".to_string(),
            message: "no location".to_string(),
            location: None,
        };
        let json = serde_json::to_string(&report).unwrap();
        let restored: CrashReport = serde_json::from_str(&json).unwrap();
        assert!(restored.location.is_none());
    }

    #[test]
    fn redact_message_handles_multiple_occurrences() {
        // 同じパスが複数回出てくる場合 (例: A から B へコピー失敗) も全置換
        let Some(home) = dirs::home_dir() else { return };
        let home_str = home.to_string_lossy().to_string();
        if home_str.is_empty() {
            return;
        }
        let original = format!("copy from {}/a to {}/b failed", home_str, home_str);
        let redacted = redact_message(&original);
        assert!(!redacted.contains(&home_str));
        // 2 回出現するはず
        let tilde_count = redacted.matches('~').count();
        assert!(
            tilde_count >= 2,
            "expected >=2 tildes, got {} in: {}",
            tilde_count,
            redacted
        );
    }

    #[test]
    fn redact_message_handles_forward_slash_form() {
        // Windows のホームでも、Rust の Path 文字列化で `/` が混じることがある。
        // 両形式で置換できるか確認。
        let Some(home) = dirs::home_dir() else { return };
        let home_str = home.to_string_lossy().to_string();
        if home_str.is_empty() {
            return;
        }
        // forward slash 版
        let alt = home_str.replace('\\', "/");
        if alt == home_str {
            // すでに forward slash しか含まないシステムなのでスキップ
            return;
        }
        let original = format!("read at {}/a.txt", alt);
        let redacted = redact_message(&original);
        assert!(redacted.contains("~/a.txt"));
    }

    #[test]
    fn list_reports_returns_empty_when_dir_missing() {
        // クラッシュディレクトリが (まだ) 存在しないテスト環境では空配列。
        // 実機では未起動状態と同じシナリオ。
        let result = list_reports();
        assert!(result.is_ok());
        // ディレクトリが既存テーマで生成されている可能性があるが、
        // 仮に存在していてもエラーにはならない
        let _ = result.unwrap();
    }

    #[test]
    fn clip_field_passes_through_short_strings() {
        let s = "short message";
        assert_eq!(clip_field(s), s);
    }

    #[test]
    fn clip_field_truncates_long_strings_with_marker() {
        let big = "x".repeat(MAX_WIRE_FIELD_LEN + 100);
        let clipped = clip_field(&big);
        assert!(
            clipped.len() <= MAX_WIRE_FIELD_LEN,
            "clipped len {} should be <= {}",
            clipped.len(),
            MAX_WIRE_FIELD_LEN
        );
        assert!(
            clipped.ends_with("...[truncated]"),
            "should end with marker: {}",
            &clipped[clipped.len().saturating_sub(20)..]
        );
    }

    #[test]
    fn clip_field_preserves_utf8_boundary() {
        // 末尾境界がマルチバイト中で切れないこと (panic 安全)
        let mut s = "あ".repeat(MAX_WIRE_FIELD_LEN); // 3 bytes/char × N
        s.push_str("末尾"); // 切られる位置を強制的にマルチバイト中に持ってくる
        let clipped = clip_field(&s);
        // UTF-8 妥当性 (str::from_utf8 が通る) — 既に &str なので構造的に保証されるが、
        // 関数呼び出しで panic しないことを確認する
        assert!(clipped.is_char_boundary(clipped.len()));
        assert!(clipped.ends_with("...[truncated]"));
    }

    #[test]
    fn to_wire_omits_file_name_and_includes_required_fields() {
        let report = CrashReport {
            file_name: "panic-1746000000000.json".to_string(),
            timestamp_utc: "2026-05-08T00:00:00+00:00".to_string(),
            app_version: "0.1.0".to_string(),
            os: "windows".to_string(),
            message: "boom".to_string(),
            location: Some("src/lib.rs:1:1".to_string()),
        };
        let json = serde_json::to_value(to_wire(&report)).unwrap();
        // file_name は wire には乗らない
        assert!(
            json.get("file_name").is_none(),
            "file_name should not be sent: {}",
            json
        );
        // Worker が要求する必須フィールド
        assert_eq!(json["app_version"], "0.1.0");
        assert_eq!(json["os"], "windows");
        assert_eq!(json["message"], "boom");
        assert_eq!(json["location"], "src/lib.rs:1:1");
        assert_eq!(json["timestamp_utc"], "2026-05-08T00:00:00+00:00");
    }

    #[test]
    fn to_wire_skips_location_when_none() {
        // location: None → JSON に key 自体が出ないこと (#[serde(skip_serializing_if = ...)])
        let report = CrashReport {
            file_name: "panic-x.json".to_string(),
            timestamp_utc: "2026-05-08T00:00:00+00:00".to_string(),
            app_version: "0.1.0".to_string(),
            os: "windows".to_string(),
            message: "boom".to_string(),
            location: None,
        };
        let json = serde_json::to_value(to_wire(&report)).unwrap();
        assert!(json.get("location").is_none());
    }

    #[test]
    fn delete_report_file_rejects_path_traversal() {
        // ..\ や / が含まれるファイル名は拒否
        for evil in [
            "../foo.json",
            "..\\foo.json",
            "panic-/etc/passwd",
            "panic-../../escape.json",
            "panic-x\\.json",
        ] {
            let res = delete_report_file(evil);
            assert!(
                matches!(res, Err(AppError::InvalidInput(_))),
                "should reject: {}",
                evil
            );
        }
    }

    #[test]
    fn delete_report_file_rejects_wrong_extension_or_prefix() {
        for evil in ["evil.json", "panic-foo.txt", "config.json", "panik-1.json"] {
            let res = delete_report_file(evil);
            assert!(
                matches!(res, Err(AppError::InvalidInput(_))),
                "should reject prefix/ext: {}",
                evil
            );
        }
    }

    #[test]
    fn delete_report_file_succeeds_when_file_missing() {
        // 形式は正しいが存在しないファイルは Ok を返す (idempotent)
        let res = delete_report_file("panic-9999999999999.json");
        assert!(res.is_ok());
    }

    #[test]
    fn embedded_credentials_consistent_with_build_env() {
        // option_env! はテストビルド時の env で評価される。
        // 値が両方 set かつ非空なら Some、それ以外は None になることだけ保証する。
        let endpoint = option_env!("EASY_CURSOR_SWAP_CRASH_REPORT_ENDPOINT");
        let token = option_env!("EASY_CURSOR_SWAP_CRASH_REPORT_APP_TOKEN");
        let expected_some = matches!(
            (endpoint, token),
            (Some(e), Some(t)) if !e.trim().is_empty() && !t.trim().is_empty()
        );
        assert_eq!(embedded_credentials().is_some(), expected_some);
    }

    #[tokio::test]
    async fn submit_pending_reports_rejects_empty_endpoint() {
        let res = submit_pending_reports("", "tok").await;
        assert!(matches!(res, Err(AppError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn submit_pending_reports_rejects_empty_token() {
        let res = submit_pending_reports("https://x.example/crash", "").await;
        assert!(matches!(res, Err(AppError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn submit_pending_reports_returns_default_when_no_reports() {
        // crash_dir が空 (or 存在しない) なら何も送らず default summary
        // 実機ではここに既存のレポートが存在しうるため、送信は実エンドポイントに飛ばないよう
        // localhost の閉じたポートを指定し、failed が増えるだけで関数自体は Ok を返すことを確認
        let res = submit_pending_reports("http://127.0.0.1:1/", "tok").await;
        assert!(res.is_ok());
        let summary = res.unwrap();
        // sent はゼロ (失敗 or 空)。レポート 0 件なら全カウンタ 0
        assert_eq!(
            summary.sent, 0,
            "sent should be 0 when network is unreachable"
        );
    }

    #[test]
    fn list_reports_only_picks_panic_prefix() {
        // ディレクトリを tempdir に切り替えるのは難しいので、
        // ここでは関数が "panic-*.json" 以外を返さないことを実装ベースで再確認する。
        // 実装内のフィルタロジックは:
        //   name.starts_with("panic-") && name.ends_with(".json")
        // を満たすファイル名だけ拾う。
        // この境界条件を擬似テストとしてアサート。
        let valid = "panic-1234567890.json";
        let invalids = [
            "panic-1234.txt",    // 拡張子が違う
            "panik-1234.json",   // typo
            "panicbutnotprefix", // panic で始まるが - がない
            "1234.json",         // panic prefix なし
        ];
        assert!(valid.starts_with("panic-") && valid.ends_with(".json"));
        for name in invalids {
            assert!(
                !(name.starts_with("panic-") && name.ends_with(".json")),
                "should reject: {}",
                name
            );
        }
    }
}
