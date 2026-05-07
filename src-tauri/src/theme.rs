//! EasyCursorSwap テーマ管理モジュール
//!
//! .cursorpack パッケージの作成・解凍・バリデーションを行う。
//! テーマメタデータ (theme.json) の管理も担当。

use crate::errors::AppResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// テーマメタデータ (theme.json のスキーマ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetadata {
    /// スキーマバージョン
    pub schema_version: u32,
    /// テーマ固有ID (UUID)
    pub id: Uuid,
    /// テーマ名 (多言語対応)
    pub name: LocalizedString,
    /// テーマバージョン (SemVer)
    pub version: String,
    /// 作成日時 (ISO8601)
    pub created_at: String,
    /// OS標準の影を必要とするか
    pub requires_os_shadow: bool,
    /// カーソル定義マップ（役割 → カーソル定義）
    pub cursors: HashMap<String, CursorDefinition>,

    // --- 推奨フィールド ---
    /// 作者名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// ライセンス (SPDX識別子)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// ホームページURL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// テーマ説明 (多言語対応)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<LocalizedString>,
    /// 最低動作アプリバージョン
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_app_version: Option<String>,
    /// 署名 (将来の検証用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// 多言語対応文字列
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocalizedString {
    /// 単純な文字列
    Simple(String),
    /// 多言語マップ
    Localized(HashMap<String, String>),
}

impl LocalizedString {
    /// 指定ロケールに合った文字列を返す
    /// フォールバック: 指定ロケール → "default" → "en" → 最初の値
    pub fn get(&self, locale: &str) -> String {
        match self {
            LocalizedString::Simple(s) => s.clone(),
            LocalizedString::Localized(map) => {
                // まず指定ロケールをチェック
                if let Some(val) = map.get(locale) {
                    return val.clone();
                }
                // "default" キーをチェック
                if let Some(val) = map.get("default") {
                    return val.clone();
                }
                // "en" フォールバック
                if let Some(val) = map.get("en") {
                    return val.clone();
                }
                // どれもなければ最初の値
                map.values().next().cloned().unwrap_or_default()
            }
        }
    }
}

/// 個別カーソルの定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorDefinition {
    /// カーソルファイルのパス（.cursorpack 内の相対パス）
    pub file: String,
    /// ホットスポット X座標（元画像のピクセル値）
    pub hotspot_x: u32,
    /// ホットスポット Y座標（元画像のピクセル値）
    pub hotspot_y: u32,
    /// リサイズアルゴリズム ("lanczos" / "nearest")
    #[serde(default = "default_resize_method")]
    pub resize_method: String,
    /// 解像度別の画像オーバーライド
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_overrides: Option<HashMap<String, SizeOverride>>,
}

/// 解像度別オーバーライド
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeOverride {
    /// このサイズ専用の画像ファイルパス
    pub file: String,
    /// このサイズ専用のホットスポット X（未指定時は基準サイズから比例計算）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotspot_x: Option<u32>,
    /// このサイズ専用のホットスポット Y
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotspot_y: Option<u32>,
}

fn default_resize_method() -> String {
    "lanczos".to_string()
}

/// `.cursorpack` をインポートする前の軽量検査結果。
/// theme.json のみ読み出してメタ情報を返し、既存ライブラリとの衝突を報告する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorpackInspection {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub role_count: u32,
    /// 既存ライブラリに同 ID のテーマがあれば情報を埋める
    pub existing: Option<ExistingTheme>,
}

/// 既存ライブラリ内テーマの参照情報 (バージョン比較用)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistingTheme {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub role_count: u32,
}

/// テーマライブラリ内の1テーマを表すサマリー情報
/// UIに表示するための軽量データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSummary {
    /// テーマID
    pub id: Uuid,
    /// テーマ名
    pub name: String,
    /// 作者名
    pub author: Option<String>,
    /// テーマバージョン
    pub version: String,
    /// 作成日時
    pub created_at: String,
    /// 適用中かどうか
    pub is_active: bool,
    /// お気に入りかどうか
    pub is_favorite: bool,
    /// 適用回数
    pub apply_count: u32,
    /// 含まれるカーソル役割の一覧
    pub included_roles: Vec<String>,
    /// テーマディレクトリのパス
    pub path: String,
}

/// テーママネージャー
pub struct ThemeManager;

impl ThemeManager {
    /// 指定 ID のテーマがディスク上に存在するかを確認する。
    /// `~/.custom_cursors/<UUID>/theme.json` の存在のみで判定する (中身は検証しない)。
    pub fn theme_exists(id: Uuid) -> bool {
        use crate::config::ConfigManager;
        let cursors_dir = match ConfigManager::cursors_dir() {
            Ok(d) => d,
            Err(_) => return false,
        };
        cursors_dir
            .join(id.to_string())
            .join("theme.json")
            .is_file()
    }

