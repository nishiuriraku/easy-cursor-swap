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
    // L1-4: ファイルの有無を `Path::exists()` で前置チェックしない。
    // `exists()` + `read()` の間には TOCTOU ウィンドウ (例: シンボリックリンクの
    // 差し替え、並行削除) が生まれうる。`std::fs::read` の `io::Error` を
    // 単一の真実とし、`AppError::Io` 経由で伝播させる。
    let blob = read_import_blob(std::path::Path::new(&input_path))?;
    Keystore::import_private_key(&blob, &passphrase)
}

/// インポート用ブロブをディスクから読み込む。
///
/// 意図的に単一の `std::fs::read` 呼び出しのみとし、`Path::exists()` 等の
/// 前置チェックを持たない。読み込み失敗 (NotFound / PermissionDenied 等) は
/// `io::Error` のまま呼び出し側へ返し、`AppError::Io` に自動変換される。
///
/// ログ/PII 注意: 入力パスは秘匿情報を含むディレクトリヒントになり得るため
/// 生で `tracing!` などに出さない。`io::Error` の `Display` にはパスが
/// 含まれないため、伝播経路でのパス漏洩面は io エラー文字列に限定される。
fn read_import_blob(input_path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(input_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L1-4: 存在しないパスを渡したときに `read_import_blob` が
    /// `io::ErrorKind::NotFound` を返し、かつエラーメッセージに生のパスが
    /// 一切混入しないことを保証する。これは `Path::exists()` 前置チェックを
    /// 廃止したこと (TOCTOU ウィンドウ排除) を行動レベルで固定するテスト。
    /// 旧実装はカスタム `AppError::Theme` で raw path を文字列に埋め込んで
    /// おり PII 懸念があったが、新実装では io エラーが単一の真実となる。
    #[test]
    fn read_import_blob_missing_path_returns_io_not_found_without_leaking_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bogus = tmp.path().join("absent-secret-export-12345.cfkey");
        let bogus_str = bogus.to_string_lossy().into_owned();

        let err = read_import_blob(&bogus).expect_err("missing file must error");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);

        let msg = err.to_string();
        assert!(
            !msg.contains("absent-secret-export-12345"),
            "io error must not embed the raw input path; got: {}",
            msg
        );

        // 二重ガード: bogus_str を区切りトークンに分解し、いずれも io エラー
        // 文字列に含まれないことを確認 (to_string_lossy 由来のサブストリング含む)。
        for token in bogus_str.split(['/', '\\']) {
            if token.is_empty() {
                continue;
            }
            assert!(
                !msg.contains(token),
                "io error leaked path token {:?}; got: {}",
                token,
                msg
            );
        }
    }

    /// L1-4: 正常系はバイト列をそのまま返す (挙動互換)。暗号インポート経路
    /// (`Keystore::import_private_key`) の意味論には触れていないことを担保する。
    #[test]
    fn read_import_blob_existing_path_returns_bytes_unchanged() {
        let tmp = tempfile::TempDir::new().unwrap();
        let p = tmp.path().join("export.cfkey");
        std::fs::write(&p, b"hello-blob-payload").unwrap();

        let bytes = read_import_blob(&p).expect("existing file must read successfully");
        assert_eq!(bytes, b"hello-blob-payload");
    }
}
