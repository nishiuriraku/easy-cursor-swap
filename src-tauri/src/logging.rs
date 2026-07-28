//! EasyCursorSwap ロギング初期化
//!
//! 仕様:
//!  - `%LOCALAPPDATA%\EasyCursorSwap\logs\app-YYYY-MM-DD.log` に日次ローテーション
//!  - 14 日経過したログを起動時に自動削除
//!  - 合計サイズが 100 MB を超えたら古いものから削除
//!  - PII 除外フィルター (絶対パス → 相対パス変換、レジストリ RAW 値ハッシュ化など)
//!  - リリース版は INFO 既定、`config.json` の `logging.level` で上書き
//!  - 起動後の `update_config` 経由でも `tracing-subscriber` の reload handle を
//!    使ってログレベルを差し替え可能 (Wave 2B / Task 4)。
//!
//! 標準出力にも出すかは `cfg!(debug_assertions)` で判定。

use crate::errors::{AppError, AppResult};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::reload;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// ログ保持日数 (この日数を超えたら自動削除)
const RETENTION_DAYS: u64 = 14;
/// ログ合計サイズ上限
const MAX_TOTAL_BYTES: u64 = 100 * 1024 * 1024;

/// ログディレクトリのパス: `%LOCALAPPDATA%\EasyCursorSwap\logs\`
pub fn log_dir() -> AppResult<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| AppError::Config("LocalAppData が取得できません".to_string()))?;
    Ok(base.join("EasyCursorSwap").join("logs"))
}

/// ログレベル文字列を検証し、`EnvFilter` に変換する (Wave 2B / Task 4)。
///
/// 受理する値 (大文字小文字どちらでも良い):
///   - "TRACE" / "DEBUG" / "INFO" / "WARN" / "ERROR" / "OFF"
///
/// 不明な値は `AppError::Config` を返す。`update_config` IPC はこのパーサを
/// 経由してログレベルを検証してから永続化するため、無効値での reload は
/// 発生しない (セキュリティ境界)。
pub fn validate_logging_level(level: &str) -> AppResult<EnvFilter> {
    let normalized = level.trim().to_ascii_uppercase();
    match normalized.as_str() {
        "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR" | "OFF" => {
            EnvFilter::try_new(normalized).map_err(|e| {
                AppError::Config(format!(
                    "ログレベル '{}' の EnvFilter 構築に失敗: {}",
                    level, e
                ))
            })
        }
        _ => Err(AppError::Config(format!(
            "未対応のログレベル: '{}' (TRACE/DEBUG/INFO/WARN/ERROR/OFF のいずれかを指定してください)",
            level
        ))),
    }
}

/// `EnvFilter` の reload handle。`init_logging` が INFO 既定でロードし、
/// `set_logging_level` が ConfigManager 初期化後と `update_config` 後に再ロードする。
type FilterHandle = reload::Handle<EnvFilter, tracing_subscriber::Registry>;

static RELOAD_HANDLE: OnceLock<Mutex<FilterHandle>> = OnceLock::new();

/// アクティブな `EnvFilter` を差し替える (Wave 2B / Task 4)。
///
/// `update_config` 経由の動的レベル変更、または ConfigManager 初期化後の
/// config 反映で利用。**初回 init 前に呼ぶとエラー** (handle 未登録)。
pub fn set_logging_level(level: &str) -> AppResult<()> {
    let new_filter = validate_logging_level(level)?;
    let mutex = RELOAD_HANDLE
        .get()
        .ok_or_else(|| AppError::Config("ロガーが未初期化です".to_string()))?;
    let handle = mutex
        .lock()
        .map_err(|e| AppError::Config(format!("ロガーリセットへの lock 失敗: {}", e)))?;
    handle
        .reload(new_filter.clone())
        .map_err(|e| AppError::Config(format!("ロガーリロード失敗: {}", e)))?;
    handle
        .reload(new_filter)
        .map_err(|e| AppError::Config(format!("ロガーリロード失敗: {}", e)))?;
    tracing::info!(level = %level, "logging level reloaded");
    Ok(())
}

