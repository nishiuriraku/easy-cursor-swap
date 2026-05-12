//! cursor.rs のホットパス (.cur ビルド + リサイズ) を計測。
//!
//! 仕様書 §「パフォーマンス UX」より:
//!  - 17 役割 × 6 解像度 = 102 枚の Lanczos リサイズが発生
//!  - 1 役割 (1 PNG → 6 サイズ .cur) のビルド時間が現実的速度であるかを確認
//!
//! 走らせ方:
//!   cargo bench --bench cursor_build --manifest-path src-tauri/Cargo.toml

use app_lib::cursor::{build_cur_from_png, clear_resize_cache, ResizeMethod};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// ベンチ用 PNG (32×32 単色)。
/// build_cur_from_png は Magic Byte をチェックするため、本物の PNG が必要。
fn make_test_png(size: u32) -> Vec<u8> {
    use image::{ImageBuffer, Rgba};
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(size, size, Rgba([124, 242, 212, 255]));
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

fn bench_build_lanczos(c: &mut Criterion) {
    let png_64 = make_test_png(64);
    let png_256 = make_test_png(256);

    // キャッシュをクリアして cold ベンチ
    c.bench_function("build_cur_from_png/64x64/lanczos/cold", |b| {
        b.iter(|| {
            clear_resize_cache();
            let _ = build_cur_from_png(black_box(&png_64), 0, 0, ResizeMethod::Lanczos, None, None)
                .expect("build");
        })
    });

    // キャッシュ温まった warm ベンチ (102 枚生成シナリオの 2 回目以降を模擬)
    let _ = build_cur_from_png(&png_64, 0, 0, ResizeMethod::Lanczos, None, None);
    c.bench_function("build_cur_from_png/64x64/lanczos/warm", |b| {
        b.iter(|| {
            let _ = build_cur_from_png(black_box(&png_64), 0, 0, ResizeMethod::Lanczos, None, None)
                .expect("build");
        })
    });

    c.bench_function("build_cur_from_png/256x256/lanczos/cold", |b| {
        b.iter(|| {
            clear_resize_cache();
            let _ =
                build_cur_from_png(black_box(&png_256), 0, 0, ResizeMethod::Lanczos, None, None)
                    .expect("build");
        })
    });
}

fn bench_build_nearest(c: &mut Criterion) {
    let png_32 = make_test_png(32);

    c.bench_function("build_cur_from_png/32x32/nearest", |b| {
        b.iter(|| {
            let _ = build_cur_from_png(black_box(&png_32), 0, 0, ResizeMethod::Nearest, None, None)
                .expect("build");
        })
    });
}

criterion_group!(benches, bench_build_lanczos, bench_build_nearest);
criterion_main!(benches);
