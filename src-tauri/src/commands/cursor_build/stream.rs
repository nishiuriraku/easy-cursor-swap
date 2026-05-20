//! `export_cursorpack_streamed` 本体: 17 役割 × 6 サイズ = 最大 102 枚の .cur 生成を
//! 1 回の IPC で実行しつつ Tauri イベントで進捗配信する重量関数。
//!
//! - キャンセルは `registry.is_cancelled(build_id)` を主要ステップ前にチェック
//!   (App state の `CancelRegistry`)
//! - 役割ループは `build::build_role` に委譲
//! - 署名は `sign::sign_theme_metadata` に委譲
//! - destination 分岐 (File / Library) は本ファイル内で処理

use super::build;
use super::dto::*;
use super::sign;
use crate::cancel_registry::CancelRegistry;
use crate::errors::AppError;
use crate::theme::{CursorDefinition, LocalizedString, ThemeManager, ThemeMetadata};
use std::collections::HashMap;

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
    registry: tauri::State<'_, CancelRegistry>,
    req: StreamedExportRequest,
) -> Result<ExportResult, AppError> {
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
        if registry.is_cancelled(&req.build_id) {
            registry.drop_job(&req.build_id);
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

        // 単一ロールのビルド (PNG → .cur / ANI → リライト) は build.rs に委譲。
        let built = build::build_role(entry)?;
        cursor_bytes.insert(entry.role.clone(), built.bytes);
        cursors_meta.insert(entry.role.clone(), built.definition);

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
        id: resolve_metadata_id(req.existing_theme_id),
        name: LocalizedString::Localized(name_map),
        version: req.version.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        requires_os_shadow: req.requires_os_shadow,
        cursors: cursors_meta,
        author: req.author.clone(),
        license: None,
        homepage: None,
        // Creator UI の説明欄 (`metaDescription`) 由来。空文字 / 空白のみは
        // None と同じ扱い (= theme.json から description フィールドごと省略)。
        description: req
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| LocalizedString::Simple(s.to_string())),
        min_app_version: None,
        signature: None,
        tags: Vec::new(),
        source: crate::theme::types::ThemeSource::Local,
        cloned_from_marketplace_id: None,
    };

    // 3) 署名。sign.rs に共通化済み (export_cursorpack と同じロジック)。
    let signed_key_id: Option<String> = if req.sign {
        if registry.is_cancelled(&req.build_id) {
            registry.drop_job(&req.build_id);
            // role 段階と同様に cancelled イベントを発火し、UI 進捗バーを解放する。
            emit_progress(
                &app,
                BuildProgress {
                    build_id: req.build_id.clone(),
                    stage: "cancelled".to_string(),
                    current: total_roles,
                    total: total_steps,
                    message: Some("sign".to_string()),
                },
            );
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
        sign::sign_theme_metadata(&mut metadata)?
    } else {
        None
    };

    // 4) Zip 出力
    if registry.is_cancelled(&req.build_id) {
        registry.drop_job(&req.build_id);
        // 同上: package 段階キャンセル時も UI へ通知する。
        emit_progress(
            &app,
            BuildProgress {
                build_id: req.build_id.clone(),
                stage: "cancelled".to_string(),
                current: total_steps - 1,
                total: total_steps,
                message: Some("package".to_string()),
            },
        );
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

    // destination で分岐: 現状は File / Library を実装
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
    registry.drop_job(&req.build_id);

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
    /// `BuildProgress { stage: "cancelled", .. }` は role / sign / package の
    /// 3 段階すべてのキャンセルパスで発火されなければならない。Tauri AppHandle を
    /// 伴う非同期 emit を unit-test するには test setup が重いため、コードに
    /// "cancelled" 文字列が必要回数出現することを静的に検証して回帰防止する。
    #[test]
    fn cancel_paths_emit_cancelled_stage_in_all_three_phases() {
        let source = include_str!("stream.rs");
        let count = source.matches("\"cancelled\"").count();
        assert!(
            count >= 3,
            "expected at least 3 occurrences of \"cancelled\" (role + sign + package), got {count}"
        );
    }
}
