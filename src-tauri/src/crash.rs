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
//! - **送信は将来実装**: 現状は収集のみ。`easycursorswap/index` の専用
//!   エンドポイントが整備されたら `submit_pending_reports` を実装する。
//!
//! ## レコードサイズ
//!
//! クラッシュレポートが大量に溜まらないよう、`list_reports` は最新 50 件に
//! 制限し、`prune_old_reports` で 30 日経過レポートを削除する。

use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// レポート保持日数 (これより古いものは起動時に自動削除)
const RETENTION_DAYS: u64 = 30;
/// `list_reports` の最大返却件数 (UI に流す上限)
const LIST_LIMIT: usize = 50;

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
}
