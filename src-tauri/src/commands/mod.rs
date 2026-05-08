//! EasyCursorSwap Tauri IPC コマンド定義
//!
//! フロントエンド (Nuxt) から呼び出し可能なコマンドを定義する。
//! 各コマンドは Tauri の `#[tauri::command]` マクロで公開される。
//!
//! 責務別にサブモジュール分割している:
//! - [`keystore`] — Ed25519 鍵ペア管理 (生成 / 削除 / Export / Import)
//!
//! 残りのコマンドは段階的に切り出し中。新規追加時は適切なサブモジュールへ。

pub mod cursor_io;
pub mod keystore;
pub mod marketplace;
pub mod profile;
pub mod theme;
pub mod windows_scheme;

use crate::accessibility::AccessibilityConflicts;
use crate::autostart;
use crate::config::{AppConfig, BackupInfo, ConfigManager};
use crate::cursor::{build_cur_from_png, ResizeMethod};
use crate::darkmode;
use crate::environment::EnvironmentReport;
use crate::errors::AppError;
use crate::health::is_major_bump;
use crate::registry::RegistryManager;
use crate::theme::{CursorDefinition, LocalizedString, ThemeManager, ThemeMetadata};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tauri::State;

/// クリエイターから渡された PNG バイト列を 6 サイズ .cur に変換し、
/// 指定パスへ書き出す。`resample` は "lanczos" / "nearest" / "auto"。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildCurRequest {
    /// PNG ファイルのバイト列 (Tauri は Vec<u8> を Number 配列として渡せる)
    pub png_bytes: Vec<u8>,
    /// 元画像でのホットスポット座標
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    /// リサンプル: "lanczos" / "nearest" / "auto"
    pub resample: String,
    /// 書き出し先ファイルパス
    pub output_path: String,
}

#[tauri::command]
pub fn build_cursor_file(req: BuildCurRequest) -> Result<u64, AppError> {
    let resample = match req.resample.as_str() {
        "auto" => ResizeMethod::Lanczos, // 自動判定は build_cur_from_png 内で行う
        other => ResizeMethod::from_str(other),
    };
    let bin = build_cur_from_png(&req.png_bytes, req.hotspot_x, req.hotspot_y, resample, None)?;

    let path = std::path::PathBuf::from(&req.output_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &bin)?;
    tracing::info!(
        "build_cursor_file: wrote {} bytes to {}",
        bin.len(),
        crate::logging::redact_path(&path)
    );
    Ok(bin.len() as u64)
}

/// `.cursorpack` をエクスポートする際のリクエスト。
/// `cursors` は役割名 → ファイルパス (Rust 側でファイル読込) で渡す。
/// パスは絶対パスを期待 (UI の保存ダイアログから渡される想定)。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCursorpackRequest {
    pub name_ja: String,
    pub name_en: Option<String>,
    pub author: Option<String>,
    pub version: String,
    pub requires_os_shadow: bool,
    /// 役割名 → 元画像ホットスポット (`{ "Arrow": { x: 4, y: 4 } }`)
    pub hotspots: std::collections::HashMap<String, Hotspot>,
    /// 役割名 → ローカル `.cur` ファイルパス
    pub cur_paths: std::collections::HashMap<String, String>,
    pub output_path: String,
    /// true の場合、現在の鍵ペアでパッケージ全体に署名する。
    /// theme.json に `signature` フィールドを埋め込む。
    pub sign: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Hotspot {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub theme_id: String,
    pub size_bytes: u64,
    pub signed: bool,
    pub key_id: Option<String>,
}

