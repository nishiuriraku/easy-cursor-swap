//! CursorForge Tauri IPC コマンド定義
//!
//! フロントエンド (Nuxt) から呼び出し可能なコマンドを定義する。
//! 各コマンドは Tauri の `#[tauri::command]` マクロで公開される。

use crate::accessibility::AccessibilityConflicts;
use crate::backup::{BackupManager, ProfileEnvelope};
use crate::config::{AppConfig, BackupInfo, ConfigManager};
use crate::health::is_major_bump;
use crate::cursor::{build_cur_from_png, clear_resize_cache, ResizeMethod};
use sha2::Digest;
use crate::darkmode;
use crate::environment::EnvironmentReport;
use crate::errors::AppError;
use crate::keystore::{Keystore, KeystoreInfo};
use crate::marketplace::{MarketplaceClient, MarketplaceIndex, MarketplaceInstallRequest};
use crate::registry::{CursorRole, RegistryManager};
use crate::theme::{CursorDefinition, CursorpackInspection, LocalizedString, ThemeManager, ThemeMetadata, ThemeSummary};
use serde::{Deserialize, Serialize};
use tauri::State;

/// フロントエンドに返すカーソル役割情報
#[derive(Debug, Serialize)]
pub struct CursorRoleInfo {
    /// レジストリ値名
    pub id: String,
    /// 日本語表示名
    pub name_ja: String,
    /// 英語表示名
    pub name_en: String,
    /// Schemes 内でのインデックス
    pub index: usize,
}

/// 全17種のカーソル役割情報を返す
#[tauri::command]
pub fn get_cursor_roles() -> Vec<CursorRoleInfo> {
    CursorRole::all()
        .iter()
        .map(|role| CursorRoleInfo {
            id: role.registry_name().to_string(),
            name_ja: role.display_name_ja().to_string(),
            name_en: role.display_name_en().to_string(),
            index: role.scheme_index(),
        })
        .collect()
}

/// 現在のカーソル設定をレジストリから読み取る
#[tauri::command]
pub fn get_current_cursors() -> Result<std::collections::HashMap<String, String>, AppError> {
    RegistryManager::read_current_cursors()
}

/// テーマ一覧を取得する。`is_active` は config の `active_theme_id` に基づく。
#[tauri::command]
pub fn get_themes(config: State<'_, ConfigManager>) -> Result<Vec<ThemeSummary>, AppError> {
    let active_id = config.get()?.general.active_theme_id;
    ThemeManager::list_themes(active_id)
}

/// 指定 ID のテーマをシステムに適用する。
/// 失敗時は内部のスナップショットから自動ロールバックされる。
/// 成功時は config の `active_theme_id` を更新して永続化する。
#[tauri::command]
pub fn apply_theme(
    config: State<'_, ConfigManager>,
    theme_id: String,
) -> Result<(), AppError> {
    let id = uuid::Uuid::parse_str(&theme_id)
        .map_err(|e| AppError::Theme(format!("無効なテーマ ID: {}", e)))?;
    ThemeManager::apply_theme(id)?;
    // 適用成功 → アクティブテーマ ID を永続化
    config.update(|c| {
        c.general.active_theme_id = Some(id);
    })?;
    Ok(())
}

/// リサイズ結果キャッシュをクリアする。
/// クリエイターで素材を差し替えた直後など、明示的にメモリを開放したいときに使用。
#[tauri::command]
pub fn clear_cursor_cache() {
    clear_resize_cache();
    tracing::info!("リサイズキャッシュをクリアしました");
}

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
pub fn keystore_import(
    passphrase: String,
    input_path: String,
) -> Result<KeystoreInfo, AppError> {
    let path = std::path::PathBuf::from(&input_path);
    if !path.exists() {
        return Err(AppError::Theme(format!("ファイルが見つかりません: {}", input_path)));
    }
    let blob = std::fs::read(&path)?;
    Keystore::import_private_key(&blob, &passphrase)
}

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
    let bin = build_cur_from_png(&req.png_bytes, req.hotspot_x, req.hotspot_y, resample)?;

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
            AppError::Theme(format!("カーソル {} が読み込めません ({}): {}", role, path.display(), e))
        })?;
        let hot = req.hotspots.get(role).cloned().unwrap_or(Hotspot { x: 0, y: 0 });
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
        let role_concat = roles.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",");
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

