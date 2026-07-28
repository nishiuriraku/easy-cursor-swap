//! AppUserModelID (AUMID) 明示登録 (Phase 7-2 残タスク)
//!
//! Windows のトースト通知 / ジャンプリスト / タスクバーグルーピングで使われる
//! プロセス識別子。`SetCurrentProcessExplicitAppUserModelID` を呼んで明示しておくと、
//! 通知センターで送信元アプリ名が EasyCursorSwap として正しく表示される。
//!
//! 仕様書 §「通知 UX」より:
//!  > AppUserModelID をマニフェストに登録 (MSIX は自動、`.msi` 版はインストーラで設定)
//!  > が、ここでは念のため起動時にも明示する。
//!
//! 参考: https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-setcurrentprocessexplicitappusermodelid

/// AppUserModelID 文字列 (`Vendor.Product.Subproduct.VersionInformation` 形式が推奨)。
/// `tauri.conf.json` の `identifier` (`dev.easycursorswap.app`) と整合させる。
#[cfg(windows)]
pub const APP_USER_MODEL_ID: &str = "dev.easycursorswap.app";

/// プロセスに AppUserModelID を設定する。
/// 失敗してもアプリ動作は継続 (通知元の表示が "Tauri アプリ" 等になるだけ)。
#[cfg(windows)]
pub fn register_aumid() {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let aumid = HSTRING::from(APP_USER_MODEL_ID);
    let result = unsafe { SetCurrentProcessExplicitAppUserModelID(&aumid) };
    match result {
        Ok(()) => {
            tracing::info!("AppUserModelID 設定: {}", APP_USER_MODEL_ID);
        }
        Err(e) => {
            tracing::warn!("AppUserModelID 設定失敗: {}", e);
        }
    }
}

#[cfg(not(windows))]
pub fn register_aumid() {
    // 非 Windows ではノーオペ
}

// --- Wave 4B: PackageContext 集約 ---

/// 自動起動の実装方針。MSIX 環境では OS 設定のスタートアップアプリで操作するため
/// アプリ側からの HKCU 書込みを no-op 化する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutostartPolicy {
    /// 自動起動を一切制御しない (MSIX など OS 側に委譲)。
    Disabled,
    /// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` への書込み
    /// (unpackaged 既定値)。
    HkeyRun,
    /// 将来用: MSIX `windows.startupTask` の API 制御。
    StartupTask,
}

impl AutostartPolicy {
    pub fn label(self) -> &'static str {
        match self {
            AutostartPolicy::Disabled => "disabled",
            AutostartPolicy::HkeyRun => "hkey-run",
            AutostartPolicy::StartupTask => "startup-task",
        }
    }
}

/// アプリ更新の実装方針。MSIX では Microsoft Store 経由の自動更新に委譲するため
/// Tauri Updater の check/install を抑止する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdaterPolicy {
    /// Tauri Updater 経路を完全に抑止 (MSIX)。
    Disabled,
    /// Tauri Updater を通常通り使用 (unpackaged 既定値)。
    Active,
}

impl UpdaterPolicy {
    pub fn label(self) -> &'static str {
        match self {
            UpdaterPolicy::Disabled => "disabled",
            UpdaterPolicy::Active => "active",
        }
    }
}

/// ヘルスロールバック (NSIS インストーラ download) の実装方針。MSIX では
/// Store 経由のリリース案内へ退避する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackPolicy {
    /// リリースページ (Web ブラウザ) を開くだけ (MSIX)。
    ReleasePage,
    /// NSIS インストーラ download + 自動適用 (unpackaged 既定値)。
    InstallerDownload,
}

impl RollbackPolicy {
    pub fn label(self) -> &'static str {
        match self {
            RollbackPolicy::ReleasePage => "release-page",
            RollbackPolicy::InstallerDownload => "installer-download",
        }
    }
}

/// 起動時のメインウィンドウ可視性。`start_minimized` 設定と `--autostart` 起動
/// 種別の組み合わせで決まる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationPolicy {
    /// 通常起動 (ウィンドウ可視)。
    Show,
    /// トレイ常駐で起動 (ウィンドウ不可視)。
    Hide,
}

