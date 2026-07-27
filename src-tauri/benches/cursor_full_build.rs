//! フルビルド (17 役割 × 6 サイズ = 102 枚) のマイクロベンチ (Wave 2AB / Task 13)。
//!
//! `build_cur_from_png` を 17 役割分 (= 17 呼び出し × 6 サイズ = 102 枚の
//! Lanczos リサイズ) 連続で回す。Cache は測定ごとに `clear_resize_cache()`
//! で初期化するため cold ベンチでは Lanczos リサイズが純粋に計測される。
//!
//! 17 役割は `CursorRole` 系 (`scheme_index` で並べる) 相当の固定 Vec を
//! このファイルに持つ。`build_cur_from_png` は `RoleBuildEntry` (= ロール名 +
//! PNG bytes + ホットスポット) を要求しない。
//!
//! 2 系統の入力を用意する (= Wave 2AB / Task 13 item k「cache-key リアリズム」):
//!  - `single_png`: 1 種類の 64×64 グラデーション PNG を全役割で共有
//!    (= 旧実装と同じキャッシュヒット中心の最小計測)
//!  - `multi_png`: 17 役割ごとに **異なる** PNG バイト列を使う
//!    (= 役割毎にキャッシュキーが異なるため、`src_hash` (= `image_short_hash`)
//!    が役割ごとに独立 = リサイズキャッシュがヒットしない cold シナリオ)
//!    現実の CreatorExport (= 各役割で別 PNG をアップロード) のキャッシュ圧力に
//!    近いシナリオを計測するための variant。
//!
//! 走らせ方:
//!   cargo bench --bench cursor_full_build --manifest-path src-tauri/Cargo.toml
//!
//! 設計ノート (= Wave 2AB / Task 13 item n「bench-helpers dark-assert out-of-band」):
//!  - bench の `b.iter(...)` クロージャ内で assert (例えば「build_cur_from_png が
//!    Ok(()) を返す」「出力が 6 サイズ含む」) を行わないこと。Criterion は同
//!    クロージャを何百回も回すため、毎 iter の assert は計測ノイズになり
//!    timing が歪む (=「assertion softened to timing-only」回帰の再発)。
//!  - dark-assert (件数 / 戻り値の健全性検証) は out-of-band (= `b.iter` の外で
//!    1 度だけ) で行う。`b.iter` 内は純粋に計測対象の関数呼び出しのみに絞り、
//!    `black_box(...)` で constant-folding を防ぐ。
//!  - env var / グローバル状態を触る bench では `Drop` を持つ RAII ガード
//!    (= list_themes.rs の `EnvVarGuard` 相当) を使い、panic 時のリソース漏れを
//!    防ぐ。process 全体に残った env var は他の cargo test とレースして flaky を生む。

use app_lib::cursor::{build_cur_from_png, clear_resize_cache, ResizeMethod};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

/// ベンチ用 PNG (N×N の多色グラデーション)。`build_cur_from_png` は PNG マジック
/// バイトをチェックするので本物の PNG が必要。64色を超える入力にして、
/// 自動判定で `ResizeMethod::Lanczos` が `Nearest` に変更されないようにする。
fn make_test_png(size: u32, seed: u8) -> Vec<u8> {
    use image::{ImageBuffer, Rgba};
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(size, size, |x, y| {
        Rgba([
            ((x as u16 + seed as u16) % 256) as u8,
            ((y as u16 + seed as u16) % 256) as u8,
            ((x as u16 + y as u16 + seed as u16) % 256) as u8,
            255,
        ])
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
    let png = make_test_png(64, 0);

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
    let png = make_test_png(64, 0);

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

/// Wave 2AB / Task 13 item (k): 役割ごとに **異なる PNG** (= 17 種) を使い、
/// 役割毎のキャッシュキーが独立した現実的な CreatorExport シナリオを計測する。
/// 旧実装は単一 PNG 共有のため全役割の `src_hash` (= `image_short_hash(png_bytes)`)
/// が同一でリサイズキャッシュが 1 件分しか効かなかった (= cache-key リアリズム低)。
/// この variant は role ごとにキャッシュミスする worst-case を純粋に計測する。
fn bench_full_build_17roles_multi_png_cold(c: &mut Criterion) {
    // 役割数 (17) 分の異なる PNG を事前生成 (seed 違いで src_hash を分散させる)
    let multi_png: Vec<Vec<u8>> = (0..ROLES_17.len())
        .map(|i| make_test_png(64, i as u8))
        .collect();

    c.bench_function("full_build_17roles_6sizes_multi_png_cold", |b| {
        b.iter(|| {
            clear_resize_cache();
            for (i, role) in ROLES_17.iter().enumerate() {
                let _ = black_box(
                    build_cur_from_png(
                        black_box(&multi_png[i]),
                        8,
                        8,
                        ResizeMethod::Lanczos,
                        None,
                        None,
                    )
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
    bench_full_build_17roles_warm,
    bench_full_build_17roles_multi_png_cold
);
criterion_main!(benches);
