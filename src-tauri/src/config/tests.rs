//! `config` モジュールの単体テスト (schema 既定値 / JSON round-trip / migrate / atomic_write / update / backup).
use super::store::atomic_write;
use std::fs;
use std::path::PathBuf;

use super::*;

#[test]
fn default_uses_current_schema_version() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(cfg.schema_version, 2);
}

#[test]
fn default_has_sane_security_thresholds() {
    // 仕様書 §「セキュリティ」: 50/200/10/1024 MB の固定上限
    let cfg = AppConfig::default();
    assert_eq!(cfg.security.max_pack_compressed_size, 50 * 1024 * 1024);
    assert_eq!(cfg.security.max_pack_uncompressed_size, 200 * 1024 * 1024);
    assert_eq!(cfg.security.max_image_file_size, 10 * 1024 * 1024);
    assert_eq!(cfg.security.storage_warning_threshold, 1024 * 1024 * 1024);
}

#[test]
fn default_panic_hotkey_is_ctrl_alt_shift_r() {
    // パニックボタンの既定は仕様で固定。ユーザーが変更する前は必ずこの値。
    let cfg = AppConfig::default();
    assert_eq!(cfg.general.panic_hotkey, "Ctrl+Alt+Shift+R");
}

#[test]
fn default_crash_reporting_is_opt_in() {
    // プライバシー優先で既定は false。ユーザーが明示 ON にしないと送信しない。
    let cfg = AppConfig::default();
    assert!(!cfg.general.crash_reporting);
}

#[test]
fn json_roundtrip_preserves_all_fields() {
    // serde で書き出して読み戻して同一になることを確認する。
    let original = AppConfig::default();
    let json = serde_json::to_string(&original).unwrap();
    let restored: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.schema_version, original.schema_version);
    assert_eq!(restored.general.auto_start, original.general.auto_start);
    assert_eq!(restored.general.language, original.general.language);
    assert_eq!(restored.general.panic_hotkey, original.general.panic_hotkey);
    assert_eq!(
        restored.general.crash_reporting,
        original.general.crash_reporting
    );
    assert_eq!(
        restored.security.max_pack_compressed_size,
        original.security.max_pack_compressed_size
    );
    assert_eq!(
        restored.logging.retention_days,
        original.logging.retention_days
    );
}

#[test]
fn deserialize_accepts_missing_crash_reporting() {
    // 旧スキーマ (crash_reporting が無い) からの後方互換: serde(default) で false。
    // JSON 中に残っている "dark_mode" ブロックは新スキーマでは未知フィールドとなり、
    // serde の既定挙動 (deny_unknown_fields 未指定) で読み飛ばされる。
    // これにより既存ユーザーの config.json から dark_mode キーが自然消滅する
    // ソフトマイグレーションが壊れていないことを兼ねて検証している。
    let json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R"
            },
            "dark_mode": {
                "enabled": false,
                "light_theme_id": null,
                "dark_theme_id": null
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
    let cfg: AppConfig = serde_json::from_str(json).expect("legacy schema should parse");
    assert!(!cfg.general.crash_reporting);
    assert_eq!(cfg.general.language, "ja");
}

#[test]
fn deserialize_accepts_missing_favorites_and_usage() {
    // 旧スキーマ (favorites / usage が無い) は serde(default) で空コレクションになる。
    // dark_mode は新スキーマで削除済だが、旧 config.json には残っている。
    // serde が未知フィールドを読み飛ばすことで透過マイグレーションする。
    let json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false
            },
            "dark_mode": {
                "enabled": false,
                "light_theme_id": null,
                "dark_theme_id": null
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
    let cfg: AppConfig = serde_json::from_str(json).expect("legacy schema should parse");
    assert!(cfg.general.favorites.is_empty());
    assert!(cfg.general.usage.is_empty());
}

#[test]
fn default_favorites_and_usage_are_empty() {
    let cfg = AppConfig::default();
    assert!(cfg.general.favorites.is_empty());
    assert!(cfg.general.usage.is_empty());
}