    /// 起動時の孤児カーソル復旧チェック。
    ///
    /// config が指すテーマ ID (`active_theme_id` / `dark_mode.{light,dark}_theme_id`) が
    /// ディスク上に存在しない場合、以下を実行する:
    ///  - `active_theme_id` が孤児: レジストリを Windows 既定に戻し、`active_theme_id = None`
    ///  - dark_mode 側の孤児: 該当フィールドを `None` に戻す (適用済みでなければレジストリは触らない)
    ///
    /// 何もする必要がなければ `Ok(false)` を返す。復旧した場合は `Ok(true)`。
    pub fn cleanup_orphan_references(config: &crate::config::ConfigManager) -> AppResult<bool> {
        use crate::registry::RegistryManager;

        let cfg = config.get()?;
        let active_orphan = cfg
            .general
            .active_theme_id
            .is_some_and(|id| !Self::theme_exists(id));
        let dark_orphan = cfg
            .dark_mode
            .dark_theme_id
            .is_some_and(|id| !Self::theme_exists(id));
        let light_orphan = cfg
            .dark_mode
            .light_theme_id
            .is_some_and(|id| !Self::theme_exists(id));

        if !active_orphan && !dark_orphan && !light_orphan {
            return Ok(false);
        }

        if active_orphan {
            tracing::warn!(
                "孤児カーソル検出: active_theme_id={:?} のディレクトリが消失 → Windows 既定へ復元",
                cfg.general.active_theme_id
            );
            // 失敗してもベストエフォートで config 側は修正する
            if let Err(e) = RegistryManager::reset_to_windows_default() {
                tracing::warn!("孤児復旧時の Windows 既定への戻し失敗: {}", e);
            }
        }
        if dark_orphan {
            tracing::warn!(
                "孤児カーソル検出: dark_mode.dark_theme_id={:?} のディレクトリが消失",
                cfg.dark_mode.dark_theme_id
            );
        }
        if light_orphan {
            tracing::warn!(
                "孤児カーソル検出: dark_mode.light_theme_id={:?} のディレクトリが消失",
                cfg.dark_mode.light_theme_id
            );
        }

        config.update(|c| {
            if active_orphan {
                c.general.active_theme_id = None;
            }
            if dark_orphan {
                c.dark_mode.dark_theme_id = None;
            }
            if light_orphan {
                c.dark_mode.light_theme_id = None;
            }
        })?;
        Ok(true)
    }

    /// インストール済みテーマの一覧を取得する。
    /// `active_id` (config.general.active_theme_id) と一致するテーマだけ
    /// `is_active = true` で返却する。
    pub fn list_themes(active_id: Option<Uuid>) -> AppResult<Vec<ThemeSummary>> {
        use crate::config::ConfigManager;

        let cursors_dir = ConfigManager::cursors_dir()?;
        let mut themes = Vec::new();

        if !cursors_dir.exists() {
            return Ok(themes);
        }

        // ~/.custom_cursors/ 配下の各ディレクトリをスキャン
        for entry in std::fs::read_dir(&cursors_dir)? {
            let entry = entry?;
            let path = entry.path();

            // _で始まる特殊ディレクトリはスキップ
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('_') {
                    continue;
                }
            }

            if !path.is_dir() {
                continue;
            }

            // theme.json を読み込む
            let theme_json_path = path.join("theme.json");
            if !theme_json_path.exists() {
                continue;
            }

            match Self::load_theme_summary(&theme_json_path, &path, active_id) {
                Ok(summary) => themes.push(summary),
                Err(e) => {
                    tracing::warn!(
                        "テーマの読み込みに失敗 ({}): {}",
                        crate::logging::redact_path(&path),
                        e
                    );
                }
            }
        }