#[tauri::command]
pub fn export_cursorpack(req: ExportCursorpackRequest) -> Result<ExportResult, AppError> {
    use std::collections::HashMap;

    // 1) cursors マップ構築
    let mut cursors_meta: HashMap<String, CursorDefinition> = HashMap::new();
    let mut cursor_bytes: HashMap<String, Vec<u8>> = HashMap::new();
    for (role, path) in &req.cur_paths {
        let path = std::path::PathBuf::from(path);
        let bin = std::fs::read(&path).map_err(|e| {
            AppError::Theme(format!(
                "カーソル {} が読み込めません ({}): {}",
                role,
                path.display(),
                e
            ))
        })?;
        let hot = req
            .hotspots
            .get(role)
            .cloned()
            .unwrap_or(Hotspot { x: 0, y: 0 });
        cursors_meta.insert(
            role.clone(),
            CursorDefinition {
                file: format!("cursors/{}.cur", role),
                hotspot_x: hot.x,
                hotspot_y: hot.y,
                resize_method: "lanczos".to_string(),
                size_overrides: None,
            },
        );
        cursor_bytes.insert(role.clone(), bin);
    }

    // 2) theme.json メタデータ
    let mut name_map = HashMap::new();
    name_map.insert("ja".to_string(), req.name_ja.clone());
    if let Some(en) = req.name_en.clone() {
        name_map.insert("en".to_string(), en);
    }

    let mut metadata = ThemeMetadata {
        schema_version: 1,
        id: uuid::Uuid::new_v4(),
        name: LocalizedString::Localized(name_map),
        version: req.version.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        requires_os_shadow: req.requires_os_shadow,
        cursors: cursors_meta,
        author: req.author.clone(),
        license: None,
        homepage: None,
        description: None,
        min_app_version: None,
        signature: None,
        tags: Vec::new(),
    };

    // 3) 署名 (rの場合)
    let mut signed_key_id: Option<String> = None;
    if req.sign {
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
        signed_key_id = info.key_id.clone();
    }

    // 4) Zip 出力
    let out_path = std::path::PathBuf::from(&req.output_path);
    let size = ThemeManager::export_cursorpack(&mut metadata, &cursor_bytes, &out_path)?;

    Ok(ExportResult {
        theme_id: metadata.id.to_string(),
        size_bytes: size,
        signed: req.sign,
        key_id: signed_key_id,
    })
}

// ===========================================================================
// ストリーム式 .cursorpack ビルド (Phase 3-1 残)
// ---------------------------------------------------------------------------
// 17 役割 × 6 サイズ = 最大 102 枚の .cur 生成は重い処理。
// 以下を 1 回の IPC で実行しつつ、進捗を Tauri イベントで配信する:
//   1. 各役割の PNG → 6 サイズ .cur をビルド
//   2. theme.json メタデータ構築
//   3. 必要なら Ed25519 署名
//   4. Zip エクスポート
// 配信イベント: `build-progress` (build_id 付き、フロントが filter する)
// キャンセル: `cancel_build(build_id)` IPC で AtomicBool 相当のセットに登録
//   各 role 処理前 / 主要ステップ前にチェックして早期終了。
// ===========================================================================

use std::sync::OnceLock;

/// キャンセル要求済みの build_id 集合。`OnceLock` で初期化、`Mutex` で同期。
fn cancel_set() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    static SET: OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    SET.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
}

fn mark_cancelled(build_id: &str) {
    if let Ok(mut s) = cancel_set().lock() {
        s.insert(build_id.to_string());
    }
}

fn is_cancelled(build_id: &str) -> bool {
    cancel_set()
        .lock()
        .map(|s| s.contains(build_id))
        .unwrap_or(false)
}

fn clear_cancel(build_id: &str) {
    if let Ok(mut s) = cancel_set().lock() {
        s.remove(build_id);
    }
}

/// 1 役割分の入力 (PNG バイト列 + ホットスポット + リサンプル指定)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleBuildEntry {
    pub role: String,
    pub png_bytes: Vec<u8>,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    /// "lanczos" / "nearest" / "auto"
    pub resample: String,
    /// サイズ別オーバーライド (px → PNG bytes)。
    /// Some の場合、対応サイズはリサンプルせずそのまま使用。
    /// None / 空なら従来どおり png_bytes をリサンプル。
    #[serde(default)]
    pub sized_png_bytes: Option<std::collections::HashMap<u32, Vec<u8>>>,
}

/// ストリーム式 .cursorpack ビルドリクエスト
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamedExportRequest {
    /// フロント側が生成した一意 ID。`build-progress` イベントの相関キー兼キャンセル ID。
    pub build_id: String,
    pub name_ja: String,
    pub name_en: Option<String>,
    pub author: Option<String>,
    pub version: String,
    pub requires_os_shadow: bool,
    pub roles: Vec<RoleBuildEntry>,
    pub output_path: String,
    pub sign: bool,
}

/// 進捗イベントペイロード
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuildProgress {
    pub build_id: String,
    /// "role" / "package" / "sign" / "done" / "cancelled" / "error"
    pub stage: String,
    pub current: u32,
    pub total: u32,
    pub message: Option<String>,
}

/// 進行中の build を中止する。実際の中止は次のチェックポイントで行われる。
#[tauri::command]
pub fn cancel_build(build_id: String) {
    mark_cancelled(&build_id);
    tracing::info!("ビルド中止要求: {}", build_id);
}

fn emit_progress(app: &tauri::AppHandle, payload: BuildProgress) {
    use tauri::Emitter;
    if let Err(e) = app.emit("build-progress", payload) {
        tracing::warn!("build-progress emit 失敗: {}", e);
    }
}