#[test]
fn deserialize_accepts_missing_github_account() {
    // v1 config (github_account 欠落) は serde(default) で None になる。
    let json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
    let cfg: AppConfig = serde_json::from_str(json).expect("legacy schema should parse");
    assert!(cfg.github_account.is_none());
}

#[test]
fn github_account_round_trips_through_json() {
    // struct update 構文で clippy::field_reassign_with_default を回避する。
    let cfg = AppConfig {
        github_account: Some(GithubAccount {
            login: "octocat".to_string(),
            token_saved_at: "2026-05-14T12:00:00Z".to_string(),
        }),
        ..AppConfig::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let back: AppConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.github_account.as_ref().unwrap().login, "octocat");
    assert_eq!(
        back.github_account.as_ref().unwrap().token_saved_at,
        "2026-05-14T12:00:00Z"
    );
}

#[test]
fn default_schema_version_is_current() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.schema_version, super::CURRENT_SCHEMA_VERSION);
    assert_eq!(cfg.schema_version, 2);
}

// ── Wave 2B / Task 3: typed-patch apply テスト ───────────────────

/// `apply_patch` 経由で `general.show_apply_toast` を変更できる。
#[test]
fn apply_patch_changes_general_field() {
    let dir = make_tempdir("patch-1");
    let path = dir.join("config.json");
    let cm = ConfigManager::init_at(&path).unwrap();
    assert!(cm.get().unwrap().general.show_apply_toast);

    let patch = patch::AppConfigPatch {
        general: Some(patch::GeneralConfigPatch {
            show_apply_toast: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = cm.apply_patch(patch).unwrap();
    assert!(!updated.general.show_apply_toast);
    let _ = fs::remove_dir_all(&dir);
}

// ── Wave 3B / AR-M1-4: settings 6 フィールド永続化の回帰テスト ──
//
// 既存の `apply_patch_changes_general_field` は `show_apply_toast` 1 フィールド
// のみカバーしていた。残 5 フィールド (`apply_shadow_control` /
// `start_minimized` / `show_storage_warning` / `require_signed_themes` /
// `warn_unsigned_import`) も個別に変更が永続化されることを固定する。
// これにより「`update_config` IPC 経由で設定値が反映されない」リグレッションを
// 検知可能にする (= AR-M1-4 が恒久的に閉じていることの証拠)。

/// `apply_patch` 経由で `general.apply_shadow_control` を変更できる。
#[test]
fn apply_patch_changes_apply_shadow_control() {
    let dir = make_tempdir("patch-apply-shadow");
    let path = dir.join("config.json");
    let cm = ConfigManager::init_at(&path).unwrap();
    assert!(cm.get().unwrap().general.apply_shadow_control);

    let patch = patch::AppConfigPatch {
        general: Some(patch::GeneralConfigPatch {
            apply_shadow_control: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = cm.apply_patch(patch).unwrap();
    assert!(!updated.general.apply_shadow_control);
    let _ = fs::remove_dir_all(&dir);
}

/// `apply_patch` 経由で `general.start_minimized` を変更できる。
/// デフォルト false → true の変更を検証 (true → false の片方向だけでは
/// 型 bool のトグルが機能している確証にならないため)。
#[test]
fn apply_patch_changes_start_minimized() {
    let dir = make_tempdir("patch-start-minimized");
    let path = dir.join("config.json");
    let cm = ConfigManager::init_at(&path).unwrap();
    assert!(!cm.get().unwrap().general.start_minimized);

    let patch = patch::AppConfigPatch {
        general: Some(patch::GeneralConfigPatch {
            start_minimized: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = cm.apply_patch(patch).unwrap();
    assert!(updated.general.start_minimized);

    // 別 ConfigManager で再ロードしても同じ (= ディスク永続化)
    let reloaded = ConfigManager::init_at(&path).unwrap();
    assert!(reloaded.get().unwrap().general.start_minimized);
    let _ = fs::remove_dir_all(&dir);
}

/// `apply_patch` 経由で `general.show_storage_warning` を変更できる。
#[test]
fn apply_patch_changes_show_storage_warning() {
    let dir = make_tempdir("patch-storage-warn");
    let path = dir.join("config.json");
    let cm = ConfigManager::init_at(&path).unwrap();
    assert!(cm.get().unwrap().general.show_storage_warning);

    let patch = patch::AppConfigPatch {
        general: Some(patch::GeneralConfigPatch {
            show_storage_warning: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = cm.apply_patch(patch).unwrap();
    assert!(!updated.general.show_storage_warning);
    let _ = fs::remove_dir_all(&dir);
}

/// `apply_patch` 経由で `security.require_signed_themes` を変更できる。
/// デフォルト false → true の変更を検証。
#[test]
fn apply_patch_changes_require_signed_themes() {
    let dir = make_tempdir("patch-require-signed");
    let path = dir.join("config.json");
    let cm = ConfigManager::init_at(&path).unwrap();
    assert!(!cm.get().unwrap().security.require_signed_themes);

    let patch = patch::AppConfigPatch {
        security: Some(patch::SecurityConfigPatch {
            require_signed_themes: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = cm.apply_patch(patch).unwrap();
    assert!(updated.security.require_signed_themes);

    // 別 ConfigManager で再ロードしても同じ (= ディスク永続化)
    let reloaded = ConfigManager::init_at(&path).unwrap();
    assert!(reloaded.get().unwrap().security.require_signed_themes);
    let _ = fs::remove_dir_all(&dir);
}

/// `apply_patch` 経由で `security.warn_unsigned_import` を変更できる。
#[test]
fn apply_patch_changes_warn_unsigned_import() {
    let dir = make_tempdir("patch-warn-unsigned");
    let path = dir.join("config.json");
    let cm = ConfigManager::init_at(&path).unwrap();
    assert!(cm.get().unwrap().security.warn_unsigned_import);

    let patch = patch::AppConfigPatch {
        security: Some(patch::SecurityConfigPatch {
            warn_unsigned_import: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = cm.apply_patch(patch).unwrap();
    assert!(!updated.security.warn_unsigned_import);
    let _ = fs::remove_dir_all(&dir);
}

/// 6 フィールドを同時に変更できることの確認 (= 一括 patch 適用の原子性)。
#[test]
fn apply_patch_changes_all_six_settings_at_once() {
    let dir = make_tempdir("patch-all-six");
    let path = dir.join("config.json");
    let cm = ConfigManager::init_at(&path).unwrap();
    let baseline = cm.get().unwrap();
    // baseline と逆方向の値を patch に設定 (false→true / true→false)
    let patch = patch::AppConfigPatch {
        general: Some(patch::GeneralConfigPatch {
            show_apply_toast: Some(!baseline.general.show_apply_toast),
            apply_shadow_control: Some(!baseline.general.apply_shadow_control),
            start_minimized: Some(!baseline.general.start_minimized),
            show_storage_warning: Some(!baseline.general.show_storage_warning),
            ..Default::default()
        }),
        // SecurityConfigPatch は 2 フィールドしか持たないため、両方を
        // 明示代入したら `..Default::default()` は冗長 (clippy::needless_update)。
        security: Some(patch::SecurityConfigPatch {
            require_signed_themes: Some(!baseline.security.require_signed_themes),
            warn_unsigned_import: Some(!baseline.security.warn_unsigned_import),
        }),
        ..Default::default()
    };
    let updated = cm.apply_patch(patch).unwrap();
    assert_eq!(
        updated.general.show_apply_toast,
        !baseline.general.show_apply_toast
    );
    assert_eq!(
        updated.general.apply_shadow_control,
        !baseline.general.apply_shadow_control
    );
    assert_eq!(
        updated.general.start_minimized,
        !baseline.general.start_minimized
    );
    assert_eq!(
        updated.general.show_storage_warning,
        !baseline.general.show_storage_warning
    );
    assert_eq!(
        updated.security.require_signed_themes,
        !baseline.security.require_signed_themes
    );
    assert_eq!(
        updated.security.warn_unsigned_import,
        !baseline.security.warn_unsigned_import
    );
    let _ = fs::remove_dir_all(&dir);
}

/// P10: 既定では未完了 (0)。
#[test]
fn default_onboarding_version_is_zero() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.general.onboarding_version, 0);
    assert!(cfg.general.onboarding_version < ONBOARDING_CURRENT_VERSION);
}

/// P10: 旧 v2 JSON (フィールド無し) は 0 で読める (schema_version 不変)。
#[test]
fn missing_onboarding_version_deserializes_as_zero() {
    let mut json = serde_json::to_value(AppConfig::default()).unwrap();
    json["general"]
        .as_object_mut()
        .unwrap()
        .remove("onboarding_version");
    let cfg: AppConfig = serde_json::from_value(json).unwrap();
    assert_eq!(cfg.general.onboarding_version, 0);
    assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
}

/// P10: patch 経由で完了バージョンを書き込める / 0 に戻せる。
#[test]
fn apply_patch_sets_onboarding_version() {
    let dir = make_tempdir("patch-onboarding");
    let path = dir.join("config.json");
    let cm = ConfigManager::init_at(&path).unwrap();
    let patch = patch::AppConfigPatch {
        general: Some(patch::GeneralConfigPatch {
            onboarding_version: Some(ONBOARDING_CURRENT_VERSION),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(cm.apply_patch(patch).unwrap().general.onboarding_version, 1);
    let reset = patch::AppConfigPatch {
        general: Some(patch::GeneralConfigPatch {
            onboarding_version: Some(0),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(cm.apply_patch(reset).unwrap().general.onboarding_version, 0);
    let _ = fs::remove_dir_all(&dir);
}

/// `apply_patch` は patch に含まれないフィールドを一切変更しない
/// (e.g. `github_account` を patch 経由で送ろうとしても無視される)。
///
/// serde unknown field 拒否 + 型が patch に存在しない二重防御。
#[test]
fn apply_patch_does_not_touch_github_account_or_schema_version() {
    let dir = make_tempdir("patch-2");
    let path = dir.join("config.json");
    let cm = ConfigManager::init_at(&path).unwrap();
    let baseline_schema = cm.get().unwrap().schema_version;
    let baseline_github_login = cm
        .get()
        .unwrap()
        .github_account
        .as_ref()
        .map(|g| g.login.clone());

    let patch = patch::AppConfigPatch {
        logging: Some(patch::LoggingConfigPatch {
            level: Some("DEBUG".to_string()),
        }),
        ..Default::default()
    };
    let updated = cm.apply_patch(patch).unwrap();
    assert_eq!(updated.schema_version, baseline_schema);
    assert_eq!(
        updated.github_account.as_ref().map(|g| g.login.clone()),
        baseline_github_login
    );
    assert_eq!(updated.logging.level, "DEBUG");
    let _ = fs::remove_dir_all(&dir);
}

/// `general.auto_start` を含む patch は disk に永続化される。
/// (Task 0 baseline で確認された in-memory / disk 不整合の安全網が
/// patch 経路でも機能することの回帰防止)
#[test]
fn apply_patch_persists_to_disk_atomically() {
    let dir = make_tempdir("patch-3");
    let path = dir.join("config.json");
    let cm = ConfigManager::init_at(&path).unwrap();
    let patch = patch::AppConfigPatch {
        general: Some(patch::GeneralConfigPatch {
            language: Some("en".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    cm.apply_patch(patch).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    let reloaded: AppConfig = serde_json::from_str(&content).unwrap();
    assert_eq!(reloaded.general.language, "en");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cursors_dir_is_under_home() {
    // ~/.custom_cursors/ をホーム配下に解決できる。
    // 同プロセスで `CUSTOM_CURSORS_DIR_OVERRIDE` を設定する別テスト
    // (`commands::theme::tests`) と直列化するため、共有ロックを取得する。
    let _g = super::cursors_dir_override_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let saved = std::env::var("CUSTOM_CURSORS_DIR_OVERRIDE").ok();
    std::env::remove_var("CUSTOM_CURSORS_DIR_OVERRIDE");
    let dir = ConfigManager::cursors_dir().unwrap();
    if let Some(v) = saved {
        std::env::set_var("CUSTOM_CURSORS_DIR_OVERRIDE", v);
    }
    assert!(dir.ends_with(".custom_cursors"));
    if let Some(home) = dirs::home_dir() {
        assert!(dir.starts_with(&home));
    }
}

// ===== G4: `update_config` を atomic write + rename 化する回帰テスト =====
//
// 既存の `fs::write` 直書きは電源断 / プロセス落ちで config.json を中途半端な
// 状態 (末尾が切れた JSON) にしてしまう。temp ファイルへ書いてから `rename` で
// 置換する atomic_write ヘルパーでこのクラスを潰す。
//
// テスト戦略:
//  - `atomic_write` ヘルパー自体は副作用が単純 (ファイル 1 個) なので直接検証
//  - `ConfigManager::update` は tempdir ベースの test-only コンストラクタ
//    `init_at` 経由で生成し、原子的書き戻しと in-memory ロールバックを検証

/// テスト専用の作業ディレクトリ。プロセス ID + ナノ秒 nonce で衝突回避。
/// 既存 `cursors_dir_is_under_home` と同様にグローバル env を触らない設計。
fn make_tempdir(label: &str) -> PathBuf {
    let pid = std::process::id();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ecs-config-test-{}-{}-{}", label, pid, nonce));
    fs::create_dir_all(&dir).expect("tempdir 作成");
    dir
}

/// atomic_write: 新規パスへの書き込み
#[test]
fn atomic_write_creates_target_file() {
    let dir = make_tempdir("create");
    let path = dir.join("config.json");
    atomic_write(&path, "{\"a\":1}").expect("atomic_write 成功");
    assert!(path.exists(), "書き込み先が存在するべき");
    assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}");
    let _ = fs::remove_dir_all(&dir);
}

/// atomic_write: 既存ファイルの内容を置換する
#[test]
fn atomic_write_overwrites_existing_file() {
    let dir = make_tempdir("overwrite");
    let path = dir.join("config.json");
    atomic_write(&path, "initial").unwrap();
    atomic_write(&path, "updated").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "updated");
    let _ = fs::remove_dir_all(&dir);
}

/// atomic_write: rename 失敗時 (target がディレクトリ) は元のファイルが消えない
/// (rename 前の状態に戻る) ことを確認する。fs::write 直書きだと
/// target ディレクトリ配下の該当パスが真っ先に消えて壊れるため、これで
/// ロールバック挙動の差分が固定できる。
#[test]
fn atomic_write_preserves_existing_file_on_failure() {
    let dir = make_tempdir("preserve");
    let path = dir.join("config.json");
    atomic_write(&path, "original").unwrap();
    // ターゲットを「ディレクトリ化」して rename を失敗させる
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    let result = atomic_write(&path, "new-content");
    assert!(result.is_err(), "rename 失敗時はエラーを返すべき");

    // 失敗後にファイル/ディレクトリが「壊れた中途半端な状態」になっていないこと
    // (ディレクトリが残っている = ターゲット内容は破壊されていない)
    assert!(
        path.is_dir(),
        "rename 失敗時にターゲットディレクトリが消えるべきでない"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// atomic_write: rename 失敗時は temp ファイルを掃除して残骸を残さない
#[test]
fn atomic_write_cleans_temp_on_failure() {
    let dir = make_tempdir("cleanup");
    let path = dir.join("config.json");
    atomic_write(&path, "original").unwrap();
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    let _ = atomic_write(&path, "new-content");
    let temp = path.with_extension("json.tmp");
    assert!(
        !temp.exists(),
        "失敗時に temp ファイルが残骸として残ってはいけない: {}",
        temp.display()
    );
    let _ = fs::remove_dir_all(&dir);
}

/// `update` が atomic 経路で永続化されること: 書き戻し後の content に
/// 上書きしたフィールドが反映されており、別 ConfigManager で再読込しても
/// 同じ状態になることを検証する (ラウンドトリップの原子性)。
#[test]
fn update_persists_changes_atomically() {
    let dir = make_tempdir("persist");
    let path = dir.join("config.json");
    let mgr = ConfigManager::init_at(&path).expect("init_at");

    mgr.update(|c| {
        c.general.language = "en".to_string();
    })
    .expect("update 成功");

    // ファイル内容に反映されている
    let content = fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("\"language\": \"en\""),
        "language=en がファイルに書かれていない: {}",
        content
    );

    // 別 ConfigManager で再読込しても同じ
    let mgr2 = ConfigManager::init_at(&path).expect("init_at reload");
    assert_eq!(mgr2.get().unwrap().general.language, "en");
    let _ = fs::remove_dir_all(&dir);
}

/// `update` のディスク書き込みが失敗した場合、in-memory 状態も変更されない
/// (atomicity: メモリとディスクの不整合を残さない)。
#[test]
fn update_rolls_back_in_memory_state_on_disk_failure() {
    let dir = make_tempdir("rollback");
    let path = dir.join("config.json");
    let mgr = ConfigManager::init_at(&path).expect("init_at");

    // 初期状態 (default) を記録
    let before = mgr.get().unwrap().general.language.clone();
    assert_eq!(before, "auto");

    // init 完了後にターゲットをディレクトリ化 → 以降の update は rename 失敗する
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    let result = mgr.update(|c| {
        c.general.language = "en".to_string();
    });
    assert!(result.is_err(), "disk 失敗時は update がエラーを返すべき");

    // メモリ側が書き換えられていない (temp への update は commit しない)
    let after = mgr.get().unwrap().general.language.clone();
    assert_eq!(
        after, "auto",
        "disk 失敗時に in-memory 状態が変更されてはいけない"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// `update` で複数フィールドが同時に更新されるケースも原子的に反映される
/// (temp 経由で書き戻し → in-memory へ一括コミット)。
#[test]
fn update_commits_multiple_fields_atomically() {
    let dir = make_tempdir("multi");
    let path = dir.join("config.json");
    let mgr = ConfigManager::init_at(&path).expect("init_at");

    mgr.update(|c| {
        c.general.language = "ja".to_string();
        c.general.crash_reporting = true;
        c.logging.level = "DEBUG".to_string();
    })
    .expect("update 成功");

    let cfg = mgr.get().unwrap();
    assert_eq!(cfg.general.language, "ja");
    assert!(cfg.general.crash_reporting);
    assert_eq!(cfg.logging.level, "DEBUG");

    // 永続化内容にも 3 フィールド全部入っている
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("\"language\": \"ja\""));
    assert!(content.contains("\"crash_reporting\": true"));
    assert!(content.contains("\"level\": \"DEBUG\""));
    let _ = fs::remove_dir_all(&dir);
}

// ===== Wave 1A: Config v2 migration tests =====
//
// V1 → V2 変換を `ConfigManager::migrate` に集約した特性化テスト群。
// log/2026-07-25.md Wave 1A 仕様「テスト: V1 JSON 5 種 (欠落キー / 未知キー /
// 旧型 `github_account` 不在 / 旧 `general` 欠落 / 旧 logging 欠落) → V2 復元」
// を網羅する。
//
// 各テストは `migrate` を直接呼び、`(cfg, changed)` の両方を検証する:
//   - `cfg.schema_version == 2` に昇格している
//   - 6 新フィールドが V2 既定値で埋まっている
//   - ユーザー設定が保持されている
//   - `changed == true` (V1 → V2 変換が発生した)

/// V1 JSON (欠落キー: `github_account` 不在) → V2 復元。
/// 旧 `general` / `security` / `logging` すべて存在し、`github_account` だけ省略。
#[test]
fn migrate_v1_missing_github_account_to_v2() {
    let v1_json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
    let (cfg, changed) = ConfigManager::migrate(v1_json).expect("V1 → V2 成功");
    assert!(changed, "V1 → V2 変換は changed=true を返すべき");
    assert_eq!(cfg.schema_version, 2);
    assert!(
        cfg.github_account.is_none(),
        "旧 V1 に無い github_account は None"
    );
    // 6 新フィールドが V2 既定値で埋まる
    assert!(cfg.general.show_apply_toast);
    assert!(cfg.general.apply_shadow_control);
    assert!(!cfg.general.start_minimized);
    assert!(cfg.general.show_storage_warning);
    assert!(!cfg.security.require_signed_themes);
    assert!(cfg.security.warn_unsigned_import);
}

/// V1 JSON (未知キー: 旧 `dark_mode` ブロック) → V2 復元。
/// `serde` 既定の未知フィールド黙殺で `dark_mode` は無視される。
#[test]
fn migrate_v1_with_unknown_dark_mode_block_to_v2() {
    let v1_json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false,
                "favorites": ["00000000-0000-0000-0000-000000000001"],
                "usage": {
                    "00000000-0000-0000-0000-000000000001": {
                        "apply_count": 5,
                        "last_applied_at": "2026-05-14T12:00:00Z"
                    }
                }
            },
            "dark_mode": {
                "enabled": false,
                "light_theme_id": null,
                "dark_theme_id": null
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            },
            "github_account": {
                "login": "octocat",
                "token_saved_at": "2026-05-14T12:00:00Z"
            }
        }"#;
    let (cfg, changed) = ConfigManager::migrate(v1_json).expect("V1 → V2 成功");
    assert!(changed);
    // dark_mode は黙殺され、それ以外のフィールドが引き継がれる
    assert_eq!(cfg.general.favorites.len(), 1);
    assert_eq!(cfg.general.usage.len(), 1);
    assert_eq!(cfg.general.usage.values().next().unwrap().apply_count, 5);
    assert_eq!(cfg.github_account.as_ref().unwrap().login, "octocat");
    assert_eq!(cfg.schema_version, 2);
}

/// V1 JSON (旧 `general` の必須フィールド欠落: `crash_reporting` 不在) → V2 復元。
/// `serde(default)` で `crash_reporting=false` にフォールバック。
#[test]
fn migrate_v1_missing_general_field_to_v2() {
    let v1_json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": false,
                "auto_update": true,
                "language": "en",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R"
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
    let (cfg, changed) = ConfigManager::migrate(v1_json).expect("V1 → V2 成功");
    assert!(changed);
    // crash_reporting は serde(default) で false にフォールバック
    assert!(!cfg.general.crash_reporting);
    // favorites / usage も serde(default) で空
    assert!(cfg.general.favorites.is_empty());
    assert!(cfg.general.usage.is_empty());
    // ユーザー指定のフィールドは保持
    assert!(!cfg.general.auto_start);
    assert_eq!(cfg.general.language, "en");
}

/// V1 JSON (旧 `logging` 欠落) はエラー (logging は serde(default) 不在の必須フィールド)。
/// この場合、v1 struct の deserialize 段階で失敗する。
#[test]
fn migrate_v1_missing_logging_block_returns_error() {
    let v1_json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R"
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            }
        }"#;
    let result = ConfigManager::migrate(v1_json);
    assert!(
        result.is_err(),
        "logging 欠落 V1 は v1 struct の serde 段階で失敗するべき"
    );
}

/// V1 JSON (完全に最小: 必須フィールドのみ) → V2 復元。
/// `serde(default)` で補えるものはすべて補填される。
#[test]
fn migrate_v1_minimal_to_v2() {
    let v1_json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "auto",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R"
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
    let (cfg, changed) = ConfigManager::migrate(v1_json).expect("V1 → V2 成功");
    assert!(changed);
    assert_eq!(cfg.schema_version, 2);
    assert_eq!(cfg.general.language, "auto");
    // 6 新フィールドがすべて V2 既定値で埋まる
    assert!(cfg.general.show_apply_toast);
    assert!(cfg.general.apply_shadow_control);
    assert!(!cfg.general.start_minimized);
    assert!(cfg.general.show_storage_warning);
    assert!(!cfg.security.require_signed_themes);
    assert!(cfg.security.warn_unsigned_import);
}

/// V2 JSON → V2 復元。`changed == false` で永続化が不要。
#[test]
fn migrate_v2_returns_unchanged() {
    let v2_json = r#"{
            "schema_version": 2,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false,
                "show_apply_toast": false,
                "apply_shadow_control": false,
                "start_minimized": true,
                "show_storage_warning": false
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824,
                "require_signed_themes": true,
                "warn_unsigned_import": false
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
    let (cfg, changed) = ConfigManager::migrate(v2_json).expect("V2 → V2 成功");
    assert!(!changed, "V2 → V2 は changed=false");
    assert_eq!(cfg.schema_version, 2);
    assert!(!cfg.general.show_apply_toast, "ユーザー設定 false を保持");
    assert!(cfg.general.start_minimized);
    assert!(cfg.security.require_signed_themes);
    assert!(!cfg.security.warn_unsigned_import);
}

/// `schema_version` が CURRENT より新しい → エラー (現行挙動と同一)。
#[test]
fn migrate_future_version_returns_error() {
    let future_json = r#"{ "schema_version": 99 }"#;
    let result = ConfigManager::migrate(future_json);
    assert!(result.is_err(), "schema_version=99 はエラー");
}

/// `schema_version` 欠落 → エラー。
#[test]
fn migrate_missing_schema_version_returns_error() {
    let no_version = r#"{ "general": {} }"#;
    let result = ConfigManager::migrate(no_version);
    assert!(result.is_err(), "schema_version なしはエラー");
}

/// 不正 JSON (括弧不一致) → エラー。
#[test]
fn migrate_invalid_json_returns_error() {
    let result = ConfigManager::migrate("{ not valid json");
    assert!(result.is_err());
}

/// `load_or_initialize` が V1 ファイルを V2 に migrate して atomic_write で永続化する E2E 確認。
#[test]
fn load_or_initialize_v1_file_persists_as_v2() {
    let dir = make_tempdir("v1-migrate");
    let path = dir.join("config.json");

    // V1 JSON を手書きで配置 (= Wave 1A 移行を模擬)。
    let v1_json = r#"{
            "schema_version": 1,
            "general": {
                "auto_start": true,
                "auto_update": true,
                "language": "ja",
                "active_theme_id": null,
                "panic_hotkey": "Ctrl+Alt+Shift+R",
                "crash_reporting": false
            },
            "security": {
                "max_pack_compressed_size": 52428800,
                "max_pack_uncompressed_size": 209715200,
                "max_image_file_size": 10485760,
                "storage_warning_threshold": 1073741824
            },
            "logging": {
                "level": "INFO",
                "retention_days": 14,
                "max_total_size": 104857600
            }
        }"#;
    fs::write(&path, v1_json).expect("V1 config.json 書き込み");

    // 起動を模擬: V1 ファイルを読み込んで V2 に migrate する。
    let mgr = ConfigManager::init_at(&path).expect("init_at");
    let cfg = mgr.get().unwrap();

    // 1. in-memory は V2
    assert_eq!(cfg.schema_version, 2);
    assert!(cfg.general.show_apply_toast);

    // 2. ディスクも V2 に書き戻されている (=次回以降は migrate 不要)
    let persisted = fs::read_to_string(&path).unwrap();
    assert!(persisted.contains("\"schema_version\": 2"));
    assert!(persisted.contains("\"show_apply_toast\": true"));
    assert!(persisted.contains("\"warn_unsigned_import\": true"));

    let _ = fs::remove_dir_all(&dir);
}