        Ok(themes)
    }

    /// 指定テーマ ID のロール毎 PNG プレビューを返す。
    ///
    /// UI のテーマカード/ApplyModal で「実物の絵」を表示するために使う。
    /// `~/.custom_cursors/<UUID>/<role-file>` を読み、`.cur` / `.ico` は最大解像度を
    /// PNG 化し、`.png` 拡張子はそのままバイト列を返す。
    ///
    /// `roles_filter` が `Some` の場合は指定ロールのみ返す (カード用に Arrow だけ等)。
    pub fn load_role_previews(
        id: Uuid,
        roles_filter: Option<&[String]>,
    ) -> AppResult<HashMap<String, Vec<u8>>> {
        use crate::config::ConfigManager;
        use crate::cursor::{parse_ico_cur, pick_largest_as_png};

        let cursors_dir = ConfigManager::cursors_dir()?;
        let theme_dir = cursors_dir.join(id.to_string());
        let theme_json_path = theme_dir.join("theme.json");
        if !theme_json_path.is_file() {
            return Err(crate::errors::AppError::Theme(format!(
                "テーマ {} が見つかりません",
                id
            )));
        }
        let content = std::fs::read_to_string(&theme_json_path)?;
        let metadata: ThemeMetadata = serde_json::from_str(&content)?;

        let mut out: HashMap<String, Vec<u8>> = HashMap::new();
        for (role, def) in &metadata.cursors {
            if let Some(filter) = roles_filter {
                if !filter.iter().any(|r| r == role) {
                    continue;
                }
            }
            let abs = theme_dir.join(&def.file);
            if !abs.is_file() {
                continue;
            }
            let bytes = match std::fs::read(&abs) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("プレビュー読込失敗 ({}): {}", role, e);
                    continue;
                }
            };
            // 拡張子で判別: .png ならそのまま、それ以外は cur/ico として解釈
            let ext = abs
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let png_bytes = if ext == "png" {
                bytes
            } else {
                match parse_ico_cur(&bytes).and_then(|p| pick_largest_as_png(&p)) {
                    Ok((_, png)) => png,
                    Err(e) => {
                        tracing::warn!("{} の PNG 化に失敗: {}", role, e);
                        continue;
                    }
                }
            };
            out.insert(role.clone(), png_bytes);
        }
        Ok(out)
    }

    /// 指定 ID のテーマが現在実際にレジストリに適用されているかを判定する。
    ///
    /// `theme.json` に書かれた各役割の絶対パスと、`HKCU\Control Panel\Cursors`
    /// の現在値を比較する。ユーザーが Windows のマウスのプロパティで別スキーム
    /// に切り替えた / 既定へリセットした直後はここで `false` になり、UI の
    /// "active" 表示も外れる。
    pub fn theme_active_in_registry(id: Uuid) -> bool {
        use crate::config::ConfigManager;
        use crate::registry::paths_match_current_registry;

        let Ok(cursors_dir) = ConfigManager::cursors_dir() else {
            return false;
        };
        let theme_dir = cursors_dir.join(id.to_string());
        let theme_json_path = theme_dir.join("theme.json");
        if !theme_json_path.is_file() {
            return false;
        }
        let content = match std::fs::read_to_string(&theme_json_path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let metadata: ThemeMetadata = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(_) => return false,
        };
        let mut expected: HashMap<String, String> = HashMap::new();
        for (role, def) in &metadata.cursors {
            let abs = theme_dir.join(&def.file);
            expected.insert(role.clone(), abs.to_string_lossy().to_string());
        }
        paths_match_current_registry(&expected)
    }

    /// 任意の `.cur` / `.ico` / `.ani` ファイル 1 件を最大解像度の PNG にレンダリングする。
    ///
    /// ライブラリ画面で Windows のシステムスキーム (HKCU\Cursors\Schemes) を表示する際、
    /// `%SystemRoot%\cursors\*.cur` 等を直接サムネイル化するために使う。
    ///
    /// 拡張子で判別:
    ///  - `.png` ならバイト列をそのまま返す
    ///  - `.ani` なら `parse_ani` で先頭フレームを取り出して PNG 化
    ///  - それ以外 (`.cur` / `.ico`) は `parse_ico_cur` + `pick_largest_as_png`
    pub fn render_cursor_file_as_png(path: &std::path::Path) -> AppResult<Vec<u8>> {
        use crate::cursor::{parse_ani, parse_ico_cur, pick_largest_as_png};
        use crate::errors::AppError;
        let bytes = std::fs::read(path).map_err(|e| {
            AppError::ImageProcessing(format!(
                "ファイル読込失敗 ({}): {}",
                crate::logging::redact_path(path),
                e
            ))
        })?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "png" {
            return Ok(bytes);
        }
        if ext == "ani" {
            let parsed = parse_ani(&bytes)?;
            let frame = parsed.frames.first().ok_or_else(|| {
                AppError::ImageProcessing(".ani にフレームがありません".to_string())
            })?;
            let mut png = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut png);
            image::ImageEncoder::write_image(
                encoder,
                frame.image.as_raw(),
                frame.image.width(),
                frame.image.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| AppError::ImageProcessing(format!("PNG エンコード失敗: {}", e)))?;
            return Ok(png);
        }
        // .cur / .ico
        let parsed = parse_ico_cur(&bytes)?;
        let (_, png) = pick_largest_as_png(&parsed)?;
        Ok(png)
    }

    /// 役割名 → 絶対パス のマップから、各ロールのサムネイル PNG を生成する。
    ///
    /// 失敗するロールはスキップしてログに残す (1 つの壊れたファイルで全体表示を
    /// 諦めない設計)。空文字列パスもスキップ。
    pub fn render_paths_as_previews(
        cursor_paths: &HashMap<String, String>,
    ) -> HashMap<String, Vec<u8>> {
        let mut out: HashMap<String, Vec<u8>> = HashMap::new();
        for (role, raw) in cursor_paths {
            if raw.is_empty() {
                continue;
            }
            let path = std::path::PathBuf::from(raw);
            if !path.is_file() {
                tracing::debug!("scheme preview skip ({}): ファイル不在", role);
                continue;
            }
            match Self::render_cursor_file_as_png(&path) {
                Ok(bytes) => {
                    out.insert(role.clone(), bytes);
                }
                Err(e) => {
                    tracing::warn!("scheme preview {} のレンダリング失敗: {}", role, e);
                }
            }
        }
        out
    }

    /// Windows のレジストリスキームを `.cursorpack` として書き出す。
    ///
    /// クリエイターを介さず、ユーザーが既に Windows のマウスのプロパティで
    /// 保存している配色をそのままパッケージ化したい用途。`.cur` / `.ani` /
    /// `.ico` といった元の拡張子を保持したまま zip に格納し、theme.json には
    /// 各ロールの相対パス (`cursors/<role>.<ext>`) を記録する。
    ///
    /// `name` が空のロール (= 既定継承スロット) は theme.cursors に含めない。
    /// 戻り値: 書き込んだバイト数。
    pub fn export_scheme_as_cursorpack(
        scheme_name: &str,
        cursor_paths: &HashMap<String, String>,
        output_path: &std::path::Path,
        author: Option<&str>,
    ) -> AppResult<(Uuid, u64)> {
        use crate::errors::AppError;
        use std::io::Write;

        let entries: Vec<(String, std::path::PathBuf)> = cursor_paths
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.clone(), std::path::PathBuf::from(v)))
            .collect();
        if entries.is_empty() {
            return Err(AppError::Theme(
                "スキームに有効なカーソルファイルがありません".to_string(),
            ));
        }

        // theme.json のメタデータ構築
        let mut name_map: HashMap<String, String> = HashMap::new();
        name_map.insert("ja".into(), scheme_name.to_string());
        name_map.insert("en".into(), scheme_name.to_string());

        let mut cursors_meta: HashMap<String, CursorDefinition> = HashMap::new();
        let mut zip_files: Vec<(String, Vec<u8>)> = Vec::new();
        for (role, path) in &entries {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("cur")
                .to_ascii_lowercase();
            // パッケージ内パスは lower_case 拡張子で統一
            let rel = format!("cursors/{}.{}", role, ext);
            let bytes = std::fs::read(path).map_err(|e| {
                AppError::Theme(format!(
                    "{} ({}) の読込に失敗: {}",
                    role,
                    crate::logging::redact_path(path),
                    e
                ))
            })?;
            cursors_meta.insert(
                role.clone(),
                CursorDefinition {
                    file: rel.clone(),
                    hotspot_x: 0,
                    hotspot_y: 0,
                    resize_method: "lanczos".to_string(),
                    size_overrides: None,
                },
            );
            zip_files.push((rel, bytes));
        }

        let metadata = ThemeMetadata {
            schema_version: 1,
            id: Uuid::new_v4(),
            name: LocalizedString::Localized(name_map),
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            requires_os_shadow: true,
            cursors: cursors_meta,
            author: author
                .map(|s| s.to_string())
                .or_else(|| Some("Windows".to_string())),
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
        };

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(output_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let metadata_json = serde_json::to_vec_pretty(&metadata)?;
        zip.start_file("theme.json", opts)?;
        zip.write_all(&metadata_json)?;

        for (rel, bytes) in zip_files {
            zip.start_file(rel, opts)?;
            zip.write_all(&bytes)?;
        }
        zip.finish()?;

        let size = std::fs::metadata(output_path)?.len();
        tracing::info!(
            "exported scheme as cursorpack: '{}' ({} roles) → {} ({} bytes)",
            scheme_name,
            metadata.cursors.len(),
            crate::logging::redact_path(output_path),
            size
        );
        Ok((metadata.id, size))
    }

    /// theme.json からサマリー情報を読み込む
    fn load_theme_summary(
        theme_json_path: &std::path::Path,
        theme_dir: &std::path::Path,
        active_id: Option<Uuid>,
    ) -> AppResult<ThemeSummary> {
        let content = std::fs::read_to_string(theme_json_path)?;
        let metadata: ThemeMetadata = serde_json::from_str(&content)?;

        let included_roles: Vec<String> = metadata.cursors.keys().cloned().collect();
        let is_active = active_id == Some(metadata.id);

        Ok(ThemeSummary {
            id: metadata.id,
            name: metadata.name.get("ja"), // TODO: ロケールに応じて切替
            author: metadata.author,
            version: metadata.version,
            created_at: metadata.created_at,
            is_active,
            is_favorite: false, // TODO: config の favorites リストから判定
            apply_count: 0,     // TODO: config の usage 統計から判定
            included_roles,
            path: theme_dir.to_string_lossy().to_string(),
        })
    }

    /// `.cursorpack` (ZIP) のバイト列を `~/.custom_cursors/<UUID>/` に展開する。
    ///
    /// セキュリティ防御:
    ///  - 圧縮サイズの上限チェック (50 MB)
    ///  - 展開後合計サイズの上限チェック (200 MB) — 逐次計測で Zip 爆弾を遮断
    ///  - 個別ファイルサイズの上限チェック (10 MB)
    ///  - Path traversal 防御 (`..` / 絶対パスを拒否)
    ///  - シンボリックリンクと特殊エントリの拒否
    ///  - `theme.json` の Magic Byte (`{` の存在) と JSON パース成功を必須化
    ///  - レジストリキー注入対策: `.cursorpack` 内のファイル名はそのまま展開し、
    ///    レジストリ書き込み時に `apply_theme` が役割名を別途サニタイズ
    ///
    /// 戻り値: 展開後のテーマ ID (theme.json の `id` フィールド)。
    /// 既存の同 ID テーマがあれば上書きする。
    pub fn import_cursorpack_bytes(bytes: &[u8]) -> AppResult<Uuid> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        use std::io::{Cursor, Read};

        // 上限値 — 将来 config から取得する設計だが、現状は仕様書の固定値
        const MAX_COMPRESSED: u64 = 50 * 1024 * 1024;
        const MAX_UNCOMPRESSED_TOTAL: u64 = 200 * 1024 * 1024;
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

        if bytes.len() as u64 > MAX_COMPRESSED {
            return Err(AppError::Theme(format!(
                ".cursorpack 圧縮サイズ {} bytes が上限 {} を超えています",
                bytes.len(),
                MAX_COMPRESSED
            )));
        }

        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| AppError::Theme(format!(".cursorpack の解析に失敗: {}", e)))?;

        // 1) theme.json を先読みして ID を確定
        let metadata: ThemeMetadata = {
            let mut entry = archive
                .by_name("theme.json")
                .map_err(|_| AppError::Theme("theme.json が見つかりません".to_string()))?;
            // theme.json の Magic Byte: 先頭 1 バイトが `{`
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            if !buf.trim_start().starts_with('{') {
                return Err(AppError::Theme(
                    "theme.json が JSON ではありません".to_string(),
                ));
            }
            serde_json::from_str(&buf)?
        };

        let theme_id = metadata.id;
        let cursors_dir = ConfigManager::cursors_dir()?;
        let target_dir = cursors_dir.join(theme_id.to_string());

        // 既存テーマがあれば一旦削除 (上書きインポート)
        if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir)?;
        }
        std::fs::create_dir_all(&target_dir)?;

        // 2) 全エントリを安全に展開
        let mut total_uncompressed: u64 = 0;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let raw_name = entry.name().to_string();

            // シンボリックリンク・特殊ビットの拒否
            // unix_mode が `S_IFLNK = 0xA000` を含むなら symlink
            if let Some(mode) = entry.unix_mode() {
                const S_IFMT: u32 = 0xF000;
                const S_IFLNK: u32 = 0xA000;
                if mode & S_IFMT == S_IFLNK {
                    return Err(AppError::Theme(format!(
                        "シンボリックリンクを含む .cursorpack は受け入れません: {}",
                        raw_name
                    )));
                }
            }

            // Path traversal 防御
            let safe_path = sanitize_archive_path(&raw_name)?;
            let dest = target_dir.join(&safe_path);

            // 念のため target_dir 配下に収まるか再チェック (canonicalize は存在前提なので
            // ここでは構造的にチェック)
            if !dest.starts_with(&target_dir) {
                return Err(AppError::Theme(format!(
                    "Path traversal を検出: {}",
                    raw_name
                )));
            }

            if entry.is_dir() {
                std::fs::create_dir_all(&dest)?;
                continue;
            }

            // 個別ファイルサイズ
            if entry.size() > MAX_FILE_SIZE {
                return Err(AppError::Theme(format!(
                    "ファイル {} のサイズ {} bytes が上限 {} を超えています",
                    raw_name,
                    entry.size(),
                    MAX_FILE_SIZE
                )));
            }

            // 累積サイズ (Zip 爆弾の最終防衛線)
            total_uncompressed = total_uncompressed.saturating_add(entry.size());
            if total_uncompressed > MAX_UNCOMPRESSED_TOTAL {
                let _ = std::fs::remove_dir_all(&target_dir);
                return Err(AppError::Theme(format!(
                    "展開後合計サイズが上限 {} bytes を超えました",
                    MAX_UNCOMPRESSED_TOTAL
                )));
            }

            // 親ディレクトリ確保 + 書き込み
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
        }

        tracing::info!(
            "テーマ {} ({}) を展開: {} bytes / {} files",
            metadata.name.get("ja"),
            theme_id,
            total_uncompressed,
            archive.len()
        );

        Ok(theme_id)
    }

    /// `.cursorpack` をローカルファイルパスから読み込んでインポートする。
    pub fn import_cursorpack_file(path: &std::path::Path) -> AppResult<Uuid> {
        let bytes = std::fs::read(path)?;
        Self::import_cursorpack_bytes(&bytes)
    }

    /// `.cursorpack` を新規エクスポートする。
    ///
    /// 引数:
    ///  - `metadata`: `theme.json` の内容
    ///  - `cursors`: 役割名 → 役割用 `.cur` バイト列 のマップ
    ///  - `output_path`: 書き出し先の絶対パス
    ///
    /// theme.json の `cursors[role].file` は自動的に `cursors/<role>.cur` にリライトされる。
    /// 戻り値: 書き込んだバイト数。
    pub fn export_cursorpack(
        metadata: &mut ThemeMetadata,
        cursors: &HashMap<String, Vec<u8>>,
        output_path: &std::path::Path,
    ) -> AppResult<u64> {
        use crate::errors::AppError;
        use std::io::Write;

        // theme.json の `file` 参照を `cursors/<role>.cur` に統一
        for (role, def) in metadata.cursors.iter_mut() {
            def.file = format!("cursors/{}.cur", role);
        }

        // 全カーソル分のバイトが揃っているか検証
        for role in metadata.cursors.keys() {
            if !cursors.contains_key(role) {
                return Err(AppError::Theme(format!(
                    "役割 {} のカーソルバイト列が指定されていません",
                    role
                )));
            }
        }

        // メタデータをシリアライズ
        let metadata_json = serde_json::to_vec_pretty(metadata)?;

        // Zip 書き出し
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(output_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("theme.json", opts)?;
        zip.write_all(&metadata_json)?;

        for (role, bin) in cursors {
            zip.start_file(format!("cursors/{}.cur", role), opts)?;
            zip.write_all(bin)?;
        }
        zip.finish()?;

        let size = std::fs::metadata(output_path)?.len();
        tracing::info!(
            "exported cursorpack: theme={} ({}) → {} ({} bytes)",
            metadata.name.get("ja"),
            metadata.id,
            crate::logging::redact_path(output_path),
            size
        );
        Ok(size)
    }

    /// `.cursorpack` のバイト列から theme.json だけを読み出す軽量検査。
    /// 既存ライブラリ内に同 ID のテーマがあるか、バージョン比較する用途に使用。
    pub fn inspect_cursorpack_bytes(bytes: &[u8]) -> AppResult<CursorpackInspection> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        use std::io::{Cursor, Read};

        const MAX_COMPRESSED: u64 = 50 * 1024 * 1024;
        if bytes.len() as u64 > MAX_COMPRESSED {
            return Err(AppError::Theme(format!(
                ".cursorpack 圧縮サイズ {} が上限 {} を超えています",
                bytes.len(),
                MAX_COMPRESSED
            )));
        }

        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| AppError::Theme(format!(".cursorpack の解析に失敗: {}", e)))?;

        let metadata: ThemeMetadata = {
            let mut entry = archive
                .by_name("theme.json")
                .map_err(|_| AppError::Theme("theme.json が見つかりません".to_string()))?;
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            if !buf.trim_start().starts_with('{') {
                return Err(AppError::Theme(
                    "theme.json が JSON ではありません".to_string(),
                ));
            }
            serde_json::from_str(&buf)?
        };

        // 既存ライブラリと衝突チェック
        let cursors_dir = ConfigManager::cursors_dir()?;
        let existing_dir = cursors_dir.join(metadata.id.to_string());
        let existing = if existing_dir.exists() {
            let theme_json = existing_dir.join("theme.json");
            if theme_json.exists() {
                let content = std::fs::read_to_string(&theme_json)?;
                let existing_meta: ThemeMetadata = serde_json::from_str(&content)?;
                Some(ExistingTheme {
                    name: existing_meta.name.get("ja"),
                    version: existing_meta.version,
                    author: existing_meta.author,
                    role_count: existing_meta.cursors.len() as u32,
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok(CursorpackInspection {
            id: metadata.id,
            name: metadata.name.get("ja"),
            version: metadata.version,
            author: metadata.author,
            role_count: metadata.cursors.len() as u32,
            existing,
        })
    }

    pub fn inspect_cursorpack_file(path: &std::path::Path) -> AppResult<CursorpackInspection> {
        let bytes = std::fs::read(path)?;
        Self::inspect_cursorpack_bytes(&bytes)
    }

    /// 指定 ID のテーマを探してレジストリに適用する。
    ///
    /// 流れ:
    /// 1. `~/.custom_cursors/<theme>/theme.json` を走査して該当テーマを検索
    /// 2. metadata.cursors から `役割名 → 絶対カーソルファイルパス` のマップを構築
    /// 3. `RegistryManager::apply_cursors` でレジストリ書き込み + SPI_SETCURSORS
    ///
    /// 内部で `RegistryManager` がスナップショット保存・失敗時ロールバックを担う。
    pub fn apply_theme(id: Uuid) -> AppResult<()> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        use crate::registry::RegistryManager;
        use std::path::PathBuf;

        let cursors_dir = ConfigManager::cursors_dir()?;

        // 該当 ID のテーマディレクトリを線形探索 (テーマ数は通常 < 100)
        let mut target: Option<(PathBuf, ThemeMetadata)> = None;
        if cursors_dir.exists() {
            for entry in std::fs::read_dir(&cursors_dir)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('_') {
                        continue;
                    }
                }
                let theme_json = path.join("theme.json");
                if !theme_json.exists() {
                    continue;
                }
                let content = std::fs::read_to_string(&theme_json)?;
                let metadata: ThemeMetadata = serde_json::from_str(&content)?;
                if metadata.id == id {
                    target = Some((path, metadata));
                    break;
                }
            }
        }

        let (theme_dir, metadata) =
            target.ok_or_else(|| AppError::Theme(format!("テーマ {} が見つかりません", id)))?;

        // 役割名 → 絶対パスのマップを構築
        let mut cursor_paths: HashMap<String, PathBuf> = HashMap::new();
        for (role, def) in &metadata.cursors {
            let abs = theme_dir.join(&def.file);
            if !abs.exists() {
                tracing::warn!(
                    "カーソルファイルが存在しない: {} ({})",
                    role,
                    crate::logging::redact_path(&abs)
                );
                continue;
            }
            cursor_paths.insert(role.clone(), abs);
        }

        if cursor_paths.is_empty() {
            return Err(AppError::Theme(
                "適用可能なカーソルファイルが見つかりません".to_string(),
            ));
        }

        RegistryManager::apply_cursors(&cursor_paths)?;

        // Windows のコントロールパネルから参照可能なよう Schemes にも登録する。
        // 失敗しても適用自体は成功扱い (UX への影響は最小限)
        let scheme_name = format!("EasyCursorSwap - {}", metadata.name.get("ja"));
        if let Err(e) = RegistryManager::register_scheme(&scheme_name, &cursor_paths) {
            tracing::warn!("Schemes 登録に失敗 (適用自体は成功): {}", e);
        }

        // OS 標準ポインターの影制御
        if let Err(e) = RegistryManager::set_cursor_shadow(metadata.requires_os_shadow) {
            tracing::warn!("ポインター影設定の更新に失敗: {}", e);
        }

        tracing::info!("テーマ {} を適用しました", metadata.name.get("ja"));
        Ok(())
    }

    #[allow(dead_code)]
    /// テーマ名のサニタイズ
    /// レジストリキーとして安全な文字列に変換
    pub fn sanitize_theme_name(name: &str) -> String {
        name.chars()
            .filter(|c| {
                c.is_alphanumeric()
                    || *c == ' '
                    || *c == '-'
                    || *c == '_'
                    || *c == '.'
                    // 日本語文字も許可
                    || (*c >= '\u{3000}' && *c <= '\u{9FFF}')
                    || (*c >= '\u{F900}' && *c <= '\u{FAFF}')
            })
            // NULLバイトとバックスラッシュを完全除去
            .filter(|c| *c != '\0' && *c != '\\')
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// 指定 ID のテーマディレクトリ (`~/.custom_cursors/<UUID>/`) を完全削除する。
    ///
    /// アクティブテーマを削除した場合、レジストリ側は EasyCursorSwap が書いたパスが
    /// ファイル不在になるが Windows は読めない値で既定にフォールバックするので追加
    /// cleanup は不要。呼び出し側は config.active_theme_id を None に戻す責任を持つ。
    pub fn delete_theme(id: Uuid) -> AppResult<()> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        let cursors_dir = ConfigManager::cursors_dir()?;
        let theme_dir = cursors_dir.join(id.to_string());
        if !theme_dir.exists() {
            return Err(AppError::Theme(format!("テーマ {} が見つかりません", id)));
        }
        std::fs::remove_dir_all(&theme_dir)?;
        tracing::info!("テーマ {} を削除しました", id);
        Ok(())
    }

    /// 指定 ID のテーマを複製し、新しい UUID を持つ別テーマとして保存する。
    ///
    /// `theme.json` は同内容のコピーで `id` だけ新規 UUID に、`name` には ` (Copy)` を
    /// 末尾付与して識別しやすくする。戻り値は新テーマの UUID。
    pub fn duplicate_theme(source_id: Uuid) -> AppResult<Uuid> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        let cursors_dir = ConfigManager::cursors_dir()?;
        let source_dir = cursors_dir.join(source_id.to_string());
        if !source_dir.exists() {
            return Err(AppError::Theme(format!(
                "複製元テーマ {} が見つかりません",
                source_id
            )));
        }

        let source_theme_json = source_dir.join("theme.json");
        let mut metadata: ThemeMetadata =
            serde_json::from_str(&std::fs::read_to_string(&source_theme_json)?)?;

        let new_id = Uuid::new_v4();
        metadata.id = new_id;
        let original_name = metadata.name.clone();
        metadata.name = clone_with_suffix(&original_name, " (Copy)");

        let target_dir = cursors_dir.join(new_id.to_string());
        copy_dir_recursive(&source_dir, &target_dir)?;
        std::fs::write(
            target_dir.join("theme.json"),
            serde_json::to_vec_pretty(&metadata)?,
        )?;

        tracing::info!("テーマ {} を {} として複製しました", source_id, new_id);
        Ok(new_id)
    }

    /// 既存テーマディレクトリを `.cursorpack` (ZIP) に再パッケージしてエクスポートする。
    ///
    /// クリエイターを経由せずライブラリから直接書き出したい場合に使う。
    /// theme.json + 全カーソルファイルをそのままアーカイブする。
    pub fn repackage_theme(id: Uuid, output_path: &std::path::Path) -> AppResult<u64> {
        use crate::config::ConfigManager;
        use crate::errors::AppError;
        let cursors_dir = ConfigManager::cursors_dir()?;
        let theme_dir = cursors_dir.join(id.to_string());
        if !theme_dir.exists() {
            return Err(AppError::Theme(format!("テーマ {} が見つかりません", id)));
        }
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = std::fs::File::create(output_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip_dir_recursive(&theme_dir, &theme_dir, &mut zip, opts)?;
        zip.finish()?;

        let size = std::fs::metadata(output_path)?.len();
        tracing::info!(
            "repackaged theme: {} -> {} ({} bytes)",
            id,
            crate::logging::redact_path(output_path),
            size
        );
        Ok(size)
    }
}

