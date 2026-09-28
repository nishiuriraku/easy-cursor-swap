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

    /// 非 Windows ビルドの OS 機能スタブ (P01 / P02) が返す。実行時に Windows で出ることは無い。
    #[error("この操作は Windows 専用です: {0}")]
    UnsupportedPlatform(String),
}

use std::collections::BTreeMap;

/// IPC 境界で安定したエラー種別。値は snake_case 文字列で TS 側 `AppErrorCode` と 1:1。
/// **既存の値を改名・削除しない** (フロントの `errors.<code>` i18n キーと結合)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(export))]
pub enum AppErrorCode {
    Config,
    Registry,
    ImageProcessing,
    Theme,
    Io,
    Json,
    Zip,
    InvalidInput,
    Other,
    BulkImportCancelled,
    NoSupportedFiles,
    OversizeFile,
    InvalidCursorpack,
    Crypto,
    Github,
    UnsupportedPlatform,
}

/// `AppError` の IPC 表現。`message` は `Display` と同一 (ログと突合可能)。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "typegen", derive(ts_rs::TS))]
#[cfg_attr(feature = "typegen", ts(export))]
pub struct AppErrorDto {
    pub code: AppErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typegen", ts(optional))]
    pub detail: Option<BTreeMap<String, String>>,
}

impl AppError {
    pub fn code(&self) -> AppErrorCode {
        match self {
            AppError::Config(_) => AppErrorCode::Config,
            AppError::Registry(_) => AppErrorCode::Registry,
            AppError::ImageProcessing(_) => AppErrorCode::ImageProcessing,
            AppError::Theme(_) => AppErrorCode::Theme,
            AppError::Io(_) => AppErrorCode::Io,
            AppError::Json(_) => AppErrorCode::Json,
            AppError::Zip(_) => AppErrorCode::Zip,
            AppError::InvalidInput(_) => AppErrorCode::InvalidInput,
            AppError::Other(_) => AppErrorCode::Other,
            AppError::BulkImportCancelled => AppErrorCode::BulkImportCancelled,
            AppError::NoSupportedFiles { .. } => AppErrorCode::NoSupportedFiles,
            AppError::OversizeFile { .. } => AppErrorCode::OversizeFile,
            AppError::InvalidCursorpack { .. } => AppErrorCode::InvalidCursorpack,
            AppError::Crypto(_) => AppErrorCode::Crypto,
            AppError::GitHub(_) => AppErrorCode::Github,
            AppError::UnsupportedPlatform(_) => AppErrorCode::UnsupportedPlatform,
        }
    }

    pub fn detail(&self) -> Option<BTreeMap<String, String>> {
        match self {
            AppError::NoSupportedFiles { path } => {
                Some(BTreeMap::from([("path".to_string(), path.clone())]))
            }
            AppError::OversizeFile { path, size } => Some(BTreeMap::from([
                ("path".to_string(), path.clone()),
                ("size".to_string(), size.to_string()),
            ])),
            AppError::InvalidCursorpack { reason } => {
                Some(BTreeMap::from([("reason".to_string(), reason.clone())]))
            }
            _ => None,
        }
    }

    pub fn to_dto(&self) -> AppErrorDto {
        AppErrorDto {
            code: self.code(),
            message: self.to_string(),
            detail: self.detail(),
        }
    }
}

