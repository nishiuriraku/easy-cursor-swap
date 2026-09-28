//! 統合テスト共通ヘルパ。`CUSTOM_CURSORS_DIR_OVERRIDE` をプロセス全体で直列化して temp に向ける.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use app_lib::cursor::{build_cur_from_png, ResizeMethod};
use app_lib::theme::types::{
    CursorDefinition, Hotspot, LocalizedString, Ratio01, ThemeMetadata, ThemeSource,
};
use uuid::Uuid;

pub const ONE_PIX_PNG: &[u8] = include_bytes!("../fixtures/1x1.png");

/// env var はプロセス共有なので、テスト間で直列化する。
pub fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// `CUSTOM_CURSORS_DIR_OVERRIDE` を temp に向け、Drop で元に戻す RAII ガード。
pub struct CursorsDirGuard {
    pub dir: PathBuf,
    prev: Option<String>,
    _lock: MutexGuard<'static, ()>,
}

impl CursorsDirGuard {
    pub fn new(tag: &str) -> Self {
        let lock = env_lock();
        let dir = std::env::temp_dir().join(format!(
            "ecs-it-{}-{}-{}",
            tag,
            std::process::id(),
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create temp cursors dir");
        let prev = std::env::var("CUSTOM_CURSORS_DIR_OVERRIDE").ok();
        std::env::set_var("CUSTOM_CURSORS_DIR_OVERRIDE", &dir);
        Self {
            dir,
            prev,
            _lock: lock,
        }
    }
}

impl Drop for CursorsDirGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var("CUSTOM_CURSORS_DIR_OVERRIDE", v),
            None => std::env::remove_var("CUSTOM_CURSORS_DIR_OVERRIDE"),
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// 1 ロール (Arrow) だけ持つ最小 ThemeMetadata。
pub fn minimal_metadata(name: &str) -> ThemeMetadata {
    let mut cursors = HashMap::new();
    cursors.insert(
        "Arrow".to_string(),
        CursorDefinition {
            file: "cursors/Arrow.cur".to_string(),
            hotspot: Hotspot {
                x: Ratio01::new(0.0),
                y: Ratio01::new(0.0),
            },
            resize_method: "lanczos".to_string(),
            size_overrides: None,
        },
    );
    ThemeMetadata {
        schema_version: 1,
        id: Uuid::new_v4(),
        name: LocalizedString::Simple(name.to_string()),
        version: "1.0.0".to_string(),
        created_at: "2026-09-28T00:00:00Z".to_string(),
        requires_os_shadow: false,
        cursors,
        author: Some("integration-test".to_string()),
        license: None,
        homepage: None,
        description: None,
        min_app_version: None,
        signature: None,
        tags: Vec::new(),
        source: ThemeSource::Local,
        cloned_from_marketplace_id: None,
    }
}

/// fixtures/1x1.png から実 .cur バイト列を作る (Arrow 1 ロール分)。
pub fn arrow_cur_bytes() -> HashMap<String, Vec<u8>> {
    let cur = build_cur_from_png(ONE_PIX_PNG, 0, 0, ResizeMethod::Lanczos, None, None)
        .expect("build .cur from 1x1.png");
    HashMap::from([("Arrow".to_string(), cur)])
}
