//! ZIP 内エントリ名のパストラバーサル対策。
//!
//! `.cursorpack` / `.cursorprofile` 展開時に必須。
//! 拒否対象を網羅した [`sanitize_archive_path`] と
//! 公開ラッパー [`sanitize_archive_path_pub`] を提供する。

use crate::errors::{AppError, AppResult};

/// 公開ラッパー (`backup` モジュール用)。
pub fn sanitize_archive_path_pub(name: &str) -> AppResult<std::path::PathBuf> {
    sanitize_archive_path(name)
}

/// ZIP 内エントリ名を相対パスとして安全に解釈する。
///
/// 拒否ルール:
///  - 絶対パス (Unix `/foo`, Windows `C:\foo`)
///  - `..` を含むパス
///  - NUL バイトを含むパス
///  - Windows ドライブ指定 (`X:`) や UNC (`\\server\…`)
pub(crate) fn sanitize_archive_path(name: &str) -> AppResult<std::path::PathBuf> {
    use std::path::{Component, Path, PathBuf};

    if name.is_empty() {
        return Err(AppError::Theme("空のエントリ名".to_string()));
    }
    if name.contains('\0') {
        return Err(AppError::Theme(format!("NUL バイト混入: {}", name)));
    }
    // Windows のバックスラッシュ区切りも `..` 検出のために正規化
    let normalized = name.replace('\\', "/");
    if normalized.starts_with('/') {
        return Err(AppError::Theme(format!("絶対パス: {}", name)));
    }
    // ドライブ指定 (`C:`) や UNC をカバー
    if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        return Err(AppError::Theme(format!("ドライブ指定を含む: {}", name)));
    }

    let mut out = PathBuf::new();
    for comp in Path::new(&normalized).components() {
        match comp {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::Theme(format!("不正なパス成分: {}", name)));
            }
        }
    }

    if out.as_os_str().is_empty() {
        return Err(AppError::Theme(format!("正規化後に空: {}", name)));
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::sanitize_archive_path;

    #[test]
    fn rejects_path_traversal() {
        assert!(sanitize_archive_path("../etc/passwd").is_err());
        assert!(sanitize_archive_path("foo/../../bar").is_err());
    }

    #[test]
    fn rejects_absolute_paths() {
        assert!(sanitize_archive_path("/etc/passwd").is_err());
        assert!(sanitize_archive_path("C:\\Windows\\system.ini").is_err());
        assert!(sanitize_archive_path("\\\\server\\share").is_err());
    }

    #[test]
    fn accepts_normal_paths() {
        let p = sanitize_archive_path("cursors/Arrow.png").unwrap();
        assert_eq!(
            p.to_string_lossy(),
            "cursors\\Arrow.png".replace('\\', std::path::MAIN_SEPARATOR_STR)
        );
    }

    #[test]
    fn rejects_nul_byte() {
        assert!(sanitize_archive_path("foo\0bar").is_err());
    }
}
