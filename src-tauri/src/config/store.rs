//! `ConfigManager`: 設定の読み書き (RwLock) と永続化 (atomic temp-write + rename)、
//! 破損時の `config.corrupt.{epoch}.json` 退避 (quarantine)、バックアップ一覧 / 復元。
//!
//! 型は `schema.rs`、v1→v2 変換は `migrate.rs`。`update` / `apply_patch` は
//! 「ディスク書込成功後に in-memory commit」の順序を守る (不整合を残さない)。

use super::schema::{AppConfig, BackupInfo};
use crate::errors::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// 設定ファイル書込時に使う temp 拡張子。同じディレクトリに `config.json` と
/// `config.json.tmp` が並ぶ形になり、電源断 / プロセス落ちで原子的置換が
/// 中断しても temp ファイルが残るのみで本ファイルは無傷。
const CONFIG_TEMP_SUFFIX: &str = "json.tmp";

/// 指定パスへファイル内容を atomic 的に書き込む。
///
/// `fs::write` 直書きは電源断 / プロセス落ちで対象ファイルが中途半端な
/// サイズ/内容になり、次回起動時の `serde_json::from_str` 失敗 → `config.corrupt.*`
/// 退避 → デフォルト復元 → ユーザー設定消失、という復旧不能な連鎖を起こす。
/// 本ヘルパーは temp ファイルへ書いてから `fs::rename` で 1 ステップで
/// 置換することで、ディスク側で常に「旧ファイル」か「新ファイル」のいずれかが
/// 完全に観測できる状態しか存在しなくなる。
///
/// 失敗時は temp ファイルを削除して元のパスを一切変更しない (呼び出し側が
/// in-memory ロールバックを判断する)。
pub(crate) fn atomic_write(path: &Path, content: &str) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::Config(format!(
            "設定ファイルの親ディレクトリが取得できません: {}",
            path.display()
        ))
    })?;
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    let temp = path.with_extension(CONFIG_TEMP_SUFFIX);
    // 前回クラッシュ時に temp が残っていても今回書込みで上書きする前に掃除する
    // (複数 temp が累積する事態を防ぐ)。
    let _ = fs::remove_file(&temp);

    fs::write(&temp, content).map_err(|e| {
        AppError::Config(format!(
            "設定ファイル temp への書き出し失敗 ({}): {}",
            crate::logging::redact_path(&temp),
            e
        ))
    })?;

    if let Err(e) = fs::rename(&temp, path) {
        // rename 失敗: temp が残骸として残らないよう掃除してからエラーを返す。
        let _ = fs::remove_file(&temp);
        return Err(AppError::Config(format!(
            "設定ファイル atomic 置換失敗 ({} → {}): {}",
            crate::logging::redact_path(&temp),
            crate::logging::redact_path(path),
            e
        )));
    }
    Ok(())
}

/// アプリケーション設定の管理を行うマネージャー
pub struct ConfigManager {
    /// 設定データ（スレッドセーフな読み書きロック）
    config: RwLock<AppConfig>,
    /// 設定ファイルのパス
    config_path: PathBuf,
}

/// `CUSTOM_CURSORS_DIR_OVERRIDE` を読む `ConfigManager::cursors_dir()` は
/// プロセス全体で env var を共有するため、override を使うテストはこの共有ロックで直列化する。
/// クレート横断 (`config::tests` / `commands::theme::tests` / `marketplace::tests` 等) の
/// 並走でも 1 つのミューテックスを共有することで env var の競合を防ぐ。
#[cfg(test)]
pub(crate) fn cursors_dir_override_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

impl ConfigManager {
    /// カーソル保存ディレクトリのパスを返す
    /// ~/.custom_cursors/
    ///
    /// `CUSTOM_CURSORS_DIR_OVERRIDE` が設定されている場合はそのパスで上書きする。
    pub fn cursors_dir() -> AppResult<PathBuf> {
        if let Ok(override_path) = std::env::var("CUSTOM_CURSORS_DIR_OVERRIDE") {
            return Ok(PathBuf::from(override_path));
        }
        let home = dirs::home_dir()
            .ok_or_else(|| AppError::Config("ホームディレクトリが見つかりません".to_string()))?;
        Ok(home.join(".custom_cursors"))
    }

    /// 設定ファイルのパスを返す
    /// %LOCALAPPDATA%/EasyCursorSwap/config.json
    fn config_file_path() -> AppResult<PathBuf> {
        let local_data = dirs::data_local_dir().ok_or_else(|| {
            AppError::Config("LocalAppData ディレクトリが見つかりません".to_string())
        })?;
        Ok(local_data.join("EasyCursorSwap").join("config.json"))
    }

