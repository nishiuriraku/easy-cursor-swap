//! アプリケーションメタ情報 IPC (Wave 4B)
//!
//! フロントエンドに対し、MSIX パッケージ環境かどうかと policy label を返す。
//! Store 環境では Tauri Updater / 自動起動 / NSIS rollback 経路を
//! UI 側で非表示化するために利用する。

/// MSIX パッケージ環境で実行されているかを返す。
/// `appusermodel::is_msix_packaged_with_fallback` (API + path 2 段戦略) と同じ結果を返す。
#[tauri::command]
pub fn is_msix_packaged() -> bool {
    crate::appusermodel::PackageContext::current().is_msix
}

/// 現在の PackageContext に対応する "store" / "unpackaged" ラベルを返す。
/// フロントエンドは `packagePolicyLabel` を条件分岐のキーとして使う。
#[tauri::command]
pub fn package_policy_label() -> String {
    crate::appusermodel::PackageContext::current()
        .policy_label()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_msix_packaged_matches_package_context() {
        // IPC 経由と直呼び出しが一致することを保証。
        // (CI 環境では通常 unpackaged なので false == false)
        let via_ipc = is_msix_packaged();
        let via_ctx = crate::appusermodel::PackageContext::current().is_msix;
        assert_eq!(via_ipc, via_ctx);
    }

    #[test]
    fn package_policy_label_is_known() {
        let label = package_policy_label();
        assert!(
            label == "store" || label == "unpackaged",
            "unexpected label: {label}"
        );
    }
}