/// ストリーム式 .cursorpack ビルド & エクスポート。
///
/// 単一 IPC 呼び出しで全工程を実行し、各ステップで `build-progress` イベントを発火する。
/// `cancel_build(build_id)` が呼ばれていれば次のチェックポイントで早期終了する。
#[tauri::command]
pub fn export_cursorpack_streamed(
    app: tauri::AppHandle,
    req: StreamedExportRequest,
) -> Result<ExportResult, AppError> {
    use std::collections::HashMap;

    let total_roles = req.roles.len() as u32;
    let total_steps = total_roles + if req.sign { 2 } else { 1 }; // roles + package (+sign)

    // 開始イベント
    emit_progress(
        &app,
        BuildProgress {
            build_id: req.build_id.clone(),
            stage: "role".to_string(),
            current: 0,
            total: total_steps,
            message: Some("preparing".to_string()),
        },
    );

    // 1) 各役割の .cur をメモリ上でビルド
    let mut cursor_bytes: HashMap<String, Vec<u8>> = HashMap::new();
    let mut cursors_meta: HashMap<String, CursorDefinition> = HashMap::new();
    for (idx, entry) in req.roles.iter().enumerate() {
        if is_cancelled(&req.build_id) {
            clear_cancel(&req.build_id);
            emit_progress(
                &app,
                BuildProgress {
                    build_id: req.build_id.clone(),
                    stage: "cancelled".to_string(),
                    current: idx as u32,
                    total: total_steps,
                    message: Some(entry.role.clone()),
                },
            );
            return Err(AppError::Other("ビルドがキャンセルされました".to_string()));
        }

        let resample = match entry.resample.as_str() {
            "auto" => ResizeMethod::Lanczos,
            other => ResizeMethod::from_str(other),
        };
        let bin = build_cur_from_png(
            &entry.png_bytes,
            entry.hotspot_x,
            entry.hotspot_y,
            resample,
            entry.sized_png_bytes.as_ref(),
        )?;
        cursor_bytes.insert(entry.role.clone(), bin);
        cursors_meta.insert(
            entry.role.clone(),
            CursorDefinition {
                file: format!("cursors/{}.cur", entry.role),
                hotspot_x: entry.hotspot_x,
                hotspot_y: entry.hotspot_y,
                resize_method: entry.resample.clone(),
                size_overrides: None,
            },
        );

        emit_progress(
            &app,
            BuildProgress {
                build_id: req.build_id.clone(),
                stage: "role".to_string(),
                current: (idx + 1) as u32,
                total: total_steps,
                message: Some(entry.role.clone()),
            },
        );
    }

    // 2) theme.json メタデータ
    let mut name_map = HashMap::new();
    name_map.insert("ja".to_string(), req.name_ja.clone());
    if let Some(en) = req.name_en.clone() {
        name_map.insert("en".to_string(), en);
    }
    let mut metadata = ThemeMetadata {
        schema_version: 1,
        id: uuid::Uuid::new_v4(),
        name: LocalizedString::Localized(name_map),
        version: req.version.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        requires_os_shadow: req.requires_os_shadow,
        cursors: cursors_meta,
        author: req.author.clone(),
        license: None,
        homepage: None,
        description: None,
        min_app_version: None,
        signature: None,
        tags: Vec::new(),
    };

    // 3) 署名
    let mut signed_key_id: Option<String> = None;
    if req.sign {
        if is_cancelled(&req.build_id) {
            clear_cancel(&req.build_id);
            return Err(AppError::Other("ビルドがキャンセルされました".to_string()));
        }
        emit_progress(
            &app,
            BuildProgress {
                build_id: req.build_id.clone(),
                stage: "sign".to_string(),
                current: total_roles,
                total: total_steps,
                message: None,
            },
        );
        let info = crate::keystore::Keystore::info()?;
        if !info.has_keypair {
            return Err(AppError::Theme(
                "鍵ペアがありません。設定 → 署名鍵 で生成してください".to_string(),
            ));
        }
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
        signed_key_id = info.key_id.clone();
    }

    // 4) Zip 出力
    if is_cancelled(&req.build_id) {
        clear_cancel(&req.build_id);
        return Err(AppError::Other("ビルドがキャンセルされました".to_string()));
    }
    emit_progress(
        &app,
        BuildProgress {
            build_id: req.build_id.clone(),
            stage: "package".to_string(),
            current: total_steps - 1,
            total: total_steps,
            message: None,
        },
    );

    let out_path = std::path::PathBuf::from(&req.output_path);
    let size = ThemeManager::export_cursorpack(&mut metadata, &cursor_bytes, &out_path)?;

    emit_progress(
        &app,
        BuildProgress {
            build_id: req.build_id.clone(),
            stage: "done".to_string(),
            current: total_steps,
            total: total_steps,
            message: Some(metadata.id.to_string()),
        },
    );
    clear_cancel(&req.build_id);

    Ok(ExportResult {
        theme_id: metadata.id.to_string(),
        size_bytes: size,
        signed: req.sign,
        key_id: signed_key_id,
    })
}

