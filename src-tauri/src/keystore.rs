//! EasyCursorSwap Ed25519 鍵ペア管理 (Phase 6-3)
//!
//! クリエイターがテーマに署名するための鍵ペアを管理する。
//!
//! - 秘密鍵: `~/.custom_cursors/_keys/private.key` に **Windows DPAPI** で暗号化保存
//!   (`CryptProtectData` でユーザーアカウント紐付き暗号化、他ユーザー復号不可)
//! - 公開鍵: 同フォルダの `public.key` に Base64 平文保存
//! - `key_id`: 公開鍵の SHA-256 先頭 16 文字。テーマメタや公開鍵レコード参照に使用。
//!
//! ## 仕様書との対応
//! - 鍵生成は OS の CSPRNG (`OsRng`) を使用
//! - エクスポート時はパスフレーズ付き (Argon2 + ChaCha20Poly1305 で暗号化) ※将来
//!   現状は DPAPI 解除済みの生バイトをそのまま吐く実装は避け、未対応とする。

use crate::config::ConfigManager;
use crate::errors::{AppError, AppResult};
use base64::Engine as _;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// 公開鍵 / 秘密鍵保管ディレクトリ
fn keys_dir() -> AppResult<PathBuf> {
    Ok(ConfigManager::cursors_dir()?.join("_keys"))
}

fn private_key_path() -> AppResult<PathBuf> {
    Ok(keys_dir()?.join("private.key"))
}

fn public_key_path() -> AppResult<PathBuf> {
    Ok(keys_dir()?.join("public.key"))
}

/// 鍵ペアの存在を表すサマリー (UI 表示用)
#[derive(Debug, Clone, serde::Serialize)]
pub struct KeystoreInfo {
    pub has_keypair: bool,
    pub key_id: Option<String>,
    pub public_key_b64: Option<String>,
}

pub struct Keystore;

impl Keystore {
    /// 鍵ペアの状態を返す。秘密鍵は DPAPI で暗号化されているのでここでは復号しない。
    pub fn info() -> AppResult<KeystoreInfo> {
        let pub_path = public_key_path()?;
        let priv_path = private_key_path()?;
        let has = pub_path.exists() && priv_path.exists();
        if !has {
            return Ok(KeystoreInfo {
                has_keypair: false,
                key_id: None,
                public_key_b64: None,
            });
        }
        let pub_b64 = std::fs::read_to_string(&pub_path)?.trim().to_string();
        let key_id = compute_key_id(&pub_b64)?;
        Ok(KeystoreInfo {
            has_keypair: true,
            key_id: Some(key_id),
            public_key_b64: Some(pub_b64),
        })
    }

    /// 新規鍵ペアを生成して保存する。既存があれば上書きしない (`force=true` で上書き可)。
    pub fn generate(force: bool) -> AppResult<KeystoreInfo> {
        let pub_path = public_key_path()?;
        let priv_path = private_key_path()?;
        if !force && pub_path.exists() && priv_path.exists() {
            return Self::info();
        }
        std::fs::create_dir_all(keys_dir()?)?;

        // OS の CSPRNG から鍵を生成
        use rand::rngs::OsRng;
        use rand::TryRngCore;
        let mut sk_bytes = [0u8; 32];
        OsRng
            .try_fill_bytes(&mut sk_bytes)
            .map_err(|e| AppError::Theme(format!("CSPRNG 失敗: {}", e)))?;
        let signing = SigningKey::from_bytes(&sk_bytes);
        let verifying = signing.verifying_key();

        // 秘密鍵を DPAPI 暗号化して保存
        let encrypted = dpapi_encrypt(&signing.to_bytes())?;
        std::fs::write(&priv_path, &encrypted)?;
        // 秘密鍵ファイルのアクセス制限はファイルシステム ACL で別途守る
        // (HKCU 配下なので OS デフォルトで他ユーザーは読めない)

        // 公開鍵は Base64 平文
        let pub_b64 = base64::engine::general_purpose::STANDARD.encode(verifying.to_bytes());
        std::fs::write(&pub_path, &pub_b64)?;

        let key_id = compute_key_id(&pub_b64)?;
        tracing::info!("Ed25519 鍵ペアを生成しました key_id={}", key_id);
        Ok(KeystoreInfo {
            has_keypair: true,
            key_id: Some(key_id),
            public_key_b64: Some(pub_b64),
        })
    }

