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

/// `n` 件のテーマディレクトリを temp 配下に作成し、そのパスを返す。
/// 既存 `ThemeManager::list_themes` の挙動と完全に一致させるため、theme.json
/// + cursors/<role>.png (1 個以上の PNG) を同梱する。
fn setup_themes(n: usize, png: &[u8]) -> std::path::PathBuf {
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

    std::env::set_var("CUSTOM_CURSORS_DIR_OVERRIDE", &dir);
    dir
}

fn bench_list_themes_parametric(c: &mut Criterion) {
    let png = tiny_png();
    let mut group = c.benchmark_group("list_themes");

    for &n in THEMES_PER_BENCH {
        let setup_dir = setup_themes(n, &png);

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let _ = ThemeManager::list_themes(None, &[], &HashMap::new());
            });
        });

        // 次の N 計測前にクリーンアップ (env + tempdir)
        let _ = std::fs::remove_dir_all(&setup_dir);
        std::env::remove_var("CUSTOM_CURSORS_DIR_OVERRIDE");
    }

    group.finish();
}

criterion_group!(benches, bench_list_themes_parametric);
criterion_main!(benches);