/// Windows 既定カーソルにリセットする（パニックボタン）
#[tauri::command]
pub fn reset_to_default() -> Result<(), AppError> {
    RegistryManager::reset_to_windows_default()
}

/// インストール前の状態にリセットする
#[tauri::command]
pub fn reset_to_initial() -> Result<(), AppError> {
    RegistryManager::restore_from_initial_snapshot()
}

/// 現在のダークモード状態を取得する
#[tauri::command]
pub fn get_dark_mode_status() -> Result<bool, AppError> {
    darkmode::is_dark_mode()
}

/// 動作環境レポートを返す (RDP / Server SKU 検出)。
/// UI 起動時に呼んで警告ダイアログ表示判定に使う。
#[tauri::command]
pub fn get_environment_report() -> EnvironmentReport {
    EnvironmentReport::detect()
}

/// アプリケーション設定を取得する
#[tauri::command]
pub fn get_config(config: State<'_, ConfigManager>) -> Result<AppConfig, AppError> {
    config.get()
}

/// アプリケーション設定を更新する。
///
/// 副作用として `general.auto_start` をレジストリ (HKCU\...\Run) に同期する。
/// 同期に失敗してもログを出すのみで設定保存自体はエラーとしない (UI 操作の妨げを防ぐため)。
#[tauri::command]
pub fn update_config(
    config: State<'_, ConfigManager>,
    updates: AppConfig,
) -> Result<AppConfig, AppError> {
    let auto_start = updates.general.auto_start;
    let saved = config.update(|c| {
        *c = updates;
    })?;
    if let Err(e) = autostart::set_enabled(auto_start) {
        tracing::warn!("自動起動レジストリ同期失敗: {}", e);
    }
    Ok(saved)
}

/// 現在の自動起動 (HKCU Run) 登録状態を返す。
///
/// 設定 `general.auto_start` とレジストリ実態が乖離していないかの確認用。
#[tauri::command]
pub fn get_autostart_status() -> bool {
    autostart::is_enabled()
}

/// アプリ情報を返す（バージョン等）
#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub cursors_dir: String,
    pub config_dir: String,
    pub os_version: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    let cursors_dir = ConfigManager::cursors_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let config_dir = dirs::data_local_dir()
        .map(|p| p.join("EasyCursorSwap").to_string_lossy().to_string())
        .unwrap_or_default();

    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        cursors_dir,
        config_dir,
        os_version: get_os_version(),
    }
}

/// OS バージョンを取得する
fn get_os_version() -> String {
    #[cfg(windows)]
    {
        let info = windows::Win32::System::SystemInformation::OSVERSIONINFOW::default();
        format!("Windows {}.{}", info.dwMajorVersion, info.dwMinorVersion)
    }
    #[cfg(not(windows))]
    {
        "Non-Windows".to_string()
    }
}

/// 設定バックアップファイルの一覧を返す。
///
/// `config.bak.v*.json` (スキーマ移行バックアップ) と
/// `config.corrupt.*.json` (破損退避ファイル) を列挙し、最終更新日時の降順で返す。
#[tauri::command]
pub fn list_config_backups(config: State<'_, ConfigManager>) -> Result<Vec<BackupInfo>, AppError> {
    config.list_backups()
}

/// 指定したバックアップファイルを `config.json` に上書きして設定を復旧する。
///
/// 復旧後は UI 側でアプリを再起動するかページをリロードすること。
#[tauri::command]
pub fn restore_config_backup(
    file_name: String,
    config: State<'_, ConfigManager>,
) -> Result<(), AppError> {
    config.restore_backup(&file_name)
}

