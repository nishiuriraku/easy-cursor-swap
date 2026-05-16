//! ビルドスクリプト
//!
//! 1. tauri-build の通常処理
//! 2. リポジトリルート (`../`) の `.env` を読み込み、各種 credentials を
//!    `option_env!` で参照可能なコンパイル時 env として注入する
//!
//! 優先順位は **シェル env > .env**。CI (GitHub Actions) ではシェル env で
//! 渡されるため `.env` の有無に依存しない。

use std::path::PathBuf;

/// `.env` 経由 / シェル env 経由いずれかで埋め込みたい credentials の env 名。
///
/// - `EASY_CURSOR_SWAP_CRASH_REPORT_*` — クラッシュレポート Worker の URL と Token
/// - `EASY_CURSOR_SWAP_GITHUB_OAUTH_CLIENT_ID`           — Marketplace 自動提出フローの OAuth App ID
const EMBED_ENV_KEYS: [&str; 3] = [
    "EASY_CURSOR_SWAP_CRASH_REPORT_ENDPOINT",
    "EASY_CURSOR_SWAP_CRASH_REPORT_APP_TOKEN",
    "EASY_CURSOR_SWAP_GITHUB_OAUTH_CLIENT_ID",
];

fn main() {
    embed_compile_time_env();
    tauri_build::build()
}

fn embed_compile_time_env() {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by cargo");
    let repo_root = PathBuf::from(&manifest_dir)
        .parent()
        .expect("src-tauri は repo の直下にある想定")
        .to_path_buf();
    let env_path = repo_root.join(".env");

    // .env が変更されたら再ビルド
    println!("cargo:rerun-if-changed={}", env_path.display());

    // .env を読み込んでビルドスクリプトのプロセス env を埋める。
    // 既にシェル env で値が設定されている場合は **上書きしない** (CI 優先)。
    if env_path.exists() {
        // dotenvy::from_path は既存変数を上書きしないので CI と両立する
        if let Err(e) = dotenvy::from_path(&env_path) {
            // 構文エラーなどは警告だけ出してビルドは続行
            println!(
                "cargo:warning=.env の読み込みに失敗: {} ({})",
                env_path.display(),
                e
            );
        }
    }

    // 明示的に env が変わったら再ビルドさせ、値があれば rustc に渡す。
    // option_env! はコンパイル時マクロなので、rustc に env を見せないと展開時に None になる。
    for key in EMBED_ENV_KEYS {
        println!("cargo:rerun-if-env-changed={key}");
        if let Ok(val) = std::env::var(key) {
            println!("cargo:rustc-env={key}={val}");
        }
    }
}