    /// 秘密鍵をパスフレーズ付きでエクスポートする。
    ///
    /// フォーマット (バージョン 1):
    /// ```text
    /// CFKEY1\n               (8 bytes magic + newline)
    /// {salt: 16 bytes}{nonce: 24 bytes}{ciphertext: 32+16 bytes (AEAD tag 含む)}
    /// ```
    /// 暗号化:
    ///  - KDF: Argon2id (m=64MiB, t=3, p=1) で salt + passphrase → 32 byte key
    ///  - AEAD: XChaCha20-Poly1305 (24 byte nonce で乱数衝突耐性)
    ///
    /// パスフレーズが空文字列の場合は `Err` を返す。
    pub fn export_private_key(passphrase: &str) -> AppResult<Vec<u8>> {
        use crate::errors::AppError;

        if passphrase.is_empty() {
            return Err(AppError::Theme(
                "パスフレーズを指定してください".to_string(),
            ));
        }

        let priv_path = private_key_path()?;
        if !priv_path.exists() {
            return Err(AppError::Theme(
                "鍵ペアが存在しません。先に生成してください".to_string(),
            ));
        }

        // DPAPI から平文 32 バイト秘密鍵を取得
        let encrypted = std::fs::read(&priv_path)?;
        let raw = dpapi_decrypt(&encrypted)?;
        if raw.len() != 32 {
            return Err(AppError::Theme(format!(
                "秘密鍵の長さが不正: {} bytes",
                raw.len()
            )));
        }

        // Salt と Nonce を生成
        use rand::TryRngCore;
        let mut salt = [0u8; 16];
        let mut nonce = [0u8; 24];
        rand::rngs::OsRng
            .try_fill_bytes(&mut salt)
            .map_err(|e| AppError::Theme(format!("CSPRNG salt 失敗: {}", e)))?;
        rand::rngs::OsRng
            .try_fill_bytes(&mut nonce)
            .map_err(|e| AppError::Theme(format!("CSPRNG nonce 失敗: {}", e)))?;

        // KDF: Argon2id でパスフレーズから 32 byte 鍵を導出
        let key = derive_key_from_passphrase(passphrase, &salt)?;

        // XChaCha20-Poly1305 で暗号化
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};
        let cipher = XChaCha20Poly1305::new(&key.into());
        let xnonce = XNonce::from_slice(&nonce);
        let ciphertext = cipher
            .encrypt(xnonce, raw.as_slice())
            .map_err(|e| AppError::Theme(format!("AEAD 暗号化失敗: {}", e)))?;

