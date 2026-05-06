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

    #[error("一括インポートが中断されました")]
    BulkImportCancelled,

    #[error("対応ファイルが見つかりません: {path}")]
    NoSupportedFiles { path: String },

    #[error("サイズ上限超過: {path} ({size} bytes)")]
    OversizeFile { path: String, size: u64 },

    #[error(".cursorpack の解析に失敗: {reason}")]
    InvalidCursorpack { reason: String },
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

/// 結果型のエイリアス
pub type AppResult<T> = Result<T, AppError>;