    /// 設定マネージャーを初期化する。
    ///
    /// 動作:
    ///  1. ファイルなし → デフォルト設定で新規作成
    ///  2. ファイルあり → パース成功 → schema_version 比較
    ///     - 同じか古い → そのまま使用 (旧フィールド欠落は `serde(default)` で透過補填)
    ///     - 新しい → アプリ更新が必要 → エラー (`Config(...)` を返し、main 側で専用画面表示)
    ///  3. ファイルあり → パース失敗 → `config.corrupt.{ts}.json` に退避してデフォルトで再作成
    ///
    /// 書込みはどちらも [`atomic_write`] 経由で temp+rename するため、
    /// 電源断 / プロセス落ちで設定ファイルが中途半端な状態にならない。
    pub fn init() -> AppResult<Self> {
        let config_path = Self::config_file_path()?;
        let config = Self::load_or_initialize(&config_path)?;

        let cursors_dir = Self::cursors_dir()?;
        if !cursors_dir.exists() {
            fs::create_dir_all(&cursors_dir)?;
        }

        Ok(Self {
            config: RwLock::new(config),
            config_path,
        })
    }

    /// テスト専用コンストラクタ。本番コードは `init()` を使う。
    ///
    /// ロックや env var を経由せず、与えられたパスをそのまま Source of Truth として
    /// 初期化する。`() テストの tempdir ベースで実ファイルへの永続化を検証する
    /// 用途を想定 (atomic_write 経路の回帰検知)。
    #[cfg(test)]
    pub(crate) fn init_at(config_path: &Path) -> AppResult<Self> {
        let config = Self::load_or_initialize(config_path)?;
        Ok(Self {
            config: RwLock::new(config),
            config_path: config_path.to_path_buf(),
        })
    }

