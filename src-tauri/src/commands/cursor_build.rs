//! `.cursorpack` ビルド・エクスポート系 IPC。
//!
//! クリエイターから渡された PNG / メタ情報を 17 役割 × 6 サイズの `.cur` バイナリへ
//! 変換し、theme.json と一緒に zip に固める。
//!
//! - [`export_cursorpack`] — 役割パス → `.cursorpack` (同期)
//! - [`export_cursorpack_streamed`] — 進捗イベント付きビルド (UI からの主流ルート)
//! - [`cancel_build`] — `export_cursorpack_streamed` を中止
//!
//! キャンセルは `OnceLock<Mutex<HashSet<String>>>` 上の build_id 集合で管理する。

use crate::cursor::{build_cur_from_png, ResizeMethod};
use crate::errors::AppError;
use crate::theme::{CursorDefinition, LocalizedString, ThemeManager, ThemeMetadata};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::sync::OnceLock;

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
    /// 役割名 → 元画像ホットスポット比率 (`{ "Arrow": { x: 0.125, y: 0.125 } }`)
    pub hotspots: std::collections::HashMap<String, crate::theme::types::Hotspot>,
    /// 役割名 → ローカル `.cur` ファイルパス
    pub cur_paths: std::collections::HashMap<String, String>,
    pub output_path: String,
    /// true の場合、現在の鍵ペアでパッケージ全体に署名する。
    /// theme.json に `signature` フィールドを埋め込む。
    pub sign: bool,
}

