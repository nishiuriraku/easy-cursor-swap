//! ConfigManager::migrate の契約 (v1→v2 昇格 / v2 パススルー / 不正 / 新しすぎる)。
//! 実ファイル quarantine (`config.corrupt.*.json`) は config::tests (unit) が担当。
//! 純ロジックのみのため OS 非依存 (Linux 可)。
use app_lib::config::{AppConfig, ConfigManager, CURRENT_SCHEMA_VERSION};

#[test]
fn migrate_passes_through_current_schema() {
    let raw = serde_json::to_string(&AppConfig::default()).unwrap();
    let (cfg, changed) = ConfigManager::migrate(&raw).expect("v2 parses");
    assert!(!changed);
    assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
}

#[test]
fn migrate_upgrades_schema_v1() {
    // v1 = schema_version:1 かつ V2 で追加されたフィールド無し。
    let raw = r#"{"schema_version":1,"general":{"auto_start":false,"auto_update":true,"language":"ja","active_theme_id":null,"panic_hotkey":"Ctrl+Alt+Shift+R","favorites":[],"usage":{}},"security":{"max_pack_compressed_size":52428800,"max_pack_uncompressed_size":209715200,"max_image_file_size":10485760,"storage_warning_threshold":1073741824,"require_signed_themes":false},"logging":{"level":"INFO","retention_days":14,"max_total_size":104857600},"github_account":null}"#;
    let (cfg, changed) = ConfigManager::migrate(raw).expect("v1 upgrades");
    assert!(changed, "v1 → v2 で changed=true");
    assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(cfg.general.language, "ja");
}

#[test]
fn migrate_rejects_garbage_and_future_schema() {
    assert!(ConfigManager::migrate("not json").is_err());
    assert!(ConfigManager::migrate("{\"schema_version\": 999}").is_err());
}