/// `tracing` の初期化。返り値の `WorkerGuard` を main で保持する必要がある
/// (drop 時に未書き出しのバッファを flush するため)。
///
/// 起動時は渡された `level` (= 呼び出し側で INFO 固定) で EnvFilter を
/// 構築し、reload handle を静的変数に格納する。ConfigManager 初期化後に
/// `set_logging_level` を呼ぶとユーザ指定レベルへ切替わる。
pub fn init_logging(level: &str) -> AppResult<WorkerGuard> {
    let dir = log_dir()?;
    std::fs::create_dir_all(&dir)?;

    // 起動時クリーンアップ (失敗してもアプリは続行)
    if let Err(e) = cleanup_old_logs(&dir) {
        eprintln!("[logging] cleanup_old_logs warn: {}", e);
    }

    // 日次ローテーション
    let appender = tracing_appender::rolling::daily(&dir, "app");
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);

    // EnvFilter: 起動時の引数 (= 呼び出し側で INFO 固定) を使う。
    // 実際のユーザ設定レベルは ConfigManager 読込後に `set_logging_level` で適用する。
    let initial_filter = validate_logging_level(level).unwrap_or_else(|_| EnvFilter::new("info"));

    // reload layer で包む。後で handle 経由でフィルタを差し替え可能。
    let (filter_layer, filter_handle) = reload::Layer::new(initial_filter);

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .with_timer(fmt::time::SystemTime);

    let subscriber = tracing_subscriber::registry()
        .with(filter_layer)
        .with(file_layer);

    // デバッグビルドでは標準出力にも (色付き)
    #[cfg(debug_assertions)]
    {
        let console_layer = fmt::layer().with_target(false);
        subscriber.with(console_layer).init();
    }
    #[cfg(not(debug_assertions))]
    {
        subscriber.init();
    }

    // reload handle を静的 Mutex に格納 (1 度だけ)。
    let _ = RELOAD_HANDLE.set(Mutex::new(filter_handle));

    tracing::info!(
        log_dir = %crate::logging::redact_path(&dir),
        retention_days = RETENTION_DAYS,
        max_bytes = MAX_TOTAL_BYTES,
        "logging initialized"
    );

    Ok(guard)
}

/// 古いログファイルを削除する。
///
/// 削除条件:
///  1. 14 日以上前に最終更新されたファイル
///  2. 合計サイズが上限を超える場合は古いものから削除して上限以下にする
fn cleanup_old_logs(dir: &Path) -> AppResult<()> {
    if !dir.exists() {
        return Ok(());
    }

    let now = SystemTime::now();
    let retention = Duration::from_secs(RETENTION_DAYS * 24 * 60 * 60);

    // ファイル一覧を (path, mtime, size) で集める
    let mut files: Vec<(PathBuf, SystemTime, u64)> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        // app.log / app.log.YYYY-MM-DD パターンのみ対象
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !name.starts_with("app") {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime = meta.modified().unwrap_or(now);
        files.push((path, mtime, meta.len()));
    }

    // 1) 古いファイル削除
    files.retain(|(path, mtime, _)| {
        if let Ok(elapsed) = now.duration_since(*mtime) {
            if elapsed > retention {
                let _ = std::fs::remove_file(path);
                return false;
            }
        }
        true
    });

    // 2) 合計サイズ上限
    let total: u64 = files.iter().map(|(_, _, s)| *s).sum();
    if total > MAX_TOTAL_BYTES {
        files.sort_by_key(|(_, mtime, _)| *mtime);
        let mut current = total;
        for (path, _, size) in &files {
            if current <= MAX_TOTAL_BYTES {
                break;
            }
            if std::fs::remove_file(path).is_ok() {
                current = current.saturating_sub(*size);
            }
        }
    }

    Ok(())
}

/// ログ用にホームディレクトリ配下のパスを `~/...` に正規化する PII フィルター。
/// 各モジュールから直接呼び出して `tracing` に渡す前に処理する。
pub fn redact_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rel) = path.strip_prefix(&home) {
            return format!("~/{}", rel.display());
        }
    }
    path.display().to_string()
}

/// SHA-256 短縮 ID を返す (ハッシュ ログ用)。`bytes` の SHA-256 先頭 12 文字。
pub fn short_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))[..12].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_hash_returns_12_hex_chars() {
        let h = short_hash(b"hello world");
        assert_eq!(h.len(), 12);
        // hex のみ
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn short_hash_is_deterministic() {
        // 同一入力は常に同一出力
        assert_eq!(short_hash(b"foo"), short_hash(b"foo"));
        // 異なる入力は (高確率で) 異なる出力
        assert_ne!(short_hash(b"foo"), short_hash(b"bar"));
    }

    #[test]
    fn short_hash_known_sha256_prefix() {
        // SHA-256("") の先頭 12 文字 = "e3b0c44298fc"
        assert_eq!(short_hash(b""), "e3b0c44298fc");
        // SHA-256("abc") の先頭 12 文字 = "ba7816bf8f01"
        assert_eq!(short_hash(b"abc"), "ba7816bf8f01");
    }

    #[test]
    fn short_hash_handles_binary_data() {
        // バイナリも問題なく処理できる (UTF-8 縛りなし)
        let h = short_hash(&[0x00, 0xff, 0x80, 0x7f]);
        assert_eq!(h.len(), 12);
    }

    // ── Wave 2B / Task 4: ログレベル検証 ─────────────────────

    #[test]
    fn validate_logging_level_accepts_known_levels_case_insensitive() {
        for s in [
            "trace", "Trace", "DEBUG", "Info", "WARN", "error", "OFF", "off",
        ] {
            assert!(
                validate_logging_level(s).is_ok(),
                "level '{s}' は受理されるべき"
            );
        }
    }

    #[test]
    fn validate_logging_level_trims_whitespace() {
        assert!(validate_logging_level("  debug  ").is_ok());
    }

    #[test]
    fn validate_logging_level_rejects_unknown_levels() {
        for s in ["verbose", "warning", "infooo", "", "TRACEER", "0"] {
            assert!(
                validate_logging_level(s).is_err(),
                "level '{s}' は拒否されるべき"
            );
        }
    }
}
