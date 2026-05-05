//! 起動初期化パスのマイクロベンチ (Phase 8-1)
//!
//! Tauri WebView の起動時間は GUI 環境が必要なため CI で直接計測できないが、
//! Rust 側の初期化クリティカルパスは計測できる。
//!
//! 計測対象:
//!  - AppConfig::default() 生成
//!  - AppConfig → JSON シリアライズ
//!  - JSON → AppConfig デシリアライズ
//!  - フルラウンドトリップ (init() の主処理を模擬)
//!
//! 仕様書 §「パフォーマンス UX」の起動時間 ≤ 1.5 秒 のうち、
//! Rust 側初期化部分は数 ms で完了することを担保する。
//!
//! 走らせ方:
//!   cargo bench --bench startup --manifest-path src-tauri/Cargo.toml

use app_lib::config::AppConfig;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_default(c: &mut Criterion) {
    c.bench_function("AppConfig/default", |b| {
        b.iter(|| black_box(AppConfig::default()))
    });
}

fn bench_serialize(c: &mut Criterion) {
    let cfg = AppConfig::default();
    c.bench_function("AppConfig/serialize_pretty", |b| {
        b.iter(|| {
            let _ = serde_json::to_string_pretty(black_box(&cfg)).expect("serialize");
        })
    });
}

fn bench_deserialize(c: &mut Criterion) {
    let cfg = AppConfig::default();
    let json = serde_json::to_string_pretty(&cfg).expect("serialize");
    c.bench_function("AppConfig/deserialize", |b| {
        b.iter(|| {
            let _: AppConfig = serde_json::from_str(black_box(&json)).expect("deserialize");
        })
    });
}

/// init() の主処理 (config 生成 → 文字列化 → 再パース) のラウンドトリップ。
/// 実 init() はファイル I/O を含むが、CI で衝突を避けるためインメモリのみで計測。
fn bench_roundtrip(c: &mut Criterion) {
    c.bench_function("AppConfig/roundtrip_in_memory", |b| {
        b.iter(|| {
            let cfg = AppConfig::default();
            let json = serde_json::to_string_pretty(&cfg).expect("serialize");
            let _: AppConfig = serde_json::from_str(&json).expect("deserialize");
        })
    });
}

criterion_group!(
    benches,
    bench_default,
    bench_serialize,
    bench_deserialize,
    bench_roundtrip
);
criterion_main!(benches);