/// 指定 URL をシステムのデフォルトブラウザで開く。
///
/// URL は `https://` または `http://` で始まる必要がある。
/// それ以外は `AppError::InvalidInput` を返す。
///
/// Windows 専用実装: Win32 ShellExecuteW を直接呼ぶ。
#[tauri::command]
pub fn open_url(url: String) -> Result<(), AppError> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(AppError::InvalidInput(format!(
            "不正な URL スキーム: {}",
            url
        )));
    }
    #[cfg(windows)]
    {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let url_h = HSTRING::from(url.as_str());
        let result = unsafe {
            ShellExecuteW(
                None,
                PCWSTR(HSTRING::from("open").as_ptr()),
                PCWSTR(url_h.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        // ShellExecuteW は HINSTANCE を返す; ポインタ値が 32 より大きければ成功
        if (result.0 as usize) <= 32 {
            return Err(AppError::Other("ShellExecuteW が失敗しました".to_string()));
        }
    }
    #[cfg(not(windows))]
    {
        return Err(AppError::Other("open_url は Windows 専用です".to_string()));
    }
    Ok(())
}

/// アクセシビリティ機能との競合を検出する。
///
/// レジストリから MouseSonar / HighContrast / CursorBaseSize を読み取り、
/// テーマ適用時にユーザーへ警告すべき状態かを返す。
#[tauri::command]
pub fn get_accessibility_conflicts() -> AccessibilityConflicts {
    AccessibilityConflicts::detect()
}

/// 現行バージョンから新バージョンへの更新がメジャー跨ぎかどうかを返す。
///
/// フロントエンドはアップデート確認時にこれを呼び出し、`true` の場合は
/// 追加の確認ダイアログをユーザーに表示すること。
#[tauri::command]
pub fn check_update_is_major_jump(current_version: String, new_version: String) -> bool {
    is_major_bump(&current_version, &new_version)
}

/// 保存済みクラッシュレポート (panic) の一覧を返す。
///
/// 新しい順、上限 50 件 (`crash::list_reports` の内部上限)。
/// レポート本体には PII 除外済みのメッセージのみ含まれる。
#[tauri::command]
pub fn list_crash_reports() -> Result<Vec<crate::crash::CrashReport>, AppError> {
    crate::crash::list_reports()
}

/// 保存済みクラッシュレポートを全削除する。戻り値は削除した件数。
///
/// ユーザーが「設定 → ログ → クラッシュ履歴を消去」を実行したときに呼ぶ想定。
#[tauri::command]
pub fn clear_crash_reports() -> Result<usize, AppError> {
    crate::crash::clear_reports()
}

/// 指定 Windows スキームのロール毎 PNG プレビューを返す。
///
/// Tauri Builder に全コマンドを登録するためのヘルパー
pub fn get_command_handlers() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![
        theme::get_cursor_roles,
        theme::get_current_cursors,
        theme::get_themes,
        theme::get_theme_previews,
        theme::apply_theme,
        theme::inspect_cursorpack,
        theme::import_cursorpack,
        build_cursor_file,
        theme::clear_cursor_cache,
        export_cursorpack,
        profile::export_profile,
        profile::import_profile,
        marketplace::marketplace_fetch_index,
        marketplace::marketplace_install,
        reset_to_default,
        reset_to_initial,
        get_dark_mode_status,
        get_environment_report,
        get_config,
        update_config,
        get_app_info,
        list_config_backups,
        restore_config_backup,
        check_update_is_major_jump,
        open_url,
        get_accessibility_conflicts,
        get_autostart_status,
        cursor_io::import_cursor_file,
        cursor_io::inspect_ani_file,
        export_cursorpack_streamed,
        cancel_build,
        list_crash_reports,
        clear_crash_reports,
        windows_scheme::list_windows_schemes,
        windows_scheme::apply_windows_scheme,
        windows_scheme::get_windows_scheme_previews,
        windows_scheme::export_windows_scheme_as_cursorpack,
        theme::delete_theme,
        theme::duplicate_theme,
        theme::repackage_theme,
        keystore::keystore_info,
        keystore::keystore_generate,
        keystore::keystore_delete,
        keystore::keystore_export,
        keystore::keystore_import,
        crate::bulk_import::bulk_resolve_assets,
        crate::bulk_import::cancel_bulk_import,
        crate::bulk_import::parse_cursorpack_for_creator,
    ]
}

#[cfg(test)]
mod tests {
    use super::{clear_cancel, is_cancelled, mark_cancelled};

    #[test]
    fn cancel_flag_lifecycle() {
        let id = "test-build-cancel-lifecycle-xyz";
        // ユニーク ID なので前提状態は false
        assert!(!is_cancelled(id));
        mark_cancelled(id);
        assert!(is_cancelled(id));
        clear_cancel(id);
        assert!(!is_cancelled(id));
    }

    #[test]
    fn cancel_flags_are_independent_per_build_id() {
        let id_a = "test-build-independent-a-xyz";
        let id_b = "test-build-independent-b-xyz";
        mark_cancelled(id_a);
        assert!(is_cancelled(id_a));
        assert!(!is_cancelled(id_b));
        clear_cancel(id_a);
    }
}
