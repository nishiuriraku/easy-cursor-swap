//! テーマ単体に対する CRUD 系 IPC。
//!
//! - 一覧 / プレビュー取得
//! - 適用 (active_theme_id を config に永続化)
//! - 削除 / 複製 / `.cursorpack` 再エクスポート
//! - `.cursorpack` の inspect / import
//!
//! .cur ビルドや署名フローは [`super::cursor_build`] (build / cancel / dto / sign / stream に分割済み)、
//! Windows スキーム連携は [`super::windows_scheme`] にある。

use crate::config::ConfigManager;
use crate::errors::AppError;
use crate::theme::{CursorpackInspection, RolePreview, ThemeManager, ThemeSummary};
use tauri::State;

/// テーマ一覧を取得する。
///
/// `is_active` は config の `active_theme_id` に加えてレジストリ実態を検証する。
/// Windows 側で別スキームに切り替えられたり、リセットされたりした場合は
/// 該当テーマの `is_active` を **false** にして返し、`config` 側の
/// `active_theme_id` もクリアする (Source of Truth はレジストリ)。
#[tauri::command]
pub fn get_themes(config: State<'_, ConfigManager>) -> Result<Vec<ThemeSummary>, AppError> {
    let cfg = config.get()?;
    let mut active_id = cfg.general.active_theme_id;

    // 実態と乖離していれば clear (例: ユーザーが Windows のマウスのプロパティで
    // 別スキームを選択 / 既定にリセットした直後)
    if let Some(id) = active_id {
        if !ThemeManager::theme_active_in_registry(id) {
            tracing::info!(
                "active_theme_id={} はレジストリ実態と一致しないためクリアします",
                id
            );
            clear_active_theme_id_with_warn(&config, "get_themes");
            active_id = None;
        }
    }

    ThemeManager::list_themes(active_id, &cfg.general.favorites, &cfg.general.usage)
}

/// テーマのお気に入りフラグを永続化する。
/// 戻り値は更新後のお気に入り ID リスト (UI でクライアントキャッシュを更新する用途)。
#[tauri::command]
pub fn set_theme_favorite(
    config: State<'_, ConfigManager>,
    theme_id: String,
    is_favorite: bool,
) -> Result<Vec<String>, AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    let updated = config.update(|c| {
        let already = c.general.favorites.contains(&id);
        if is_favorite && !already {
            c.general.favorites.push(id);
        } else if !is_favorite && already {
            c.general.favorites.retain(|x| x != &id);
        }
    })?;
    Ok(updated
        .general
        .favorites
        .iter()
        .map(|u| u.to_string())
        .collect())
}

/// 指定テーマのロール毎 PNG プレビューを返す。
///
/// `roles` が空配列なら全ロールを返す。値が指定されていればそのロールのみ。
/// レスポンスは `HashMap<role, PNG bytes>` で、IPC では `Vec<u8>` がそのまま JSON 配列化される。
#[tauri::command]
pub fn get_theme_previews(
    theme_id: String,
    roles: Vec<String>,
) -> Result<std::collections::HashMap<String, Vec<u8>>, AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    let filter: Option<&[String]> = if roles.is_empty() { None } else { Some(&roles) };
    ThemeManager::load_role_previews(id, filter)
}

/// [`get_theme_previews`] のリッチ版。各ロールに PNG + 寸法 + ホットスポット座標を返す。
///
/// テーマ詳細ドロワーで「ホットスポットの位置」を視覚化する用途のみ使用。
/// 旧 [`get_theme_previews`] はテーマカードのサムネ等で使い続ける (ペイロード軽量)。
#[tauri::command]
pub fn get_theme_role_previews(
    theme_id: String,
    roles: Vec<String>,
) -> Result<std::collections::HashMap<String, RolePreview>, AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    let filter: Option<&[String]> = if roles.is_empty() { None } else { Some(&roles) };
    ThemeManager::load_role_previews_with_hotspots(id, filter)
}