/// Tauri IPC 向けのシリアライズ可能エラー。
///
/// Tauri の invoke ハンドラからは `AppErrorDto` (`{code, message, detail?}`) として
/// 渡る。フロントは `app/utils/appError.ts` で受ける。`message` は `Display` と
/// 同一なので、ログ / ダイアログ (`e.to_string()`) と突合できる。
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_dto().serialize(serializer)
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
    /// フロントは `code` で分岐し `message` は表示専用。
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

    /// シリアライズ結果が DTO 形 (`{code, message}`) になることを保証する。
    /// `message` は `to_string()` と同一。`detail` は構造体 variant のみ。
    #[test]
    fn crypto_serialization_matches_display() {
        let e = AppError::Crypto("DPAPI 失敗".to_string());
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"code": "crypto", "message": "crypto: DPAPI 失敗"})
        );
        assert!(!v.as_object().unwrap().contains_key("detail"));
    }

    #[test]
    fn github_serialization_matches_display() {
        let e = AppError::GitHub("GET /user タイムアウト".to_string());
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"code": "github", "message": "github: GET /user タイムアウト"})
        );
        assert!(!v.as_object().unwrap().contains_key("detail"));
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

    /// 全 16 variant が安定した code 文字列に写像されることを固定する。
    /// variant 追加時は match に分岐を足さないとコンパイルエラーになる (`_ =>` 禁止)。
    #[test]
    fn every_variant_maps_to_a_stable_code() {
        let cases: Vec<(AppError, &str)> = vec![
            (AppError::Config("x".into()), "\"config\""),
            (AppError::Registry("x".into()), "\"registry\""),
            (
                AppError::ImageProcessing("x".into()),
                "\"image_processing\"",
            ),
            (AppError::Theme("x".into()), "\"theme\""),
            (AppError::Io(std::io::Error::other("x")), "\"io\""),
            (
                AppError::Json(serde_json::from_str::<serde_json::Value>("{").unwrap_err()),
                "\"json\"",
            ),
            (
                AppError::Zip(zip::result::ZipError::FileNotFound),
                "\"zip\"",
            ),
            (AppError::InvalidInput("x".into()), "\"invalid_input\""),
            (AppError::Other("x".into()), "\"other\""),
            (AppError::BulkImportCancelled, "\"bulk_import_cancelled\""),
            (
                AppError::NoSupportedFiles { path: "p".into() },
                "\"no_supported_files\"",
            ),
            (
                AppError::OversizeFile {
                    path: "p".into(),
                    size: 1,
                },
                "\"oversize_file\"",
            ),
            (
                AppError::InvalidCursorpack { reason: "r".into() },
                "\"invalid_cursorpack\"",
            ),
            (AppError::Crypto("x".into()), "\"crypto\""),
            (AppError::GitHub("x".into()), "\"github\""),
            (
                AppError::UnsupportedPlatform("x".into()),
                "\"unsupported_platform\"",
            ),
        ];
        assert_eq!(cases.len(), 16);
        for (e, expected) in cases {
            assert_eq!(serde_json::to_string(&e.code()).unwrap(), expected);
        }
    }

    /// 構造体 variant は `detail` にフィールドを持つ。
    #[test]
    fn struct_variants_carry_detail() {
        let e = AppError::OversizeFile {
            path: "a".into(),
            size: 11,
        };
        let dto = e.to_dto();
        assert_eq!(dto.code, AppErrorCode::OversizeFile);
        let detail = dto.detail.unwrap();
        assert_eq!(detail.get("path").map(String::as_str), Some("a"));
        assert_eq!(detail.get("size").map(String::as_str), Some("11"));
        // プレーン variant には detail が付かない
        assert!(AppError::Theme("x".into()).to_dto().detail.is_none());
    }

    /// `AppErrorCode` 16 値の serde round-trip。
    #[test]
    fn code_round_trips_through_serde() {
        for code in [
            AppErrorCode::Config,
            AppErrorCode::Registry,
            AppErrorCode::ImageProcessing,
            AppErrorCode::Theme,
            AppErrorCode::Io,
            AppErrorCode::Json,
            AppErrorCode::Zip,
            AppErrorCode::InvalidInput,
            AppErrorCode::Other,
            AppErrorCode::BulkImportCancelled,
            AppErrorCode::NoSupportedFiles,
            AppErrorCode::OversizeFile,
            AppErrorCode::InvalidCursorpack,
            AppErrorCode::Crypto,
            AppErrorCode::Github,
            AppErrorCode::UnsupportedPlatform,
        ] {
            let s = serde_json::to_string(&code).unwrap();
            let back: AppErrorCode = serde_json::from_str(&s).unwrap();
            assert_eq!(back, code);
        }
    }

    #[test]
    fn unsupported_platform_display() {
        assert_eq!(
            AppError::UnsupportedPlatform("x".to_string()).to_string(),
            "この操作は Windows 専用です: x"
        );
    }
}