/// `.cursorpack` をインポートする前のメタデータ検査。
/// 既存ライブラリに同 ID のテーマがあればバージョン比較情報を返す。
#[tauri::command]
pub fn inspect_cursorpack(path: String) -> Result<CursorpackInspection, AppError> {
    let buf = std::path::PathBuf::from(&path);
    if !buf.exists() {
        return Err(AppError::Theme(format!("ファイルが見つかりません: {}", path)));
    }
    ThemeManager::inspect_cursorpack_file(&buf)
}

/// ローカルの `.cursorpack` ファイルをライブラリにインポートする。
/// パストラバーサル / Zip 爆弾 / シンボリックリンク防御つきで展開し、
/// 戻り値として展開後のテーマ ID (UUID 文字列) を返す。
#[tauri::command]
pub fn import_cursorpack(path: String) -> Result<String, AppError> {
    let buf = std::path::PathBuf::from(&path);
    if !buf.exists() {
        return Err(AppError::Theme(format!("ファイルが見つかりません: {}", path)));
    }
    // 拡張子を弱バリデーション (Magic Byte は ThemeManager 内で再チェック)
    let ext_ok = buf
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("cursorpack"))
        .unwrap_or(false);
    if !ext_ok {
        return Err(AppError::Theme(
            ".cursorpack 以外の拡張子は受け入れません".to_string(),
        ));
    }
    let id = ThemeManager::import_cursorpack_file(&buf)?;
    Ok(id.to_string())
}

/// マーケットプレイス経由のインストール後はテーマ ID を返却する。
/// (`apply_theme` を続けて呼ぶことで即時アクティブ化可能)

/// `.cursorprofile` (設定 + 全テーマ) を指定パスに書き出す。
#[tauri::command]
pub fn export_profile(
    config: State<'_, ConfigManager>,
    path: String,
) -> Result<(), AppError> {
    let cfg = config.get()?;
    let target = std::path::PathBuf::from(&path);
    BackupManager::export(&target, &cfg)
}

/// `.cursorprofile` を読み込んで設定と全テーマを復元する。
/// `merge=true` なら既存テーマを保持し新規分のみ反映、`false` なら完全上書き。
#[tauri::command]
pub fn import_profile(
    config: State<'_, ConfigManager>,
    path: String,
    merge: bool,
) -> Result<ProfileEnvelope, AppError> {
    let buf = std::path::PathBuf::from(&path);
    if !buf.exists() {
        return Err(AppError::Theme(format!("ファイルが見つかりません: {}", path)));
    }
    let envelope = BackupManager::import(&buf, merge)?;
    // 設定もファイル経由で復元
    config.update(|c| {
        *c = envelope.config.clone();
    })?;
    Ok(envelope)
}

/// 公式インデックス (Marketplace) のメタデータを取得する。
/// `cursorforge/index` リポジトリの `index.json` を HTTPS + rustls で取得。
#[tauri::command]
pub async fn marketplace_fetch_index() -> Result<MarketplaceIndex, AppError> {
    MarketplaceClient::fetch_index().await
}

/// 公式インデックスから指定エントリをダウンロード→検証→展開する。
/// (1) ダウンロード, (2) SHA-256 整合性, (3) Ed25519 署名検証, (4) ZIP 展開。
/// 戻り値はインポートしたテーマ ID (UUID 文字列)。
#[tauri::command]
pub async fn marketplace_install(req: MarketplaceInstallRequest) -> Result<String, AppError> {
    let id = MarketplaceClient::install(req).await?;
    Ok(id.to_string())
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

/// アプリケーション設定を更新する
#[tauri::command]
pub fn update_config(
    config: State<'_, ConfigManager>,
    updates: AppConfig,
) -> Result<AppConfig, AppError> {
    config.update(|c| {
        *c = updates;
    })
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
        .map(|p| p.join("CursorForge").to_string_lossy().to_string())
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
        format!(
            "Windows {}.{}",
            info.dwMajorVersion, info.dwMinorVersion
        )
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
pub fn list_config_backups(
    config: State<'_, ConfigManager>,
) -> Result<Vec<BackupInfo>, AppError> {
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
            return Err(AppError::Other(
                "ShellExecuteW が失敗しました".to_string(),
            ));
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

/// Tauri Builder に全コマンドを登録するためのヘルパー
pub fn get_command_handlers() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![
        get_cursor_roles,
        get_current_cursors,
        get_themes,
        apply_theme,
        inspect_cursorpack,
        import_cursorpack,
        build_cursor_file,
        keystore_info,
        keystore_generate,
        keystore_delete,
        keystore_export,
        keystore_import,
        clear_cursor_cache,
        export_cursorpack,
        export_profile,
        import_profile,
        marketplace_fetch_index,
        marketplace_install,
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
    ]
}
