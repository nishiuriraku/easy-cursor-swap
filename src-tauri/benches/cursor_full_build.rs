//! フルビルド (17 役割 × 6 サイズ = 102 枚) のマイクロベンチ (Wave 2AB / Task 13)。
//!
//! `build_cur_from_png` を 17 役割分 (= 17 呼び出し × 6 サイズ = 102 枚の
//! Lanczos リサイズ) 連続で回す。Cache は測定ごとに `clear_resize_cache()`
//! で初期化するため cold ベンチでは Lanczos リサイズが純粋に計測される。
//!
//! 17 役割は `CursorRole` 系 (`scheme_index` で並べる) 相当の固定 Vec を
//! このファイルに持つ。`build_cur_from_png` は `RoleBuildEntry` (= ロール名 +
//! PNG bytes + ホットスポット) を要求しないので、PNG を 64×64 のグラデーションで
//! 1 種類用意して全役割で共有することで入力を最小化する。
//!
//! 走らせ方:
//!   cargo bench --bench cursor_full_build --manifest-path src-tauri/Cargo.toml

use app_lib::cursor::{build_cur_from_png, clear_resize_cache, ResizeMethod};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

/// ベンチ用 PNG (64×64 の多色グラデーション)。`build_cur_from_png` は PNG マジック
/// バイトをチェックするので本物の PNG が必要。64色を超える入力にして、
/// 自動判定で `ResizeMethod::Lanczos` が `Nearest` に変更されないようにする。
fn make_test_png(size: u32) -> Vec<u8> {
    use image::{ImageBuffer, Rgba};
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(size, size, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
    });
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        size,
        size,
        image::ExtendedColorType::Rgba8,
    )
    .expect("encode");
    buf
}

/// 17 役割の固定リスト。`CursorRole::all()` の出力と同順序 (= `scheme_index` 順)
/// を意図しているが、独立した Vec を持つことで `CursorRole` モジュールへの
/// 依存を避けている (bench は公開 API のみで構成)。
const ROLES_17: &[&str] = &[
    "Arrow",
    "Help",
    "AppStarting",
    "Wait",
    "Crosshair",
    "IBeam",
    "NWPen",
    "No",
    "SizeNS",
    "SizeWE",
    "SizeNWSE",
    "SizeNESW",
    "SizeAll",
    "UpArrow",
    "Hand",
    "Pin",
    "Person",
];

fn bench_full_build_17roles_cold(c: &mut Criterion) {
    let png = make_test_png(64);

    c.bench_function("full_build_17roles_6sizes_cold", |b| {
        b.iter(|| {
            // キャッシュをクリアして cold ベンチ (= Lanczos リサイズを再計算)
            clear_resize_cache();
            for role in ROLES_17 {
                let _ = black_box(
                    build_cur_from_png(black_box(&png), 8, 8, ResizeMethod::Lanczos, None, None)
                        .expect("build_cur_from_png should succeed"),
                );
                let _ = role;
            }
        })
    });
}

fn bench_full_build_17roles_warm(c: &mut Criterion) {
    let png = make_test_png(64);

    // warm: 1 回回してキャッシュに載せる
    clear_resize_cache();
    for _ in 0..3 {
        for _role in ROLES_17 {
            let _ = build_cur_from_png(&png, 8, 8, ResizeMethod::Lanczos, None, None);
        }
    }

    c.bench_function("full_build_17roles_6sizes_warm", |b| {
        b.iter(|| {
            // warm 経路: キャッシュにヒット
            for role in ROLES_17 {
                let _ = black_box(
                    build_cur_from_png(black_box(&png), 8, 8, ResizeMethod::Lanczos, None, None)
                        .expect("build_cur_from_png should succeed"),
                );
                let _ = role;
            }
        })
    });
}

criterion_group!(
    benches,
    bench_full_build_17roles_cold,
    bench_full_build_17roles_warm
);
criterion_main!(benches);
