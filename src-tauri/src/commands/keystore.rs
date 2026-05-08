//! 鍵管理 (Ed25519) 系の IPC コマンド。
//!
//! 秘密鍵は DPAPI 暗号化された状態で `~/.custom_cursors/_keys/` に保存される。
//! エクスポート/インポートはパスフレーズ + XChaCha20-Poly1305 + Argon2id で再暗号化したバイト列で行う。

use crate::errors::AppError;
use crate::keystore::{Keystore, KeystoreInfo};

/// 鍵ペアの状態を返す。秘密鍵は DPAPI 暗号化されているので復号せずファイル存在のみ確認。
#[tauri::command]
pub fn keystore_info() -> Result<KeystoreInfo, AppError> {
    Keystore::info()
}

/// 新規 Ed25519 鍵ペアを生成して保存する。
/// `force=true` なら既存鍵を上書き。
#[tauri::command]
pub fn keystore_generate(force: bool) -> Result<KeystoreInfo, AppError> {
    Keystore::generate(force)
}

/// 鍵ペアを削除する (PC 移行や再発行のため)。
#[tauri::command]
pub fn keystore_delete() -> Result<(), AppError> {
    Keystore::delete()
}

/// 秘密鍵をパスフレーズ付きでエクスポートして指定パスに書き出す。
/// XChaCha20-Poly1305 + Argon2id でフォーマット化された不透明バイト列を保存。
#[tauri::command]
pub fn keystore_export(passphrase: String, output_path: String) -> Result<u64, AppError> {
    let blob = Keystore::export_private_key(&passphrase)?;
    let path = std::path::PathBuf::from(&output_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &blob)?;
    Ok(blob.len() as u64)
}

/// パスフレーズ付きエクスポートデータを読み込んで秘密鍵をインポート。
/// 既存鍵があれば上書きする。
#[tauri::command]
pub fn keystore_import(passphrase: String, input_path: String) -> Result<KeystoreInfo, AppError> {
    let path = std::path::PathBuf::from(&input_path);
    if !path.exists() {
        return Err(AppError::Theme(format!(
            "ファイルが見つかりません: {}",
            input_path
        )));
    }
    let blob = std::fs::read(&path)?;
    Keystore::import_private_key(&blob, &passphrase)
}