        // フォーマット: magic + salt + nonce + ciphertext
        let mut out = Vec::with_capacity(8 + 16 + 24 + ciphertext.len());
        out.extend_from_slice(b"CFKEY1\n\0");
        out.extend_from_slice(&salt);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);

        tracing::info!("秘密鍵をエクスポートしました ({} bytes)", out.len());
        Ok(out)
    }

    /// パスフレーズ付きエクスポートデータを読み込んで秘密鍵をインポートする。
    /// 既存鍵があれば上書きする (UI 側で確認ダイアログを出すこと)。
    pub fn import_private_key(blob: &[u8], passphrase: &str) -> AppResult<KeystoreInfo> {
        use crate::errors::AppError;
        use ed25519_dalek::SigningKey;

        if passphrase.is_empty() {
            return Err(AppError::Theme(
                "パスフレーズを指定してください".to_string(),
            ));
        }
        const MAGIC: &[u8] = b"CFKEY1\n\0";
        if blob.len() < MAGIC.len() + 16 + 24 + 16 {
            return Err(AppError::Theme(
                "エクスポートデータが短すぎます".to_string(),
            ));
        }
        if &blob[..MAGIC.len()] != MAGIC {
            return Err(AppError::Theme(
                "エクスポートデータの形式が不正です (magic 不一致)".to_string(),
            ));
        }

        let body = &blob[MAGIC.len()..];
        let salt = &body[..16];
        let nonce = &body[16..40];
        let ciphertext = &body[40..];

        let key = derive_key_from_passphrase(passphrase, salt)?;

        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};
        let cipher = XChaCha20Poly1305::new(&key.into());
        let xnonce = XNonce::from_slice(nonce);
        let plaintext = cipher
            .decrypt(xnonce, ciphertext)
            .map_err(|_| AppError::Theme("復号失敗 (パスフレーズが違います)".to_string()))?;

        if plaintext.len() != 32 {
            return Err(AppError::Theme(format!(
                "復号結果の長さが不正: {} bytes",
                plaintext.len()
            )));
        }
        let mut sk_bytes = [0u8; 32];
        sk_bytes.copy_from_slice(&plaintext);
        let signing = SigningKey::from_bytes(&sk_bytes);
        let verifying = signing.verifying_key();

        // 保存先ディレクトリ確保
        std::fs::create_dir_all(keys_dir()?)?;

        // 秘密鍵を DPAPI 暗号化して保存
        let dpapi_encrypted = dpapi_encrypt(&signing.to_bytes())?;
        std::fs::write(private_key_path()?, &dpapi_encrypted)?;

        // 公開鍵を Base64 平文保存
        let pub_b64 = base64::engine::general_purpose::STANDARD.encode(verifying.to_bytes());
        std::fs::write(public_key_path()?, &pub_b64)?;

        let key_id = compute_key_id(&pub_b64)?;
        tracing::info!("秘密鍵をインポートしました key_id={}", key_id);
        Ok(KeystoreInfo {
            has_keypair: true,
            key_id: Some(key_id),
            public_key_b64: Some(pub_b64),
        })
    }

    /// 鍵ペアを削除する。
    pub fn delete() -> AppResult<()> {
        let pub_path = public_key_path()?;
        let priv_path = private_key_path()?;
        if pub_path.exists() {
            std::fs::remove_file(&pub_path)?;
        }
        if priv_path.exists() {
            std::fs::remove_file(&priv_path)?;
        }
        tracing::info!("Ed25519 鍵ペアを削除しました");
        Ok(())
    }

    /// メッセージに署名する (Base64 で返す)。
    pub fn sign(message: &[u8]) -> AppResult<String> {
        let priv_path = private_key_path()?;
        let encrypted = std::fs::read(&priv_path)?;
        let raw = dpapi_decrypt(&encrypted)?;
        if raw.len() != 32 {
            return Err(AppError::Theme(format!(
                "秘密鍵の長さが不正: {} bytes",
                raw.len()
            )));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&raw);
        let signing = SigningKey::from_bytes(&bytes);
        let sig: Signature = signing.sign(message);
        Ok(base64::engine::general_purpose::STANDARD.encode(sig.to_bytes()))
    }

    /// 公開鍵で検証する。デバッグ / 自己テスト用。
    pub fn verify(message: &[u8], signature_b64: &str) -> AppResult<bool> {
        let pub_path = public_key_path()?;
        if !pub_path.exists() {
            return Ok(false);
        }
        let pub_b64 = std::fs::read_to_string(&pub_path)?.trim().to_string();
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&pub_b64)
            .map_err(|e| AppError::Theme(format!("公開鍵 Base64 デコード失敗: {}", e)))?;
        let bytes: [u8; 32] = raw
            .as_slice()
            .try_into()
            .map_err(|_| AppError::Theme("公開鍵長が不正".to_string()))?;
        let verifying = VerifyingKey::from_bytes(&bytes)
            .map_err(|e| AppError::Theme(format!("公開鍵パース失敗: {}", e)))?;
        let sig_raw = base64::engine::general_purpose::STANDARD
            .decode(signature_b64)
            .map_err(|e| AppError::Theme(format!("署名 Base64 デコード失敗: {}", e)))?;
        let sig_bytes: [u8; 64] = sig_raw
            .as_slice()
            .try_into()
            .map_err(|_| AppError::Theme("署名長が不正".to_string()))?;
        let signature = Signature::from_bytes(&sig_bytes);
        use ed25519_dalek::Verifier;
        Ok(verifying.verify(message, &signature).is_ok())
    }
}

/// パスフレーズ + salt から Argon2id で 32 byte 鍵を導出する。
/// パラメータ (m=64MiB, t=3, p=1) は OWASP 推奨の保守的な設定。
fn derive_key_from_passphrase(passphrase: &str, salt: &[u8]) -> AppResult<[u8; 32]> {
    use crate::errors::AppError;
    use argon2::{Algorithm, Argon2, Params, Version};

    let params = Params::new(64 * 1024, 3, 1, Some(32))
        .map_err(|e| AppError::Theme(format!("Argon2 params 不正: {}", e)))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut out)
        .map_err(|e| AppError::Theme(format!("Argon2 派生失敗: {}", e)))?;
    Ok(out)
}

