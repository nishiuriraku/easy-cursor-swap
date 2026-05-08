//! Marketplace 投稿用に `.cursorpack` の SHA-256 を Ed25519 で署名する一時 CLI。
//!
//! 用途:
//!   - `easy-cursor-swap-index` への初回登録用に、ローカル DPAPI 保管の鍵を使って
//!     SHA-256 hex 文字列に対する Ed25519 署名を生成する。
//!   - 鍵ペアが未生成なら自動生成する (`Keystore::generate(false)`)。既存の鍵は維持。
//!
//! 注意:
//!   - 署名対象は `validate.mjs` および `marketplace::install` 系の仕様に従い
//!     **SHA-256 hex の小文字文字列の UTF-8 バイト** (生 32 byte ハッシュではない)。
//!   - 出力は stdout への JSON 1 行。stderr に進捗ログが出る。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use app_lib::keystore::Keystore;
use sha2::{Digest, Sha256};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: sign_for_marketplace <pack_path>");
        return ExitCode::from(2);
    }

    let pack_path = PathBuf::from(&args[1]);
    if !pack_path.exists() {
        eprintln!("error: ファイルが存在しません: {}", pack_path.display());
        return ExitCode::from(2);
    }

    // 鍵ペアの存在確認 / なければ生成 (force=false で既存維持)
    let info = match Keystore::generate(false) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: 鍵ペア準備失敗: {e}");
            return ExitCode::from(1);
        }
    };
    let key_id = match info.key_id.clone() {
        Some(k) => k,
        None => {
            eprintln!("error: key_id が取得できませんでした");
            return ExitCode::from(1);
        }
    };
    let public_key = match info.public_key_b64.clone() {
        Some(p) => p,
        None => {
            eprintln!("error: public_key が取得できませんでした");
            return ExitCode::from(1);
        }
    };

    eprintln!("[sign] key_id={key_id}");

    // SHA-256 hex (小文字) を計算
    let bytes = match fs::read(&pack_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: ファイル読み込み失敗: {e}");
            return ExitCode::from(1);
        }
    };
    let size_bytes = bytes.len();
    let sha = Sha256::digest(&bytes);
    let sha_hex = hex::encode(sha);
    eprintln!(
        "[sign] file={} size={} sha256={}",
        pack_path.display(),
        size_bytes,
        sha_hex
    );

    // 署名対象 = SHA-256 hex 小文字文字列の UTF-8 バイト列
    let message = sha_hex.as_bytes();
    let signature = match Keystore::sign(message) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: 署名失敗: {e}");
            return ExitCode::from(1);
        }
    };

    // 自己検証 (誤りがあれば validate.mjs 通過前に気付ける)
    match Keystore::verify(message, &signature) {
        Ok(true) => eprintln!("[sign] 自己検証 OK"),
        Ok(false) => {
            eprintln!("error: 自己検証 NG (公開鍵と署名が整合しません)");
            return ExitCode::from(1);
        }
        Err(e) => {
            eprintln!("warn: 自己検証エラー (継続): {e}");
        }
    }

    // 出力 JSON (フィールド順を安定させる)
    let out = serde_json::json!({
        "key_id": key_id,
        "public_key": public_key,
        "signature": signature,
        "sha256": sha_hex,
        "size_bytes": size_bytes,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    ExitCode::SUCCESS
}
