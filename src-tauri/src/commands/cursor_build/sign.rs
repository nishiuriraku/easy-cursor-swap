//! `.cursorpack` の Ed25519 署名フロー。
//!
//! `export_cursorpack` (非ストリーム) と `export_cursorpack_streamed` の
//! 両方から呼ばれる。元は両者で digest 計算と Keystore::sign の重複が
//! あったため、Phase 3b で本モジュールに統合した。

use crate::errors::{AppError, AppResult};
use crate::theme::ThemeMetadata;
use sha2::Digest;

/// `metadata` の `signature` フィールドを Ed25519 で埋める。
///
/// 戻値: 署名に使った `key_id` (Keystore::info から取得)。
/// 鍵ペアが無い場合は `Err(AppError::Theme)` を返す。
///
/// 署名対象は `id|version|sorted_role_names` の SHA-256 (hex 文字列)。
/// この計算と Keystore::sign 呼び出しは元来 `export_cursorpack` と
/// `export_cursorpack_streamed` の両者で重複していた。本関数に集約。
pub(super) fn sign_theme_metadata(metadata: &mut ThemeMetadata) -> AppResult<Option<String>> {
    let info = crate::keystore::Keystore::info()?;
    if !info.has_keypair {
        return Err(AppError::Theme(
            "鍵ペアがありません。設定 → 署名鍵 で生成してください".to_string(),
        ));
    }
    // 署名対象 = `id|version|sorted_role_names` の SHA-256 の hex 文字列
    let mut roles: Vec<&String> = metadata.cursors.keys().collect();
    roles.sort();
    let role_concat = roles
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let sign_input = format!("{}|{}|{}", metadata.id, metadata.version, role_concat);
    let digest = hex::encode(sha2::Sha256::digest(sign_input.as_bytes()));
    let sig = crate::keystore::Keystore::sign(digest.as_bytes())?;
    metadata.signature = Some(sig);
    Ok(info.key_id.clone())
}