/// 指定 ID のテーマをシステムに適用する。
/// 失敗時は内部のスナップショットから自動ロールバックされる。
/// 成功時は config の `active_theme_id` と `usage` を更新して永続化する。
#[tauri::command]
pub fn apply_theme(config: State<'_, ConfigManager>, theme_id: String) -> Result<(), AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    ThemeManager::apply_theme(id)?;
    // 適用成功 → アクティブテーマ ID + 利用統計を永続化
    let now = chrono::Utc::now().to_rfc3339();
    config.update(|c| {
        c.general.active_theme_id = Some(id);
        let entry = c.general.usage.entry(id).or_default();
        entry.apply_count = entry.apply_count.saturating_add(1);
        entry.last_applied_at = Some(now);
    })?;
    Ok(())
}

/// `.cursorpack` をインポートする前のメタデータ検査。
/// 既存ライブラリに同 ID のテーマがあればバージョン比較情報を返す。
#[tauri::command]
pub fn inspect_cursorpack(path: String) -> Result<CursorpackInspection, AppError> {
    let buf = std::path::PathBuf::from(&path);
    if !buf.exists() {
        return Err(AppError::Theme(format!(
            "ファイルが見つかりません: {}",
            path
        )));
    }
    ThemeManager::inspect_cursorpack_file(&buf)
}

/// ローカルの `.cursorpack` ファイルをライブラリにインポートする。
/// パストラバーサル / Zip 爆弾 / シンボリックリンク防御つきで展開し、
/// 戻り値として展開後のテーマ ID (UUID 文字列) を返す。
#[tauri::command]
pub fn import_cursorpack(path: String) -> Result<String, AppError> {
    let buf = std::path::PathBuf::from(&path);
    if !buf.exists() {
        return Err(AppError::Theme(format!(
            "ファイルが見つかりません: {}",
            path
        )));
    }
    // 拡張子を弱バリデーション (Magic Byte は ThemeManager 内で再チェック)
    let ext_ok = buf
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("cursorpack"))
        .unwrap_or(false);
    if !ext_ok {
        return Err(AppError::Theme(
            ".cursorpack 以外の拡張子は受け入れません".to_string(),
        ));
    }
    let id = ThemeManager::import_cursorpack_file(&buf)?;
    Ok(id.to_string())
}

/// 指定 ID のテーマを ~/.custom_cursors/<UUID>/ ごと完全削除する。
///
/// 削除されたテーマがアクティブだった場合、呼び出し側 (UI) は config の
/// active_theme_id をクリアする責任を持つ。Windows 側はファイル不在時に
/// 既定カーソルへフォールバックするので追加処理は不要。
///
/// # エラー
/// レジストリで現在適用中のテーマを削除しようとした場合は `AppError::Theme` を返す。
#[tauri::command]
pub fn delete_theme(config: State<'_, ConfigManager>, theme_id: String) -> Result<(), AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    // 適用中のテーマは削除不可。UI 側でもボタンを disabled にしているが、
    // IPC 直叩き / 競合状態 / 別経路からの呼び出しに備えた IPC 層ガード。
    // Source of Truth はレジストリなので theme_active_in_registry を使う。
    if ThemeManager::theme_active_in_registry(id) {
        return Err(AppError::Theme(
            "適用中のテーマは削除できません。先に別のテーマを適用してから削除してください。"
                .to_string(),
        ));
    }
    ThemeManager::delete_theme(id)?;
    // 削除されたテーマが (active_in_registry でなくても) config 側に残っていれば掃除
    if let Ok(c) = config.get() {
        if c.general.active_theme_id == Some(id) {
            clear_active_theme_id_with_warn(&config, "delete_theme");
        }
    }
    Ok(())
}

/// 指定 ID のテーマを複製する。新テーマの UUID を返す。
#[tauri::command]
pub fn duplicate_theme(theme_id: String) -> Result<String, AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    let new_id = ThemeManager::duplicate_theme(id)?;
    Ok(new_id.to_string())
}

/// 既存ライブラリのテーマを `.cursorpack` ファイルに書き出す。
///
/// クリエイターを介さずライブラリ画面からそのままエクスポートできるよう、
/// `~/.custom_cursors/<UUID>/` を ZIP 化して指定パスに保存する。戻り値は
/// 書き込んだバイト数。
#[tauri::command]
pub fn repackage_theme(theme_id: String, output_path: String) -> Result<u64, AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    // marketplace 由来テーマは編集 / エクスポート不可。複製してから操作するよう促す。
    let metadata = ThemeManager::load_metadata(id)?;
    if metadata.source.is_marketplace() {
        return Err(AppError::Theme(
            "公式インデックスから取得したテーマはエクスポート・編集できません。複製してから操作してください。".to_string(),
        ));
    }
    let path = std::path::PathBuf::from(&output_path);
    ThemeManager::repackage_theme(id, &path)
}