impl ActivationPolicy {
    pub fn label(self) -> &'static str {
        match self {
            ActivationPolicy::Show => "show",
            ActivationPolicy::Hide => "hide",
        }
    }
}

/// パッケージング環境から導出される 1 か所集約の実行時ポリシー。
/// `PackageContext::current()` を唯一の参照点として autostart / updater /
/// rollback / activation を決定する。`start_minimized` の実動作と MSIX
/// activation contract の差は `ActivationPolicy` のみで吸収する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageContext {
    pub is_msix: bool,
    pub package_family_name: Option<&'static str>,
    pub autostart: AutostartPolicy,
    pub updater: UpdaterPolicy,
    pub rollback: RollbackPolicy,
    pub activation: ActivationPolicy,
}

impl PackageContext {
    /// 現在のプロセスに適用される PackageContext を返す。
    /// MSIX 検出は `is_msix_packaged_via_api` (Windows) + path fallback。
    /// 非 Windows では常に unpackaged。
    pub fn current() -> Self {
        let is_msix = is_msix_packaged_with_fallback();
        if is_msix {
            PackageContext {
                is_msix: true,
                package_family_name: None,
                autostart: AutostartPolicy::Disabled,
                updater: UpdaterPolicy::Disabled,
                rollback: RollbackPolicy::ReleasePage,
                activation: ActivationPolicy::Show,
            }
        } else {
            PackageContext {
                is_msix: false,
                package_family_name: None,
                autostart: AutostartPolicy::HkeyRun,
                updater: UpdaterPolicy::Active,
                rollback: RollbackPolicy::InstallerDownload,
                activation: ActivationPolicy::Show,
            }
        }
    }

    /// "store" / "unpackaged" のいずれかを返す。
    pub fn policy_label(&self) -> &'static str {
        if self.is_msix {
            "store"
        } else {
            "unpackaged"
        }
    }
}

/// MSIX パッケージ環境かを判定する。`current_exe()` の path に `\WindowsApps\`
/// が含まれるかで簡易判定する (Microsoft が `GetCurrentPackageFullName` の
/// 前段スクリーニングとして例示する手法)。失敗時は `false` にフォールバック。
pub fn is_msix_packaged() -> bool {
    is_msix_packaged_for_path(std::env::current_exe().ok().as_deref())
}