/// 公開鍵 Base64 → key_id (公開鍵 SHA-256 の先頭 16 文字)
pub fn compute_key_id(pubkey_b64: &str) -> AppResult<String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(pubkey_b64)
        .map_err(|e| AppError::Theme(format!("公開鍵 Base64 デコード失敗: {}", e)))?;
    Ok(hex::encode(Sha256::digest(&raw))[..16].to_string())
}

// ----------------------------------------------------------------------------
// DPAPI ラッパー
// ----------------------------------------------------------------------------

#[cfg(windows)]
fn dpapi_encrypt(plain: &[u8]) -> AppResult<Vec<u8>> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Foundation::HLOCAL;
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptProtectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|e| AppError::Theme(format!("DPAPI Protect 失敗: {}", e)))?;
    }

    let slice = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let result = slice.to_vec();

    unsafe {
        let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
    }
    Ok(result)
}

#[cfg(windows)]
fn dpapi_decrypt(cipher: &[u8]) -> AppResult<Vec<u8>> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Foundation::HLOCAL;
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: cipher.len() as u32,
        pbData: cipher.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|e| AppError::Theme(format!("DPAPI Unprotect 失敗: {}", e)))?;
    }

    let slice = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let result = slice.to_vec();

    unsafe {
        let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
    }
    Ok(result)
}

#[cfg(not(windows))]
fn dpapi_encrypt(_plain: &[u8]) -> AppResult<Vec<u8>> {
    Err(AppError::Theme("DPAPI は Windows 専用です".to_string()))
}

#[cfg(not(windows))]
fn dpapi_decrypt(_cipher: &[u8]) -> AppResult<Vec<u8>> {
    Err(AppError::Theme("DPAPI は Windows 専用です".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_key_id_returns_16_lowercase_hex() {
        let pk = base64::engine::general_purpose::STANDARD.encode([0xABu8; 32]);
        let id = compute_key_id(&pk).unwrap();
        assert_eq!(id.len(), 16);
        assert!(id
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn compute_key_id_same_input_same_output() {
        let pk = base64::engine::general_purpose::STANDARD.encode([1u8; 32]);
        assert_eq!(compute_key_id(&pk).unwrap(), compute_key_id(&pk).unwrap());
    }

    #[test]
    fn compute_key_id_distinct_inputs_distinct_outputs() {
        let a = base64::engine::general_purpose::STANDARD.encode([1u8; 32]);
        let b = base64::engine::general_purpose::STANDARD.encode([2u8; 32]);
        assert_ne!(compute_key_id(&a).unwrap(), compute_key_id(&b).unwrap());
    }

    #[test]
    fn compute_key_id_matches_marketplace_implementation() {
        // keystore::compute_key_id と marketplace::compute_key_id は別実装
        // (前者は鍵ペア生成時、後者は MarketplaceClient::install 時に使用) だが、
        // 同じ仕様 = 「公開鍵バイトの SHA-256 先頭 16 文字」のはず。
        let pk = base64::engine::general_purpose::STANDARD.encode([42u8; 32]);
        assert_eq!(
            compute_key_id(&pk).unwrap(),
            crate::marketplace::compute_key_id(&pk).unwrap()
        );
    }

    #[test]
    fn compute_key_id_rejects_invalid_base64() {
        assert!(compute_key_id("!!!not base64!!!").is_err());
    }

    #[test]
    fn derive_key_from_passphrase_is_deterministic() {
        // 同じパスフレーズと salt で何度呼んでも同じ 32 byte 鍵が得られる。
        let salt = [0u8; 16];
        let k1 = derive_key_from_passphrase("hunter2", &salt).unwrap();
        let k2 = derive_key_from_passphrase("hunter2", &salt).unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn derive_key_from_passphrase_changes_with_salt() {
        // salt を変えると別の鍵が出る (rainbow table 防御)。
        let k1 = derive_key_from_passphrase("hunter2", &[0u8; 16]).unwrap();
        let k2 = derive_key_from_passphrase("hunter2", &[1u8; 16]).unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn derive_key_from_passphrase_changes_with_passphrase() {
        let salt = [7u8; 16];
        let k1 = derive_key_from_passphrase("alpha", &salt).unwrap();
        let k2 = derive_key_from_passphrase("beta", &salt).unwrap();
        assert_ne!(k1, k2);
    }
}