    /// 設定ファイルを読み込み、必要ならデフォルトを atomic 書きする共通処理。
    /// `init()` / `init_at()` 両方から呼ばれる。
    ///
    /// Wave 1A: 設定 JSON 読み込みは `Self::migrate(&content)` 経由に統一。
    /// v1 / v2 / 不正 の 3 ケースを `migrate` が判定し、v1 の場合は V2 へアップグレード +
    /// atomic_write で永続化する。これにより「通常起動」「`restore_config_backup`」
    /// 「`.cursorprofile` import」の 3 入力経路が同一の V1→V2 変換を共有する。
    fn load_or_initialize(config_path: &Path) -> AppResult<AppConfig> {
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            match Self::migrate(&content) {
                Ok((migrated, changed)) => {
                    if changed {
                        // v1 → v2 昇格が発生したので、永続化して次回からは v2 を読む。
                        atomic_write(config_path, &serde_json::to_string_pretty(&migrated)?)?;
                    }
                    Ok(migrated)
                }
                Err(e) => {
                    // パース失敗 → 退避して新規作成
                    Self::backup_corrupt(config_path, &content, &e.to_string())?;
                    let fresh = AppConfig::default();
                    atomic_write(config_path, &serde_json::to_string_pretty(&fresh)?)?;
                    tracing::warn!("設定ファイルが破損していたためデフォルトで再作成しました");
                    Ok(fresh)
                }
            }
        } else {
            let fresh = AppConfig::default();
            atomic_write(config_path, &serde_json::to_string_pretty(&fresh)?)?;
            Ok(fresh)
        }
    }

    /// 設定 JSON 文字列を V2 AppConfig へ変換する共通エントリ (実装は `migrate.rs`)。
    pub fn migrate(raw_json: &str) -> AppResult<(AppConfig, bool)> {
        super::migrate::migrate(raw_json)
    }

    /// パース不可な設定ファイルを `config.corrupt.{epoch}.json` に退避する。
    fn backup_corrupt(config_path: &Path, raw: &str, reason: &str) -> AppResult<()> {
        let ts = chrono::Utc::now().timestamp();
        let bak = config_path.with_file_name(format!("config.corrupt.{}.json", ts));
        fs::write(&bak, raw)?;
        tracing::error!(
            "設定ファイルが破損 ({}) → 退避: {}",
            reason,
            crate::logging::redact_path(&bak)
        );
        Ok(())
    }

    /// 現在の設定を取得する
    pub fn get(&self) -> AppResult<AppConfig> {
        let config = self
            .config
            .read()
            .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
        Ok(config.clone())
    }

    /// 設定を更新し、ディスクに永続化する
    ///
    /// 永続化は [`atomic_write`] 経由 (temp 書き出し + `fs::rename`) で原子的に
    /// 行う。書込みが失敗した場合は in-memory 状態もコミットせず、呼び出し側に
    /// エラーを返す。これにより「メモリ側は新状態、ディスクは旧/空」という
    /// 最悪の不整合を残さない。
    ///
    /// ロック獲得を `read` → クローン用 `draft` 作成 → `write` コミット、の
    /// 2 段にする理由は、rename 失敗時に `*config` へ書き戻す操作 (= 副作用)
    /// を「アトミック失敗」の中でする必要をなくすため。
    pub fn update<F>(&self, updater: F) -> AppResult<AppConfig>
    where
        F: FnOnce(&mut AppConfig),
    {
        // 1. 現在のスナップショットを取り、updater を適用した draft を作る
        //    (read lock 内で完結するため、副作用は発生しない)。
        let draft = {
            let guard = self
                .config
                .read()
                .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
            let mut draft = guard.clone();
            updater(&mut draft);
            draft
        };

        // 2. ディスクへ atomic 書き出し。失敗時は in-memory を一切変えない。
        let content = serde_json::to_string_pretty(&draft)?;
        atomic_write(&self.config_path, &content)?;

        // 3. 書込み成功を確認できたので、ようやく in-memory を commit。
        let mut guard = self
            .config
            .write()
            .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
        *guard = draft.clone();

        Ok(draft)
    }

    /// `update_config` IPC から呼ばれる typed-patch 適用の正準エントリ。
    ///
    /// `update<F>` クロージャ版と異なり、patch に存在しないフィールドは
    /// 一切触らない (`schema_version` / `github_account` / セキュリティ閾値 /
    /// `favorites` / `usage` / `active_theme_id` は patch に含まれないため、
    /// フロントから上書きされることは決してない)。
    ///
    /// ディスク永続化は `update<F>` と同じく atomic_write 経由。
    /// 失敗時は in-memory を変えない (Task 0 baseline で確認済みの安全網)。
    pub fn apply_patch(&self, patch: super::patch::AppConfigPatch) -> AppResult<AppConfig> {
        // 1. draft 作成 (read lock 内、副作用なし)
        let draft = {
            let guard = self
                .config
                .read()
                .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
            let mut draft = guard.clone();
            patch.apply_to(&mut draft);
            draft
        };

        // 2. ディスクへ atomic 書き出し
        let content = serde_json::to_string_pretty(&draft)?;
        atomic_write(&self.config_path, &content)?;

        // 3. in-memory commit
        let mut guard = self
            .config
            .write()
            .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
        *guard = draft.clone();
        Ok(draft)
    }

    /// 設定ディレクトリ内のバックアップファイル一覧を返す。
    ///
    /// 対象: `config.corrupt.*.json` (パースエラー時の退避ファイル)
    /// 返却: 最終更新日時の降順（最新が先頭）
    pub fn list_backups(&self) -> AppResult<Vec<BackupInfo>> {
        let dir = self
            .config_path
            .parent()
            .ok_or_else(|| AppError::Config("設定ディレクトリの取得に失敗".to_string()))?;

        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut backups: Vec<BackupInfo> = fs::read_dir(dir)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                if !(name.starts_with("config.corrupt.") && name.ends_with(".json")) {
                    return None;
                }
                let kind = "corrupt";
                let meta = entry.metadata().ok()?;
                let modified = meta.modified().ok()?;
                let secs = modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0)
                    .unwrap_or_default();
                Some(BackupInfo {
                    file_name: name,
                    modified_utc: dt.to_rfc3339(),
                    size_bytes: meta.len(),
                    kind: kind.to_string(),
                })
            })
            .collect();

        // 最新が先頭
        backups.sort_by(|a, b| b.modified_utc.cmp(&a.modified_utc));
        Ok(backups)
    }

    /// 指定したバックアップファイルを `config.json` に上書きして設定を再ロードする。
    ///
    /// セキュリティ: `file_name` は `config.corrupt.*.json` のみ許可。
    pub fn restore_backup(&self, file_name: &str) -> AppResult<()> {
        // ファイル名の簡易バリデーション (パストラバーサル防止)
        let valid = file_name.starts_with("config.corrupt.")
            && file_name.ends_with(".json")
            && !file_name.contains('/')
            && !file_name.contains('\\')
            && !file_name.contains("..");
        if !valid {
            return Err(AppError::Config(format!(
                "不正なバックアップファイル名: {}",
                file_name
            )));
        }

        let dir = self
            .config_path
            .parent()
            .ok_or_else(|| AppError::Config("設定ディレクトリの取得に失敗".to_string()))?;
        let backup_path = dir.join(file_name);

        if !backup_path.exists() {
            return Err(AppError::Config(format!(
                "バックアップファイルが見つかりません: {}",
                file_name
            )));
        }

        // バックアップを読み込んで有効な JSON (V1 / V2) か確認。
        // Wave 1A: `Self::migrate` 経由で V1 → V2 変換も許容する。
        // 古いバックアップ (v1 schema) もそのまま復元できるようになった。
        let content = fs::read_to_string(&backup_path)?;
        let (restored, _) = Self::migrate(&content)
            .map_err(|e| AppError::Config(format!("バックアップファイルが無効です: {}", e)))?;

        // config.json を atomic 書込み (temp → rename) で置換
        atomic_write(&self.config_path, &serde_json::to_string_pretty(&restored)?)?;

        // in-memory 更新
        let mut guard = self
            .config
            .write()
            .map_err(|e| AppError::Config(format!("設定のロックに失敗: {}", e)))?;
        *guard = restored;

        tracing::info!(
            "バックアップから復旧: {} → config.json",
            crate::logging::redact_path(&backup_path)
        );
        Ok(())
    }
}
