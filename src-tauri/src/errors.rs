//! EasyCursorSwap エラー型定義
//!
//! アプリケーション全体で使用するエラー型を一元管理

use thiserror::Error;

/// アプリケーション全体のエラー型
#[derive(Error, Debug)]
pub enum AppError {
    /// 設定ファイルの読み書きエラー
    #[error("設定エラー: {0}")]
    Config(String),

    /// レジストリ操作エラー
    #[error("レジストリエラー: {0}")]
    Registry(String),

    /// 画像処理エラー
    #[error("画像処理エラー: {0}")]
    ImageProcessing(String),

    /// テーマパッケージエラー
    #[error("テーマエラー: {0}")]
    Theme(String),

    /// ファイルI/Oエラー
    #[error("ファイルI/Oエラー: {0}")]
    Io(#[from] std::io::Error),

    /// JSONシリアライズ/デシリアライズエラー
    #[error("JSONエラー: {0}")]
    Json(#[from] serde_json::Error),

    /// Zipアーカイブエラー
    #[error("Zipエラー: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// 不正な入力 (URL スキーム違反など)
    #[error("入力エラー: {0}")]
    InvalidInput(String),

    /// その他のエラー
    #[error("{0}")]
    Other(String),

    /// 一括インポートがユーザー操作で中断されたとき。
    #[error("一括インポートが中断されました")]
    BulkImportCancelled,

    /// 指定パス配下に対応拡張子のファイルがなかったとき。
    #[error("対応ファイルが見つかりません: {path}")]
    NoSupportedFiles { path: String },

    /// 個別ファイルが MAX_FILE_BYTES を超えたとき。
    #[error("サイズ上限超過: {path} ({size} bytes)")]
    OversizeFile { path: String, size: u64 },

    /// .cursorpack ZIP / metadata が壊れているとき。
    #[error(".cursorpack の解析に失敗: {reason}")]
    InvalidCursorpack { reason: String },

    /// 暗号化 / 鍵操作 (Ed25519 / DPAPI / Argon2id / XChaCha20-Poly1305 など) の失敗。
    /// フロントには `crypto: <理由>` 形式でシリアライズされる。
    #[error("crypto: {0}")]
    Crypto(String),

    /// GitHub REST API / Device Flow / PR 作成などの失敗。
    /// フロントには `github: <理由>` 形式でシリアライズされる。
    #[error("github: {0}")]
    GitHub(String),
}

/// Tauri IPC 向けのシリアライズ可能エラー
/// Tauri の invoke ハンドラから返すため Serialize が必要
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// `tauri::Error` を汎用 `AppError::Other` に変換する。
///
/// トレイ初期化など `Box<dyn Error>` だった経路で `?` をそのまま使えるようにするための
/// フォールバック。マッピングが確定しているエラー (`Crypto` / `GitHub` 等) は
/// 呼び出し側で明示的に `.map_err()` すること。
impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Other(format!("tauri エラー: {}", e))
    }
}

/// 結果型のエイリアス
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// `AppError::Crypto` / `AppError::GitHub` の表示・シリアライズ契約。
    ///
    /// 表示プレフィクスは lowercase の `crypto: ` / `github: ` で固定する。
    /// フロント側は文字列マッチでハンドリングしているため、プレフィクスを
    /// 変更すると既存ハンドラが壊れる (i18n キーを増やさない方針なので
    /// ここを揺らさない)。
    #[test]
    fn crypto_display_prefix_is_lowercase() {
        let e = AppError::Crypto("Ed25519 生成失敗".to_string());
        assert_eq!(e.to_string(), "crypto: Ed25519 生成失敗");
    }

    #[test]
    fn github_display_prefix_is_lowercase() {
        let e = AppError::GitHub("POST forks 401".to_string());
        assert_eq!(e.to_string(), "github: POST forks 401");
    }

    /// シリアライズ結果が `to_string()` と完全一致することを保証する。
    /// フロントはエラー文字列で分岐するため、JSON 形を変えてはいけない。
    #[test]
    fn crypto_serialization_matches_display() {
        let e = AppError::Crypto("DPAPI 失敗".to_string());
        let s = serde_json::to_string(&e).unwrap();
        assert_eq!(s, "\"crypto: DPAPI 失敗\"");
    }

    #[test]
    fn github_serialization_matches_display() {
        let e = AppError::GitHub("GET /user タイムアウト".to_string());
        let s = serde_json::to_string(&e).unwrap();
        assert_eq!(s, "\"github: GET /user タイムアウト\"");
    }

    /// 既存バリアント (`Theme` / `InvalidInput` 等) の表示形をリグレッション検出用に固定する。
    /// 新バリアント追加時に既存の文字列を壊していないか確認する。
    #[test]
    fn existing_variants_remain_unchanged() {
        assert_eq!(
            AppError::Theme("壊れた JSON".to_string()).to_string(),
            "テーマエラー: 壊れた JSON"
        );
        assert_eq!(
            AppError::InvalidInput("javascript:foo".to_string()).to_string(),
            "入力エラー: javascript:foo"
        );
    }
}
