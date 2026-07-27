//! `ThemeManager::list_themes` のマイクロベンチ (Wave 2AB / Task 13)。
//!
//! ホームディレクトリ下の `~/.custom_cursors/` (= 実ユーザ環境) には触れず、
//! `std::env::temp_dir()` 配下のユニークサブディレクトリに N 個のテーマ
//! (= theme.json + cursors/Arrow.png) を合成して計測する。
//!
//! テーマ数 (N) を 10 / 50 / 100 でパラメトリックに走らせ、Library 初期化
//! (= `list_themes`) の現実的なデータ量を網羅する。
//!
//! 各 N について独立 tempdir を新規作成する (= bench iter 間でディレクトリ
//! 構成を競合せず、`list_themes` の純粋な走査時間を計測する)。
//!
//! 走らせ方:
//!   cargo bench --bench list_themes --manifest-path src-tauri/Cargo.toml
//!
//! 設計ノート:
//!  - `CUSTOM_CURSORS_DIR_OVERRIDE` env var は `EnvVarGuard` (RAII Drop) で
//!    スコープ管理し、bench iter 中の panic でも必ず解除する (= プロセスレベルで
//!    残った env var が他テスト/プロセスに漏れない)。
//!  - bench の `b.iter(...)` 内では assertion (= `count == n`) を行わない:
//!    Criterion は同関数を何百回も回すため、毎 iter の count 比較は計測ノイズになる。
//!    代わりにセットアップ直後 (= warm-up 前) に 1 度だけ list_themes を呼んで
//!    件数整合性を out-of-band に assert し、bench 計測対象 (= iter 内) は
//!    純粋な走査時間のみに絞り込む。

use app_lib::theme::types::{
    CursorDefinition, Hotspot, LocalizedString, Ratio01, ThemeMetadata, ThemeSource,
};
use app_lib::theme::ThemeManager;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use uuid::Uuid;

const THEMES_PER_BENCH: &[usize] = &[10, 50, 100];

/// 1×1 黒 PNG のバイト列 (lazy 初期化で 1 度だけ作る)。
fn tiny_png() -> Vec<u8> {
    use image::{ImageBuffer, Rgba};
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 255]));
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(encoder, img.as_raw(), 1, 1, image::ExtendedColorType::Rgba8)
        .expect("encode tiny png");
    buf
}

/// env var の RAII ガード。Drop で `set_var` でセットした値を必ず `remove_var` する。
/// bench iter 中の panic でも Drop が走り、process 全体への env var 漏れを防ぐ
/// (= 後続テストや cargo test --lib とのレース回避)。
struct EnvVarGuard {
    key: &'static str,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &std::path::Path) -> Self {
        std::env::set_var(key, value);
        Self { key }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        std::env::remove_var(self.key);
    }
}

/// `n` 件のテーマディレクトリを temp 配下に作成し、そのパスを EnvVarGuard と共に返す。
/// 既存 `ThemeManager::list_themes` の挙動と完全に一致させるため、theme.json
/// + cursors/<role>.png (1 個以上の PNG) を同梱する。
fn setup_themes(n: usize, png: &[u8]) -> (std::path::PathBuf, EnvVarGuard) {
    let pid = std::process::id();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ecs-list-themes-bench-{}-{}-x", pid, nonce));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    for i in 0..n {
        let theme_id = Uuid::new_v4();
        let theme_dir = dir.join(theme_id.to_string());
        std::fs::create_dir_all(theme_dir.join("cursors")).expect("mkdir");

        let mut cursors = HashMap::new();
        cursors.insert(
            "Arrow".to_string(),
            CursorDefinition {
                file: "cursors/Arrow.png".to_string(),
                hotspot: Hotspot {
                    x: Ratio01::new(0.0),
                    y: Ratio01::new(0.0),
                },
                resize_method: "lanczos".to_string(),
                size_overrides: None,
            },
        );
        let metadata = ThemeMetadata {
            schema_version: 1,
            id: theme_id,
            name: LocalizedString::Simple(format!("Bench Theme {}", i)),
            version: "1.0.0".to_string(),
            created_at: "2026-07-27T00:00:00Z".to_string(),
            requires_os_shadow: false,
            cursors,
            author: Some("bench".to_string()),
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
            tags: Vec::new(),
            source: ThemeSource::Local,
            cloned_from_marketplace_id: None,
        };
        std::fs::write(
            theme_dir.join("theme.json"),
            serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
        )
        .expect("write theme.json");
        std::fs::write(theme_dir.join("cursors").join("Arrow.png"), png)
            .expect("write cursors/Arrow.png");
    }

    let guard = EnvVarGuard::set("CUSTOM_CURSORS_DIR_OVERRIDE", &dir);
    (dir, guard)
}

fn bench_list_themes_parametric(c: &mut Criterion) {
    let png = tiny_png();
    let mut group = c.benchmark_group("list_themes");

    for &n in THEMES_PER_BENCH {
        let (setup_dir, _env_guard) = setup_themes(n, &png);

        // Out-of-band assertion (item h, n): bench 計測 (= b.iter) の外で 1 度だけ
        // `list_themes` を呼んで、N 件のテーマが返ることを verify する。
        // Criterion は b.iter を何百回も回すため、毎 iter 内で assert すると計測
        // ノイズになり timing が歪む (=「assertion softened to timing-only」回帰の再発)。
        // dark-assert out-of-band パターンを採用し、計測対象は純粋な走査のみに絞る。
        let themes = ThemeManager::list_themes(None, &[], &HashMap::new())
            .expect("list_themes should succeed for fresh tempdir");
        assert_eq!(
            themes.len(),
            n,
            "list_themes returned {} themes, expected {} (synthesized N mismatch)",
            themes.len(),
            n
        );

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let _ = ThemeManager::list_themes(None, &[], &HashMap::new());
            });
        });

        // 次の N 計測前にクリーンアップ: tempdir 削除 + env var 解除 (RAII guard Drop)
        let _ = std::fs::remove_dir_all(&setup_dir);
        // _env_guard は for ループの次 iter で drop される (= remove_var 自動実行)
    }

    group.finish();
}

criterion_group!(benches, bench_list_themes_parametric);
criterion_main!(benches);