/// テスト容易性のため、判定対象パスを引数で受け取る純粋関数。
fn is_msix_packaged_for_path(exe: Option<&std::path::Path>) -> bool {
    let Some(path) = exe else { return false };
    let s = path.to_string_lossy().to_ascii_lowercase();
    s.contains(r"\windowsapps\")
}

/// `GetCurrentPackageFullName` 経由で package identity を取得する。unpackaged
/// プロセスでは `APPMODEL_ERROR_NO_PACKAGE` を返して `false` になる。
/// 失敗時は `false` (= unpackaged 扱い)。
#[cfg(windows)]
pub fn is_msix_packaged_via_api() -> bool {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::WIN32_ERROR;
    use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;

    let mut len: u32 = 0;
    // 1st call: 必要バッファ長を取得 (普通は ERROR_INSUFFICIENT_BUFFER = 122)
    let rc = unsafe { GetCurrentPackageFullName(&mut len, None) };
    if rc != WIN32_ERROR(0) {
        // APPMODEL_ERROR_NO_PACKAGE (= 15700) またはその他の失敗
        return false;
    }
    if len == 0 {
        return false;
    }
    let mut buf = vec![0u16; len as usize];
    let rc2 = unsafe { GetCurrentPackageFullName(&mut len, Some(PWSTR(buf.as_mut_ptr()))) };
    rc2 == WIN32_ERROR(0)
}

#[cfg(not(windows))]
pub fn is_msix_packaged_via_api() -> bool {
    false
}

/// path fallback を含む 2 段判定。`is_msix_packaged_via_api` が true なら
/// 確実に MSIX。API 失敗時は path heuristic にフォールバック。
fn is_msix_packaged_with_fallback() -> bool {
    if is_msix_packaged_via_api() {
        return true;
    }
    is_msix_packaged()
}

#[cfg(test)]
mod tests {
    /// `APP_USER_MODEL_ID` は `tauri.conf.json` の `identifier` と整合させる
    /// 契約がある。`Vendor.Product.Subproduct.VersionInformation` 形式の
    /// ベンダー ID + アプリ ID + Subproduct の少なくとも 3 ドットを含む
    /// canonical 形であることを保証する。
    ///
    /// NOTE: `super::APP_USER_MODEL_ID` は `#[cfg(windows)] pub const` なので、
    /// `cargo check --target x86_64-unknown-linux-gnu` 等の非 Windows ターゲット
    /// でもこのテストモジュールをコンパイルできるよう、本テストにも
    /// `#[cfg(windows)]` を付与する。
    #[cfg(windows)]
    #[test]
    fn app_user_model_id_has_canonical_dot_separated_form() {
        let parts: Vec<&str> = super::APP_USER_MODEL_ID.split('.').collect();
        assert!(
            parts.len() >= 3,
            "AUMID should have at least 3 dot-separated segments: got {:?}",
            super::APP_USER_MODEL_ID
        );
        for (i, seg) in parts.iter().enumerate() {
            assert!(
                !seg.is_empty(),
                "AUMID segment {i} is empty: {:?}",
                super::APP_USER_MODEL_ID
            );
            assert!(
                seg.chars().all(|c| c.is_ascii_alphanumeric()),
                "AUMID segment {i:?} must be ASCII alphanumeric only: {:?}",
                super::APP_USER_MODEL_ID
            );
        }
    }

    /// AUMID 文字列は `dev.easycursorswap.app` 形式 (= `tauri.conf.json` の
    /// identifier と完全一致) を維持する。これを意図せず変更すると Windows
    /// のトースト通知 / ジャンプリスト / タスクバーグルーピングが Tauri 既定
    /// (タスクバーで別アプリ扱い) にフォールバックする。
    ///
    /// NOTE: Windows-only シンボルを参照するため `#[cfg(windows)]` で gate。
    #[cfg(windows)]
    #[test]
    fn app_user_model_id_matches_tauri_identifier() {
        assert_eq!(super::APP_USER_MODEL_ID, "dev.easycursorswap.app");
    }

    /// 非 Windows での `register_aumid` はノーオペ。
    /// panic / error を返さず呼び出しが即座に返ることのみを保証する。
    #[cfg(not(windows))]
    #[test]
    fn register_aumid_is_noop_on_non_windows() {
        // 失敗しないことだけ確認。
        super::register_aumid();
    }

    // --- Wave 4B: PackageContext 集約 ---

    /// `PackageContext::current()` は常に何らかの `PackageContext` を返す。
    /// (unpackaged 環境では `is_msix = false`、autostart/updater/rollback は通常経路)
    #[test]
    fn package_context_current_returns_something() {
        let ctx = super::PackageContext::current();
        // 通常版では autostart は HkeyRun が既定値
        assert!(matches!(
            ctx.autostart,
            super::AutostartPolicy::HkeyRun | super::AutostartPolicy::Disabled
        ));
    }

    /// `policy_label()` は "store" または "unpackaged" のいずれかを返す。
    #[test]
    fn package_context_policy_label_is_known() {
        let ctx = super::PackageContext::current();
        let label = ctx.policy_label();
        assert!(
            label == "store" || label == "unpackaged",
            "unexpected policy label: {label}"
        );
    }

    /// `AutostartPolicy` の表示は "disabled" / "hkey-run" / "startup-task" のいずれか。
    #[test]
    fn autostart_policy_label_is_known() {
        let cases = [
            (super::AutostartPolicy::Disabled, "disabled"),
            (super::AutostartPolicy::HkeyRun, "hkey-run"),
            (super::AutostartPolicy::StartupTask, "startup-task"),
        ];
        for (policy, expected) in cases {
            assert_eq!(policy.label(), expected);
        }
    }

    /// `UpdaterPolicy` の表示は "disabled" / "active" のいずれか。
    #[test]
    fn updater_policy_label_is_known() {
        let cases = [
            (super::UpdaterPolicy::Disabled, "disabled"),
            (super::UpdaterPolicy::Active, "active"),
        ];
        for (policy, expected) in cases {
            assert_eq!(policy.label(), expected);
        }
    }

    /// `RollbackPolicy` の表示は "release-page" / "installer-download" のいずれか。
    #[test]
    fn rollback_policy_label_is_known() {
        let cases = [
            (super::RollbackPolicy::ReleasePage, "release-page"),
            (
                super::RollbackPolicy::InstallerDownload,
                "installer-download",
            ),
        ];
        for (policy, expected) in cases {
            assert_eq!(policy.label(), expected);
        }
    }

    /// `ActivationPolicy` の表示は "show" / "hide" のいずれか。
    #[test]
    fn activation_policy_label_is_known() {
        let cases = [
            (super::ActivationPolicy::Show, "show"),
            (super::ActivationPolicy::Hide, "hide"),
        ];
        for (policy, expected) in cases {
            assert_eq!(policy.label(), expected);
        }
    }

    /// `is_msix_packaged_via_api` は通常版テストでは呼び出し失敗 → `false`。
    /// (Windows cfg では本物の API を呼び出すが、本テストは API 失敗を許容する)
    #[cfg(windows)]
    #[test]
    fn is_msix_packaged_via_api_returns_false_when_unpackaged() {
        // 通常版の cargo test では MSIX パッケージではないため、`GetCurrentPackageFullName`
        // は APPMODEL_ERROR_NO_PACKAGE を返して `false` になる。
        assert!(!super::is_msix_packaged_via_api());
    }

    /// unpackaged 環境では `current()` は `is_msix=false` を返し、各ポリシーが通常経路になる。
    #[cfg(windows)]
    #[test]
    fn current_policy_for_unpackaged_uses_legacy_paths() {
        let ctx = super::PackageContext::current();
        assert!(!ctx.is_msix, "expected unpackaged in test env");
        assert!(matches!(ctx.autostart, super::AutostartPolicy::HkeyRun));
        assert!(matches!(ctx.updater, super::UpdaterPolicy::Active));
        assert!(matches!(
            ctx.rollback,
            super::RollbackPolicy::InstallerDownload
        ));
        assert!(matches!(ctx.activation, super::ActivationPolicy::Show));
    }

    /// path heuristic は大文字 / 小文字 / 混在のいずれも MSIX と判定する。
    #[test]
    fn detects_msix_path_case_insensitive() {
        use std::path::PathBuf;
        let cases = [
            r"C:\Program Files\WindowsApps\dev.easycursorswap.app_1.0.0_x64__abc\app.exe",
            r"C:\Program Files\windowsapps\dev.easycursorswap.app_1.0.0_x64__abc\app.exe",
            r"C:\PROGRAM FILES\WINDOWSAPPS\dev.easycursorswap.app_1.0.0_x64__abc\app.exe",
        ];
        for c in cases {
            let p = PathBuf::from(c);
            assert!(
                super::is_msix_packaged_for_path(Some(&p)),
                "MSIX として判定されるべき: {c}"
            );
        }
    }

    /// 通常インストールのパスは MSIX 扱いされない。
    #[test]
    fn does_not_detect_normal_install_paths_as_msix() {
        use std::path::PathBuf;
        let cases = [
            r"C:\Program Files\EasyCursorSwap\easy-cursor-swap.exe",
            r"C:\Users\me\AppData\Local\Programs\EasyCursorSwap\app.exe",
            r"D:\dev\target\release\easy-cursor-swap.exe",
        ];
        for c in cases {
            let p = PathBuf::from(c);
            assert!(
                !super::is_msix_packaged_for_path(Some(&p)),
                "通常インストールは MSIX 扱いされるべきでない: {c}"
            );
        }
    }

    /// exe path が取得できない場合は `false`。
    #[test]
    fn returns_false_when_exe_path_is_unavailable() {
        assert!(!super::is_msix_packaged_for_path(None));
    }
}
