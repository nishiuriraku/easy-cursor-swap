//! 連続起動失敗時の自動ロールバック処理。
//!
//! `main.rs::show_rollback_dialog` から呼ばれる pre-main フェーズの fn 群:
//!   1. `download_to_temp` — installer + .sig を `%TEMP%` に DL
//!   2. `verify_minisign` — Tauri Updater と同じ engine (minisign-verify) で検証
//!   3. `launch_silent_installer` — `/S` (NSIS) or `/quiet` (MSI) で起動
//!
//! Tauri ランタイム未起動なので Tauri Dialog 使用不可。
//! 失敗時は `Result::Err` を返し、呼び出し元の MessageBox fallback に委ねる。

use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;

/// `tauri.conf.json` の `plugins.updater.pubkey` をビルド時に埋め込む。
/// 鍵ローテーション時はビルドし直しで反映される。
/// `easycursorswap.pub` は base64 でラップ済みの minisign フォーマット 1 行ファイル。
pub const EMBEDDED_PUBKEY: &str = include_str!("../signing/easycursorswap.pub");

#[derive(Debug, Error)]
pub enum RollbackError {
    #[error("ダウンロードに失敗: {0}")]
    Download(String),
    #[error("一時ファイルの書き出しに失敗: {0}")]
    Io(#[from] std::io::Error),
    #[error("minisign 検証失敗: {0}")]
    Verify(String),
    #[error("インストーラの起動に失敗: {0}")]
    Spawn(String),
}

/// minisign 公開鍵と署名 (`.sig` 内容) で `data` を検証する。
///
/// `tauri-plugin-updater` 内部の `verify_signature` と同じ engine を使うので
/// 同じ署名フォーマット (untrusted comment + base64 + trusted comment + base64)
/// を扱える。`pubkey` も `tauri.conf.json` に書かれている base64 文字列のまま渡してよい。
pub fn verify_minisign(data: &[u8], sig: &str, pubkey: &str) -> Result<(), RollbackError> {
    use base64::Engine;
    use minisign_verify::{PublicKey, Signature};
    // pubkey は base64 で 1 段ラップされた minisign テキスト。
    let pk_text_bytes = base64::engine::general_purpose::STANDARD
        .decode(pubkey.trim())
        .map_err(|e| RollbackError::Verify(format!("pubkey base64 decode: {e}")))?;
    let pk_text = std::str::from_utf8(&pk_text_bytes)
        .map_err(|e| RollbackError::Verify(format!("pubkey utf8: {e}")))?;
    let pk = PublicKey::decode(pk_text)
        .map_err(|e| RollbackError::Verify(format!("pubkey decode: {e}")))?;
    // signature は base64 で 1 段ラップされた minisign テキスト (latest.json 同様)。
    let sig_text_bytes = base64::engine::general_purpose::STANDARD
        .decode(sig.trim())
        .map_err(|e| RollbackError::Verify(format!("sig base64 decode: {e}")))?;
    let sig_text = std::str::from_utf8(&sig_text_bytes)
        .map_err(|e| RollbackError::Verify(format!("sig utf8: {e}")))?;
    let sig_decoded = Signature::decode(sig_text)
        .map_err(|e| RollbackError::Verify(format!("signature decode: {e}")))?;
    pk.verify(data, &sig_decoded, true)
        .map_err(|e| RollbackError::Verify(format!("verify: {e}")))?;
    Ok(())
}

/// 30 秒タイムアウト付きで URL を `%TEMP%/<filename>` に DL し、書き出した PathBuf を返す。
pub fn download_to_temp(url: &str, filename: &str) -> Result<PathBuf, RollbackError> {
    let mut path = std::env::temp_dir();
    path.push(filename);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| RollbackError::Download(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .map_err(|e| RollbackError::Download(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(RollbackError::Download(format!("HTTP {}", resp.status())));
    }
    let bytes = resp
        .bytes()
        .map_err(|e| RollbackError::Download(e.to_string()))?;
    std::fs::write(&path, &bytes)?;
    Ok(path)
}

/// `.exe` なら `/S` (NSIS silent)、`.msi` なら `msiexec /i ... /quiet` で起動。
/// 起動成功後は呼び元が `std::process::exit(0)` する想定 (本 fn は spawn だけ)。
#[cfg(windows)]
pub fn launch_silent_installer(installer: &Path) -> Result<(), RollbackError> {
    use std::process::Command;
    let ext = installer.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        "exe" => {
            Command::new(installer)
                .arg("/S")
                .spawn()
                .map_err(|e| RollbackError::Spawn(format!("NSIS spawn: {e}")))?;
        }
        "msi" => {
            Command::new("msiexec")
                .args(["/i"])
                .arg(installer)
                .args(["/quiet", "/norestart"])
                .spawn()
                .map_err(|e| RollbackError::Spawn(format!("msiexec spawn: {e}")))?;
        }
        _ => return Err(RollbackError::Spawn(format!("未対応の拡張子: {ext}"))),
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn launch_silent_installer(_installer: &Path) -> Result<(), RollbackError> {
    Err(RollbackError::Spawn("Windows 以外では未対応".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_minisign_returns_err_on_garbage_pubkey() {
        // pubkey が base64 として無効な文字列 → エラー
        let err = verify_minisign(b"hello", "garbage_sig", "garbage_pubkey").unwrap_err();
        assert!(matches!(err, RollbackError::Verify(_)));
    }

    #[test]
    fn verify_minisign_returns_err_on_empty_inputs() {
        let err = verify_minisign(b"", "", "").unwrap_err();
        assert!(matches!(err, RollbackError::Verify(_)));
    }

    #[test]
    fn verify_minisign_returns_err_on_invalid_signature_format() {
        // pubkey は正規 (リポに含まれる) だが signature が不正な形式
        let err = verify_minisign(b"hello", "not-minisign-format", EMBEDDED_PUBKEY).unwrap_err();
        assert!(matches!(err, RollbackError::Verify(_)));
    }

    #[test]
    fn embedded_pubkey_is_loaded_at_compile_time() {
        // include_str! が空でないこと + minisign フォーマットへデコード可能なことを確認
        assert!(EMBEDDED_PUBKEY.len() > 50);
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(EMBEDDED_PUBKEY.trim())
            .expect("EMBEDDED_PUBKEY should be base64");
        let text = std::str::from_utf8(&decoded).expect("decoded should be UTF-8 minisign text");
        assert!(text.contains("untrusted comment"));
        assert!(text.contains("minisign public key"));
    }
}
