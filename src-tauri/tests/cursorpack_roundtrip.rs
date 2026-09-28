//! .cursorpack の build → inspect → import → list/load → duplicate → delete の往復と、
//! sanitize (path traversal) の拒否を、レジストリを触らずに検証する。
//!
//! `delete_theme` は backend 経由のため Windows 専用 (`WindowsCursorBackend` は
//! 実レジストリの Schemes キーを読むだけに留まり、一致しない値は書かない)。
#![cfg(windows)]

mod common;

use app_lib::platform::windows::WindowsCursorBackend;
use app_lib::theme::{sanitize_archive_path_pub, ThemeManager};
use common::*;
use std::io::Write;

fn backend() -> WindowsCursorBackend {
    WindowsCursorBackend
}

#[test]
fn cursorpack_build_inspect_import_delete_roundtrip() {
    let g = CursorsDirGuard::new("roundtrip");
    let mut meta = minimal_metadata("IT Roundtrip");
    let id = meta.id;
    let bytes = ThemeManager::write_cursorpack_to_buffer(&mut meta, &arrow_cur_bytes())
        .expect("write cursorpack");
    assert!(bytes.starts_with(b"PK"), "zip magic");

    let insp = ThemeManager::inspect_cursorpack_bytes(&bytes).expect("inspect");
    assert_eq!(insp.id, id);
    assert_eq!(insp.role_count, 1);
    assert!(
        insp.existing.is_none(),
        "未インポートなので existing は None"
    );

    let imported = ThemeManager::import_cursorpack_bytes(&bytes).expect("import");
    assert_eq!(imported, id);
    assert!(ThemeManager::theme_exists(id));
    assert!(g.dir.join(id.to_string()).join("theme.json").is_file());
    assert!(g
        .dir
        .join(id.to_string())
        .join("cursors")
        .join("Arrow.cur")
        .is_file());

    let loaded = ThemeManager::load_metadata(id).expect("load_metadata");
    assert_eq!(loaded.cursors.len(), 1);

    // 2 回目の inspect は existing が埋まる
    let insp2 = ThemeManager::inspect_cursorpack_bytes(&bytes).expect("inspect again");
    assert!(insp2.existing.is_some());

    let dup = ThemeManager::duplicate_theme(id).expect("duplicate");
    assert_ne!(dup, id);
    let listed = ThemeManager::list_themes(None, &[], &Default::default()).expect("list");
    assert_eq!(listed.len(), 2);

    ThemeManager::delete_theme(&backend(), id).expect("delete original");
    ThemeManager::delete_theme(&backend(), dup).expect("delete dup");
    assert!(!ThemeManager::theme_exists(id));
}

#[test]
fn import_rejects_path_traversal_entry() {
    let g = CursorsDirGuard::new("traversal");
    // theme.json は正常、cursors エントリ名に `..` を仕込んだ zip を手で組む
    let meta = minimal_metadata("Evil");
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("theme.json", opts).unwrap();
        zip.write_all(&serde_json::to_vec(&meta).unwrap()).unwrap();
        zip.start_file("../evil.cur", opts).unwrap();
        zip.write_all(b"MZ").unwrap();
        zip.finish().unwrap();
    }
    let err = ThemeManager::import_cursorpack_bytes(buf.get_ref()).unwrap_err();
    assert!(
        format!("{err}").contains(".."),
        "エラーにエントリ名が含まれる: {err}"
    );
    // NOTE: 現行実装は traversal 検出時に target_dir を掃除しないため
    // theme.json が残る (P09 非スコープ。将来のクリーンアップ対象として decision-log に記録)。
    assert!(g.dir.join(meta.id.to_string()).join("theme.json").is_file());
}

#[test]
fn sanitize_rejects_absolute_and_drive_paths() {
    for bad in ["/etc/passwd", "C:\\Windows\\x.cur", "..\\x", "a\0b", ""] {
        assert!(
            sanitize_archive_path_pub(bad).is_err(),
            "{bad:?} は拒否されるべき"
        );
    }
    assert_eq!(
        sanitize_archive_path_pub("cursors/Arrow.cur").unwrap(),
        std::path::PathBuf::from("cursors").join("Arrow.cur")
    );
}