/// LocalizedString の各ロケール文字列にサフィックスを付けたコピーを返す。
///
/// serde_json::Value 経由で実装することで、LocalizedString が将来増減する
/// ロケールに自動追従する。
fn clone_with_suffix(src: &LocalizedString, suffix: &str) -> LocalizedString {
    let mut value: serde_json::Value = match serde_json::to_value(src) {
        Ok(v) => v,
        Err(_) => return src.clone(),
    };
    if let Some(map) = value.as_object_mut() {
        for (_, v) in map.iter_mut() {
            if let Some(s) = v.as_str() {
                *v = serde_json::Value::String(format!("{}{}", s, suffix));
            }
        }
    }
    serde_json::from_value(value).unwrap_or_else(|_| src.clone())
}

/// `from` 配下を `to` に再帰コピーする。シンボリックリンクは追わず通常ファイルとして扱う。
fn copy_dir_recursive(from: &std::path::Path, to: &std::path::Path) -> AppResult<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            copy_dir_recursive(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

/// `dir` 配下を `root` からの相対パスで Zip に書き込む。
fn zip_dir_recursive<W: std::io::Write + std::io::Seek>(
    dir: &std::path::Path,
    root: &std::path::Path,
    zip: &mut zip::ZipWriter<W>,
    opts: zip::write::SimpleFileOptions,
) -> AppResult<()> {
    use std::io::Write;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if entry.metadata()?.is_dir() {
            zip_dir_recursive(&path, root, zip, opts)?;
        } else {
            let bytes = std::fs::read(&path)?;
            zip.start_file(rel_str, opts)?;
            zip.write_all(&bytes)?;
        }
    }
    Ok(())
}

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
fn sanitize_archive_path(name: &str) -> AppResult<std::path::PathBuf> {
    use crate::errors::AppError;
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

    #[test]
    fn theme_exists_returns_false_for_random_uuid() {
        // ~/.custom_cursors/<random-uuid>/theme.json はまず存在しないので false
        let id = uuid::Uuid::new_v4();
        assert!(!super::ThemeManager::theme_exists(id));
    }
}