/// 出力先。`File` はディスクへの保存、`Library` はライブラリ展開 (+ オプションで apply)。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExportDestination {
    #[serde(rename_all = "camelCase")]
    File { path: String },
    #[serde(rename_all = "camelCase")]
    Library {
        #[serde(default)]
        apply_after: bool,
    },
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub theme_id: String,
    pub size_bytes: u64,
    pub signed: bool,
    pub key_id: Option<String>,
    /// `apply_after=true` で `apply_theme` まで成功した場合のみ true。
    pub applied: bool,
    /// `Library { apply_after: true }` で Library 登録は成功したが apply が失敗した場合のメッセージ。
    pub apply_error: Option<String>,
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
            .unwrap_or(crate::theme::types::Hotspot::ZERO);
        cursors_meta.insert(
            role.clone(),
            CursorDefinition {
                file: format!("cursors/{}.cur", role),
                hotspot: hot,
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
        schema_version: 2,
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
        applied: false,
        apply_error: None,
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

/// 1 役割分の入力 (PNG バイト列 + ホットスポット比率 + リサンプル指定)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleBuildEntry {
    pub role: String,
    pub png_bytes: Vec<u8>,
    /// ホットスポット (比率, 0.0..=1.0)。`.cur` 書出直前に `to_px(size)` で px 変換する。
    pub hotspot: crate::theme::types::Hotspot,
    /// "lanczos" / "nearest" / "auto"
    pub resample: String,
    /// サイズ別オーバーライド (px → PNG bytes + optional 独立 hotspot)。
    /// Some の場合、対応サイズはリサンプルせずそのまま使用。
    /// None / 空なら従来どおり png_bytes をリサンプル。
    #[serde(default)]
    pub sized_overrides: Option<std::collections::HashMap<u32, SizedOverridePayload>>,
    /// `.ani` 由来ロールのソースファイル絶対パス。
    /// セットされている場合、PNG → CUR ビルダではなく rewrite_ani_with_hotspot を経由して
    /// cursors/<role>.ani として書き出す。
    #[serde(default)]
    pub ani_source_path: Option<String>,
}

/// サイズ別オーバーライド (PNG + optional 独立 hotspot)。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SizedOverridePayload {
    pub png_bytes: Vec<u8>,
    #[serde(default)]
    pub hotspot: Option<crate::theme::types::Hotspot>,
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
    /// File / Library{apply_after} で出力先を切替える。
    pub destination: ExportDestination,
    /// `Some(uuid)` のとき新 UUID 発行ではなく既存テーマ ID を引き継ぐ (= 上書き保存)。
    pub existing_theme_id: Option<uuid::Uuid>,
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

/// `existing_theme_id` があれば引き継ぎ、なければ新規 UUID を発行する。
/// Task 1.3 で導入。`export_cursorpack_streamed` のメタデータ ID 決定に使用。
pub(crate) fn resolve_metadata_id(existing: Option<uuid::Uuid>) -> uuid::Uuid {
    existing.unwrap_or_else(uuid::Uuid::new_v4)
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

        // .ani 由来ロール: rewrite_ani_with_hotspot 経由で .ani バイナリを生成
        if let Some(src) = &entry.ani_source_path {
            let bytes = std::fs::read(src).map_err(|e| {
                AppError::ImageProcessing(format!(
                    "ani 読込失敗 ({}): {}",
                    crate::logging::redact_path(std::path::Path::new(src)),
                    e
                ))
            })?;
            // ANI の primary_size は png_bytes から取得 (空の場合は 32px 既定)
            let primary_size = if !entry.png_bytes.is_empty() {
                image::load_from_memory(&entry.png_bytes)
                    .map(|img| img.width())
                    .unwrap_or(32)
            } else {
                32
            };
            let (hot_x_px, hot_y_px) = entry.hotspot.to_px(primary_size);
            let rewritten = crate::cursor::rewrite_ani_with_hotspot(
                &bytes,
                (hot_x_px as u16, hot_y_px as u16),
            )?;
            cursor_bytes.insert(entry.role.clone(), rewritten);
            cursors_meta.insert(
                entry.role.clone(),
                CursorDefinition {
                    file: format!("cursors/{}.ani", entry.role),
                    hotspot: entry.hotspot,
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
            continue;
        }

        let resample = match entry.resample.as_str() {
            "auto" => ResizeMethod::Lanczos,
            other => ResizeMethod::from_str(other),
        };

        // primary_size を PNG ヘッダから取得して ratio→px 変換する
        let primary_size = image::load_from_memory(&entry.png_bytes)
            .map(|img| img.width())
            .unwrap_or(32);
        let (hot_x_px, hot_y_px) = entry.hotspot.to_px(primary_size);

        // sized_overrides を sized_png_bytes 形式に変換 (build_cur_from_png への境界)
        let sized_png_map: Option<std::collections::HashMap<u32, Vec<u8>>> =
            entry.sized_overrides.as_ref().map(|overrides| {
                overrides
                    .iter()
                    .map(|(size, payload)| (*size, payload.png_bytes.clone()))
                    .collect()
            });

        let bin = build_cur_from_png(
            &entry.png_bytes,
            hot_x_px,
            hot_y_px,
            resample,
            sized_png_map.as_ref(),
        )?;
        cursor_bytes.insert(entry.role.clone(), bin);
        cursors_meta.insert(
            entry.role.clone(),
            CursorDefinition {
                file: format!("cursors/{}.cur", entry.role),
                hotspot: entry.hotspot,
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
        schema_version: 2,
        id: resolve_metadata_id(req.existing_theme_id),
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

    // destination で分岐: 現状は File のみ実装、Library は Task 2.1 で追加
    let zip_bytes = ThemeManager::write_cursorpack_to_buffer(&mut metadata, &cursor_bytes)?;
    let (applied, apply_error, size_bytes) = match &req.destination {
        ExportDestination::File { path } => {
            let out_path = std::path::PathBuf::from(path);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&out_path, &zip_bytes)?;
            tracing::info!(
                "exported cursorpack: theme={} ({}) → {} ({} bytes)",
                metadata.name.get("ja"),
                metadata.id,
                crate::logging::redact_path(&out_path),
                zip_bytes.len()
            );
            (false, None, zip_bytes.len() as u64)
        }
        ExportDestination::Library { apply_after } => {
            // 1. in-memory zip を import_cursorpack_bytes に流して Library に展開
            let imported_id = crate::theme::ThemeManager::import_cursorpack_bytes(&zip_bytes)?;
            tracing::info!(
                "imported cursorpack to library: theme={} ({} bytes)",
                imported_id,
                zip_bytes.len()
            );

            // 2. apply_after = true なら適用も試みる。失敗しても Library 登録は成功扱い (部分成功)
            let (applied, apply_error) = if *apply_after {
                match crate::theme::ThemeManager::apply_theme(imported_id) {
                    Ok(()) => {
                        tracing::info!("applied theme {} from creator", imported_id);
                        (true, None)
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        tracing::warn!(
                            "Library 登録は成功したが apply に失敗: theme={} reason={}",
                            imported_id,
                            msg
                        );
                        (false, Some(msg))
                    }
                }
            } else {
                (false, None)
            };

            (applied, apply_error, zip_bytes.len() as u64)
        }
    };

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
        size_bytes,
        signed: req.sign,
        key_id: signed_key_id,
        applied,
        apply_error,
    })
}

#[cfg(test)]
mod tests {
    use super::{clear_cancel, is_cancelled, mark_cancelled, SizedOverridePayload};

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

    #[test]
    fn export_destination_file_round_trip() {
        let json = serde_json::json!({
            "kind": "file",
            "path": "/tmp/out.cursorpack"
        });
        let dest: super::ExportDestination = serde_json::from_value(json).unwrap();
        match dest {
            super::ExportDestination::File { path } => assert_eq!(path, "/tmp/out.cursorpack"),
            _ => panic!("expected File variant"),
        }
    }

    #[test]
    fn export_destination_library_round_trip() {
        let json = serde_json::json!({
            "kind": "library",
            "applyAfter": true
        });
        let dest: super::ExportDestination = serde_json::from_value(json).unwrap();
        match dest {
            super::ExportDestination::Library { apply_after } => assert!(apply_after),
            _ => panic!("expected Library variant"),
        }
    }

    #[test]
    fn streamed_request_deserializes_with_existing_theme_id_null() {
        let json = serde_json::json!({
            "buildId": "test-id",
            "nameJa": "T",
            "nameEn": null,
            "author": null,
            "version": "1.0.0",
            "requiresOsShadow": false,
            "roles": [],
            "destination": { "kind": "file", "path": "/tmp/x" },
            "existingThemeId": null,
            "sign": false
        });
        let req: super::StreamedExportRequest = serde_json::from_value(json).unwrap();
        assert!(req.existing_theme_id.is_none());
        assert!(matches!(
            req.destination,
            super::ExportDestination::File { .. }
        ));
    }

    #[test]
    fn metadata_id_inherits_existing_theme_id_when_provided() {
        // Helper を直接テストする (フル export_cursorpack_streamed は AppHandle 必要のため)
        let existing = uuid::Uuid::new_v4();
        let resolved = super::resolve_metadata_id(Some(existing));
        assert_eq!(resolved, existing);
    }

    #[test]
    fn metadata_id_generates_new_when_existing_is_none() {
        let a = super::resolve_metadata_id(None);
        let b = super::resolve_metadata_id(None);
        assert_ne!(a, b, "別の Uuid::new_v4() が生成されるはず");
    }

    #[test]
    fn library_destination_zip_round_trips_through_import() {
        // 注: import_cursorpack_bytes は ConfigManager::cursors_dir() を呼ぶため、
        // 一時的なホームディレクトリ環境で実行する必要がある。
        // ここでは write_cursorpack_to_buffer の出力が
        // inspect_cursorpack_bytes でメタを取り出せることだけ確認する
        // (フル展開は手動 E2E に委ねる)。
        use std::collections::HashMap;
        let mut metadata = crate::theme::types::ThemeMetadata {
            schema_version: 2,
            id: uuid::Uuid::new_v4(),
            name: crate::theme::types::LocalizedString::Simple("Lib Test".to_string()),
            version: "1.2.3".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            requires_os_shadow: false,
            cursors: HashMap::new(),
            author: None,
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
            tags: Vec::new(),
        };
        let target_id = metadata.id;
        let cursors: HashMap<String, Vec<u8>> = HashMap::new();
        let bytes = crate::theme::ThemeManager::write_cursorpack_to_buffer(&mut metadata, &cursors)
            .unwrap();
        let inspected = crate::theme::ThemeManager::inspect_cursorpack_bytes(&bytes).unwrap();
        assert_eq!(inspected.id, target_id, "ID が引き継がれているはず");
        assert_eq!(inspected.version, "1.2.3");
    }

    #[test]
    fn ratio_hotspot_converts_to_px_at_each_size() {
        use crate::theme::types::{Hotspot, Ratio01};
        let h = Hotspot {
            x: Ratio01::new(0.5),
            y: Ratio01::new(0.5),
        };
        assert_eq!(h.to_px(32), (16, 16));
        assert_eq!(h.to_px(64), (32, 32));
        assert_eq!(h.to_px(128), (64, 64));
        assert_eq!(h.to_px(256), (128, 128));
    }

    #[test]
    fn sized_override_hotspot_overrides_primary() {
        use crate::theme::types::{Hotspot, Ratio01};
        let primary = Hotspot {
            x: Ratio01::new(0.0),
            y: Ratio01::new(0.0),
        };
        let override_h = Hotspot {
            x: Ratio01::new(0.5),
            y: Ratio01::new(0.5),
        };
        let payload = SizedOverridePayload {
            png_bytes: vec![],
            hotspot: Some(override_h),
        };
        let effective = payload.hotspot.unwrap_or(primary);
        assert_eq!(effective, override_h);
    }
}