/// `config.update` 失敗時の警告経路を一箇所に集約したヘルパー。
///
/// `get_themes` (レジストリ実態と乖離した `active_theme_id` のクリア) と
/// `delete_theme` (削除済みテーマ ID の掃除) から呼ばれる。config.json への
/// 永続化が失敗しても UI 操作は確定させる方針 (G4 の `update_config` と同じ) を
/// 採り、戻り値は `()` (UI には伝播しない)。失敗時は `tracing::warn!` で
/// context ラベル + 復旧パス (設定画面での別テーマ適用 / 再起動後のレジストリ
/// 実態からの自動再同期) をログに残し、運用での解消に備える。
fn clear_active_theme_id_with_warn(config: &ConfigManager, context: &str) {
    if let Err(err) = config.update(|c| c.general.active_theme_id = None) {
        tracing::warn!(
            "{}: active_theme_id クリア失敗 (config.json への保存がブロックされた可能性、\
             in-memory 状態はコミットされない)。復旧: 設定画面から別テーマを適用するか、\
             アプリ再起動後にレジストリ実態から自動再同期されます: {}",
            context,
            err
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{cursors_dir_override_lock, ConfigManager};
    use crate::theme::types::{LocalizedString, ThemeMetadata, ThemeSource};
    use std::collections::HashMap;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    /// 共有バッファに `tracing` 出力を書き出す `MakeWriter`。
    /// `Send + Sync` を満たすため `Arc<Mutex<Vec<u8>>>` で保持する。
    #[derive(Clone, Default)]
    struct LogCapture(Arc<Mutex<Vec<u8>>>);

    impl LogCapture {
        fn into_string(self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl io::Write for LogCapture {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogCapture {
        type Writer = LogCapture;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// WARN 以上のイベントを `LogCapture` に流して `f` を実行する。
    /// 戻り値はキャプチャした UTF-8 文字列。
    fn capture_warns<F: FnOnce()>(f: F) -> String {
        let capture = LogCapture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(capture.clone())
            .with_max_level(tracing::Level::WARN)
            .with_target(false)
            .without_time()
            .finish();
        tracing::subscriber::with_default(subscriber, f);
        capture.into_string()
    }

    /// プロセス ID + ナノ秒 nonce で衝突回避する test tempdir ヘルパー。
    /// 既存 `config.rs::tests::make_tempdir` と同方針 (グローバル env を触らない)。
    fn make_tempdir(label: &str) -> std::path::PathBuf {
        let pid = std::process::id();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ecs-theme-test-{}-{}-{}", label, pid, nonce));
        std::fs::create_dir_all(&dir).expect("tempdir 作成");
        dir
    }

    fn write_theme_with_source(dir: &std::path::Path, id: uuid::Uuid, source: ThemeSource) {
        let metadata = ThemeMetadata {
            schema_version: 1,
            id,
            name: LocalizedString::Simple("T".into()),
            version: "1.0.0".into(),
            created_at: "2026-05-14T00:00:00Z".into(),
            requires_os_shadow: false,
            cursors: HashMap::new(),
            author: None,
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
            tags: Vec::new(),
            source,
            cloned_from_marketplace_id: None,
        };
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("theme.json"),
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn marketplace_theme_cannot_be_repackaged() {
        let _guard = cursors_dir_override_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().unwrap();
        std::env::set_var("CUSTOM_CURSORS_DIR_OVERRIDE", temp.path());
        let id = uuid::Uuid::new_v4();
        let theme_dir = temp.path().join(id.to_string());
        write_theme_with_source(&theme_dir, id, ThemeSource::Marketplace);

        let out = temp.path().join("out.cursorpack");
        let result = repackage_theme(id.to_string(), out.to_string_lossy().to_string());
        std::env::remove_var("CUSTOM_CURSORS_DIR_OVERRIDE");
        assert!(
            result.is_err(),
            "marketplace テーマは repackage 不可: {result:?}"
        );
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("公式インデックス"),
            "エラーメッセージに公式インデックスが含まれるはず: {err_str}"
        );
    }

    #[test]
    fn local_theme_passes_marketplace_guard() {
        // local テーマは guard をパスする (cursors 空でも write_cursorpack_to_buffer は成功)
        let _guard = cursors_dir_override_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().unwrap();
        std::env::set_var("CUSTOM_CURSORS_DIR_OVERRIDE", temp.path());
        let id = uuid::Uuid::new_v4();
        let theme_dir = temp.path().join(id.to_string());
        write_theme_with_source(&theme_dir, id, ThemeSource::Local);

        let out = temp.path().join("out.cursorpack");
        let result = repackage_theme(id.to_string(), out.to_string_lossy().to_string());
        std::env::remove_var("CUSTOM_CURSORS_DIR_OVERRIDE");
        assert!(result.is_ok(), "local テーマは guard をパス: {result:?}");
    }

    // ===== G5: `let _ = config.update(...)` の silent failure を `tracing::warn!` 化する回帰テスト =====
    //
    // 元コード (`get_themes` line 35 / `delete_theme` line 184) はディスク書込み失敗時に
    // エラーを握り潰し、復旧の手がかりをログに残さなかった。`clear_active_theme_id_with_warn`
    // ヘルパーで警告 + 復旧パスを必ずログに出す方針に置き換えたので、以下を固定する:
    //
    //  1. ディスク失敗時に context ラベル + 復旧パスを含む WARN が出る
    //  2. 成功時は WARN を出さない (ノイズを増やさない)
    //  3. 成功時は in-memory の `active_theme_id` が None にクリアされる
    //
    // IPC コマンド (`get_themes` / `delete_theme`) は `State<'_, ConfigManager>` を要求するため
    // 直接呼びにくいが、内部で呼ぶ本ヘルパーを独立に検証すれば両 callsite の挙動が固定できる。

    /// `config.update` がディスク失敗する状況下では、context ラベル + 復旧パスを
    /// 含む WARN ログが出ることを確認する (silent failure 退行の検知)。
    #[test]
    fn clear_active_theme_id_with_warn_logs_failure_context() {
        let dir = make_tempdir("warn_fail");
        let path = dir.join("config.json");
        let mgr = ConfigManager::init_at(&path).expect("init_at");

        // init_at 後にターゲットをディレクトリ化して以降の update を rename 失敗させる
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();

        let logs = capture_warns(|| {
            clear_active_theme_id_with_warn(&mgr, "get_themes");
        });

        assert!(logs.contains("WARN"), "WARN レベルで出力されるべき: {logs}");
        assert!(
            logs.contains("get_themes"),
            "context ラベルが含まれるべき: {logs}"
        );
        assert!(
            logs.contains("active_theme_id クリア失敗"),
            "失敗理由の定型句が含まれるべき: {logs}"
        );
        assert!(logs.contains("復旧"), "復旧パスを含むべき: {logs}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 成功時に WARN を出さない (正常系のログノイズを増やさない) ことを確認する。
    #[test]
    fn clear_active_theme_id_with_warn_succeeds_silently() {
        let dir = make_tempdir("warn_ok");
        let path = dir.join("config.json");
        let mgr = ConfigManager::init_at(&path).expect("init_at");

        let logs = capture_warns(|| {
            clear_active_theme_id_with_warn(&mgr, "delete_theme");
        });

        assert!(
            !logs.contains("WARN") && !logs.contains("active_theme_id クリア失敗"),
            "成功時は WARN を出さないはず: {logs}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 成功時は in-memory の `active_theme_id` が None にクリアされることを確認する
    /// (元の `let _ = config.update(...)` が成立していた不変条件)。
    #[test]
    fn clear_active_theme_id_with_warn_clears_in_memory_state() {
        let dir = make_tempdir("clear_state");
        let path = dir.join("config.json");
        let mgr = ConfigManager::init_at(&path).expect("init_at");

        // 事前に active_theme_id をセット
        let target = uuid::Uuid::new_v4();
        mgr.update(|c| c.general.active_theme_id = Some(target))
            .expect("事前セット");
        assert_eq!(mgr.get().unwrap().general.active_theme_id, Some(target));

        clear_active_theme_id_with_warn(&mgr, "get_themes");

        assert!(
            mgr.get().unwrap().general.active_theme_id.is_none(),
            "成功時は in-memory の active_theme_id が None になるはず"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
