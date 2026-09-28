//! .cursorprofile (BackupManager) の export → 全消去 → import(merge=false) の往復。
#![cfg(windows)]

mod common;

use app_lib::backup::BackupManager;
use app_lib::config::AppConfig;
use app_lib::theme::ThemeManager;
use common::*;

#[test]
fn profile_export_then_import_restores_theme_dirs() {
    let g = CursorsDirGuard::new("profile");
    let mut meta = minimal_metadata("IT Profile");
    let id = meta.id;
    let pack = ThemeManager::write_cursorpack_to_buffer(&mut meta, &arrow_cur_bytes()).unwrap();
    ThemeManager::import_cursorpack_bytes(&pack).unwrap();
    // `_` 始まりのファイルは export 対象外であることも確認
    std::fs::write(g.dir.join("_initial_snapshot.json"), b"{}").unwrap();

    let out = g.dir.join("out").join("test.cursorprofile");
    BackupManager::export(&out, &AppConfig::default()).expect("export");
    assert!(out.is_file());

    std::fs::remove_dir_all(g.dir.join(id.to_string())).unwrap();
    assert!(!ThemeManager::theme_exists(id));

    let env = BackupManager::import(&out, false).expect("import");
    assert_eq!(
        env.config.schema_version,
        AppConfig::default().schema_version
    );
    assert!(
        ThemeManager::theme_exists(id),
        "テーマディレクトリが復元される"
    );
    assert!(g
        .dir
        .join(id.to_string())
        .join("cursors")
        .join("Arrow.cur")
        .is_file());
}

#[test]
fn profile_import_rejects_missing_profile_json() {
    let g = CursorsDirGuard::new("profile-bad");
    let p = g.dir.join("bad.cursorprofile");
    {
        let f = std::fs::File::create(&p).unwrap();
        let mut zip = zip::ZipWriter::new(f);
        zip.start_file(
            "cursors/x/theme.json",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.finish().unwrap();
    }
    assert!(BackupManager::import(&p, true).is_err());
}
