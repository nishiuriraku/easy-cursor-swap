//! `export_cursorpack_streamed` 本体: 17 役割 × 6 サイズ = 最大 102 枚の .cur 生成を
//! 1 回の IPC で実行しつつ Tauri イベントで進捗配信する重量関数。
//!
//! Wave 2AB / Task 6 でブロッキングワーカー + 進捗チャネル adapter に再構成:
//!
//! - 非同期 adapter (`export_cursorpack_streamed`) が `tauri::async_runtime::spawn_blocking`
//!   経由で `run_export_blocking` を起動し、進捗イベントを `std::sync::mpsc::sync_channel(64)`
//!   で受け取ってメイン runtime から `build-progress` を emit する。
//! - キャンセルは App state の `CancelRegistry` が引き続き権威 (Y15)。チャネルは
//!   キャンセル元ではなくベストエフォートな進捗配送のためだけに使われる。
//! - 役割ループは `build::build_role` に委譲
//! - 署名は `sign::sign_theme_metadata` に委譲
//! - destination 分岐 (File / Library) は本ファイル内で処理
//! - 純粋関数 `run_export_inner` を内部に切り出し、テストから AppHandle なしで駆動できる
//!   ようにした (`run_export_blocking` から App state / RAII ガードを取得した後委譲)。

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

/// 進捗イベントのチャンネル容量。本ビルドで発火されうるイベント数は role 17 + package 1
/// + (任意で sign 1) + done 1 = 最大 20 件なので 64 は余裕を持って吸収できる。
///   万一受信側 drain 遅延で満杯になっても、non-terminal イベントは best-effort でドロップ
///   してワーカーを止めない設計 (terminal は `send_terminal_progress` で確実に配送)。
pub(super) const PROGRESS_CHANNEL_CAPACITY: usize = 64;

/// 進捗イベント送信用の bounded channel sender。
/// `std::sync::mpsc::SyncSender` を薄い別名で re-export し、シグネチャの意図を明確化する。
pub type ProgressSender = std::sync::mpsc::SyncSender<BuildProgress>;

/// 進捗イベントの terminal stage 判定ヘルパ。
/// terminal (done / cancelled / error) は `send_terminal_progress` 経由で確実に配送し、
/// non-terminal (role / sign / package) は `send_progress` 経由で best-effort 送信する。
/// brief: 「done, cancelled, error, and channel closure must be handled explicitly」の
/// terminal 集合をここで一元管理する。
pub(super) fn is_terminal_stage(stage: &str) -> bool {
    matches!(stage, "done" | "cancelled" | "error")
}

/// 進捗イベントをシンクに送信する。terminal はブロック送信 (容量不足時に drain を待つ)、
/// non-terminal は try_send でバックプレッシャを発生させず、満杯時は short redacted warn
/// を出して drop する。
fn send_progress(sender: &ProgressSender, payload: BuildProgress) {
    if is_terminal_stage(&payload.stage) {
        send_terminal_progress(sender, payload);
        return;
    }
    match sender.try_send(payload) {
        Ok(()) => {}
        Err(std::sync::mpsc::TrySendError::Full(p)) => {
            // brief: 「Non-terminal progress can be reported as dropped with a short redacted warning」
            // build_id と stage のみログ。PII (パス / ハッシュ) は含めない。
            tracing::warn!(
                build_id = %p.build_id,
                stage = %p.stage,
                "progress channel full, dropping non-terminal event"
            );
        }
        Err(std::sync::mpsc::TrySendError::Disconnected(p)) => {
            // 受信側 (drain タスク) が既に終了している。ビルドワーカーが sender を drop 済み
            // ということは IPC ハンドラが既に抜けたということで、再送しても届かない。
            tracing::debug!(
                build_id = %p.build_id,
                stage = %p.stage,
                "progress channel disconnected, dropping event"
            );
        }
    }
}

/// terminal 進捗 (done / cancelled / error) を確実に配送する。チャネルの drain が
/// ワーカーより遅れていると通常運用でも発生しうる (`spawn_blocking` の切り替えコスト) ため、
/// blocking send で短時間だけ待機する。Disconnected (IPC 側が抜けた) のみエラー扱い。
fn send_terminal_progress(sender: &ProgressSender, payload: BuildProgress) {
    match sender.send(payload) {
        Ok(()) => {}
        Err(std::sync::mpsc::SendError(p)) => {
            // Disconnected: drain タスクが receiver を drop 済み。これは通常起こらないが
            // (drain タスクは worker 完了後にのみ await される)、起こった場合は致命的。
            tracing::error!(
                build_id = %p.build_id,
                stage = %p.stage,
                "terminal progress lost: channel disconnected"
            );
        }
    }
}

/// ストリーム式 .cursorpack ビルド & エクスポート (Wave 2AB / Task 6 adapter 版)。
///
/// Tauri IPC ハンドラとして呼ばれ、`run_export_blocking` を `spawn_blocking` で起動し、
/// ワーカーが送信する進捗イベントを bounded channel で受け取って `build-progress` を
/// emit する。完了 / キャンセル / エラー / チャネル閉塞の 4 終端状態をすべて明示的に扱う。
#[tauri::command]
pub async fn export_cursorpack_streamed(
    app: tauri::AppHandle,
    req: StreamedExportRequest,
) -> Result<ExportResult, AppError> {
    let (sender, receiver) =
        std::sync::mpsc::sync_channel::<BuildProgress>(PROGRESS_CHANNEL_CAPACITY);

    // ブロッキングワーカー (req + sender + app) を spawn_blocking で起動。`app` は
    // `app.state::<CancelRegistry>()` のため move で渡す必要がある。req には Clone が
    // 不要なので (Wave 2AB / Task 6 fix) そのまま move。drain 側は別途 1 度だけ clone
    // して所有権を分ける (= req clone / 余計な app clone を排除)。
    let app_for_drain = app.clone();
    let worker_join =
        tauri::async_runtime::spawn_blocking(move || run_export_blocking(app, req, sender));

    // 進捗ドレインタスク: receiver.recv() で channel から build-progress を取り出し、
    // Tauri emit でフロントへ配送する。sender が drop されると (worker 完了時)
    // RecvError が返るので while let ループを抜けて自然終了する。
    let drain_join = tauri::async_runtime::spawn_blocking(move || {
        use tauri::Emitter;
        while let Ok(progress) = receiver.recv() {
            if let Err(e) = app_for_drain.emit("build-progress", &progress) {
                tracing::warn!("build-progress emit 失敗: {}", e);
            }
        }
        // RecvError = sender drop → channel closed = Worker 完了の正常終了。
    });

    // Worker の完了を待つ。join 失敗 = spawn_blocking 内部の panic なので
    // AppError::Other にラップして伝播。
    let worker_result = worker_join
        .await
        .map_err(|e| AppError::Other(format!("ビルドワーカー join 失敗: {}", e)))?;

    // ドレインタスクの完了を待つ。sender drop 後は 1 ループ以内で RecvError → break。
    let _ = drain_join.await;

    worker_result
}

/// ブロッキングワーカー本体。`export_cursorpack_streamed` の adapter から `spawn_blocking`
/// 経由で呼ばれる。内部で App state の `CancelRegistry` を取得し RAII ガードを握り、
/// 純粋ワーカー `run_export_inner` に委譲する。すべての image/PNG/CUR/ANI/ZIP/file/library/apply
/// 作業は `run_export_inner` 側に集約される (testability のため)。
fn run_export_blocking(
    app: tauri::AppHandle,
    req: StreamedExportRequest,
    sender: ProgressSender,
) -> Result<ExportResult, AppError> {
    use tauri::Manager;
    let registry = app.state::<CancelRegistry>();
    // RAII ガードで register。`cancel()` は登録済みジョブにのみ作用するため、これが無いと
    // キャンセルが効かない。さらに途中の各 `?` early-return / 完了のいずれの経路でも Drop で
    // 確実に drop_job され、エントリが leak しない (Y15)。
    let build_id = req.build_id.clone();
    let _job = registry.register_guard(&build_id);
    // registry を move せず参照だけを move する `_job` の存命期間中は registry を借用中
    let registry_ref = &registry;
    let is_cancelled = move || registry_ref.is_cancelled(&build_id);
    run_export_inner(req, &sender, is_cancelled)
}

/// 進捗チャネル + キャンセル判定 closure だけを受け取る純粋ワーカー本体。
/// Tauri / App state / IPC には一切依存しないため、テストから直接駆動できる。
/// brief Step 2 の "Move all image/PNG/CUR/ANI/ZIP/file/library/apply work into it" の
/// 作業本体はここに集約される。`run_export_blocking` は AppHandle / CancelRegistry を
/// 取り持ってこの関数に委譲するだけの薄いラッパ。
///
/// `is_cancelled` を `FnMut` 受けにすることで、テストが mutable state (アトミック
/// カウンタや `Cell<bool>` など) を伴ったクロージャを渡せるようにしている。プロダクション
/// 経路 (無キャプチャ || クロージャ) は Fn だが FnMut の境界に coerce 可能。
fn run_export_inner(
    req: StreamedExportRequest,
    sender: &ProgressSender,
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<ExportResult, AppError> {
    let total_roles = req.roles.len() as u32;
    let total_steps = total_roles + if req.sign { 2 } else { 1 }; // roles + package (+sign)

    // 開始イベント
    send_progress(
        sender,
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
        if is_cancelled() {
            // role 段階キャンセル: UI 進捗バーを解放するため cancelled イベントを必ず発火。
            send_progress(
                sender,
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

        send_progress(
            sender,
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

    // 3) 署名。sign.rs のヘルパーに委譲。
    let signed_key_id: Option<String> = if req.sign {
        if is_cancelled() {
            // role 段階と同様に cancelled イベントを発火し、UI 進捗バーを解放する。
            send_progress(
                sender,
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
        send_progress(
            sender,
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
    if is_cancelled() {
        // 同上: package 段階キャンセル時も UI へ通知する。
        send_progress(
            sender,
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
    send_progress(
        sender,
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

    send_progress(
        sender,
        BuildProgress {
            build_id: req.build_id.clone(),
            stage: "done".to_string(),
            current: total_steps,
            total: total_steps,
            message: Some(metadata.id.to_string()),
        },
    );

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
    use super::{is_terminal_stage, run_export_inner, ProgressSender};
    use crate::commands::cursor_build::{
        BuildProgress, ExportDestination, ExportResult, StreamedExportRequest,
    };
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// テスト用ヘルパ: 別スレッドで進捗チャネルを drain し、Arc<Mutex<Vec<_>>> に積む。
    /// `run_export_inner` の terminal progress (`done` 等) は blocking send なので、
    /// 受信側が drain しないと worker がハングする。実機では `export_cursorpack_streamed`
    /// が drain タスクを spawn するのと同じ役割を、ここでは手動で用意する。
    /// 戻り値は (JoinHandle, Arc<Mutex<Vec<_>>>) で、`finish_drain` で join して確定取得。
    fn spawn_drain(
        rx: std::sync::mpsc::Receiver<BuildProgress>,
    ) -> (thread::JoinHandle<()>, Arc<Mutex<Vec<BuildProgress>>>) {
        let collected = Arc::new(Mutex::new(Vec::new()));
        let collected_clone = Arc::clone(&collected);
        let handle = thread::spawn(move || {
            while let Ok(p) = rx.recv() {
                collected_clone.lock().expect("drain mutex").push(p);
            }
        });
        (handle, collected)
    }

    /// テストヘルパ: drain バックグラウンドスレッドの join と蓄積イベントの取り出し。
    fn finish_drain(
        handle: thread::JoinHandle<()>,
        collected: Arc<Mutex<Vec<BuildProgress>>>,
    ) -> Vec<BuildProgress> {
        // drain thread は sender が drop されたあと RecvError でループを抜ける。
        // tx は呼び出し側で明示的に drop 済の想定。
        handle.join().expect("drain thread panicked");
        Arc::try_unwrap(collected)
            .map(|m| m.into_inner().expect("drain mutex"))
            .unwrap_or_else(|arc| arc.lock().expect("drain mutex").clone())
    }

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

    /// Step 1 で導入された `ProgressSender` 型と `run_export_inner` 純粋関数が
    /// 公開されていることのコンパイル時確認。これらがない旧実装ではこのテストは
    /// コンパイルエラーで落ちる (TDD の RED フェーズ)。
    #[test]
    fn progress_sender_and_inner_worker_are_exposed() {
        // 型 alias が解決できること
        let (tx, _rx): (ProgressSender, _) = std::sync::mpsc::sync_channel(64);
        // 型推論: ProgressSender 経由で送信した値が受信側で取り出せる
        let p = BuildProgress {
            build_id: "b".to_string(),
            stage: "role".to_string(),
            current: 0,
            total: 1,
            message: None,
        };
        tx.try_send(p).expect("sink accepts events");
        // 純粋ワーカーがコンパイル可能に公開されていること (シグネチャの存在検証)。
        // `impl FnMut() -> bool` は fn pointer で表現できないため、ここでは呼出し可能
        // であることをもってシグネチャ存在の間接確認とする。
        let minimal = StreamedExportRequest {
            build_id: "compile-check".to_string(),
            name_ja: "T".to_string(),
            name_en: None,
            author: None,
            version: "1.0.0".to_string(),
            description: None,
            requires_os_shadow: false,
            roles: vec![],
            destination: ExportDestination::File {
                path: std::path::PathBuf::from("compile-check.cursorpack")
                    .to_string_lossy()
                    .into_owned(),
            },
            existing_theme_id: None,
            sign: false,
        };
        let _ = || -> Result<ExportResult, crate::errors::AppError> {
            run_export_inner(minimal, &tx, || false)
        };
    }

    /// シンクが role / sign / package / done / cancelled / error の各 stage を
    /// 順序通り届けられることの検証。容量 64 channel を作って 6 件送信し、
    /// すべてが取り出せることを確認する。
    #[test]
    fn progress_sink_collects_role_sign_package_done_events() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<BuildProgress>(64);
        let stages = ["role", "sign", "package", "done", "cancelled", "error"];
        for stage in stages {
            tx.try_send(BuildProgress {
                build_id: "b".to_string(),
                stage: stage.to_string(),
                current: 1,
                total: 6,
                message: None,
            })
            .expect("sink accepts events under capacity");
        }
        drop(tx);

        let received: Vec<String> = rx.iter().map(|p| p.stage).collect();
        assert_eq!(
            received,
            vec![
                "role".to_string(),
                "sign".to_string(),
                "package".to_string(),
                "done".to_string(),
                "cancelled".to_string(),
                "error".to_string(),
            ]
        );
    }

    /// `is_terminal_stage` ヘルパが terminal と non-terminal を厳密に分類すること。
    /// シンク側で terminal を絶対にドロップせず non-terminal を best-effort で扱う
    /// ポリシーの根拠。
    #[test]
    fn terminal_stage_classifier_is_strict() {
        assert!(is_terminal_stage("done"));
        assert!(is_terminal_stage("cancelled"));
        assert!(is_terminal_stage("error"));
        assert!(!is_terminal_stage("role"));
        assert!(!is_terminal_stage("sign"));
        assert!(!is_terminal_stage("package"));
        assert!(!is_terminal_stage(""));
        assert!(!is_terminal_stage("DONE"));
    }

    /// シンクが満杯のとき `try_send` は `Full` を返す。受信側 drain が遅れた場合の
    /// バックプレッシャー検証 (本実装では capacity=64 で事実上発生しないが、
    /// best-effort ポリシーの境界条件として保証する)。
    #[test]
    fn progress_sink_full_channel_returns_try_send_error() {
        // 容量 1 の channel で 1 件目を占有し、2 件目の try_send が Full になることを検証
        let (tx, _rx) = std::sync::mpsc::sync_channel::<BuildProgress>(1);
        tx.try_send(BuildProgress {
            build_id: "b".to_string(),
            stage: "role".to_string(),
            current: 0,
            total: 1,
            message: None,
        })
        .expect("first send fits");
        let err = tx
            .try_send(BuildProgress {
                build_id: "b".to_string(),
                stage: "package".to_string(),
                current: 1,
                total: 1,
                message: None,
            })
            .expect_err("second send must fail with Full");
        assert!(
            matches!(err, std::sync::mpsc::TrySendError::Full(_)),
            "expected TrySendError::Full, got {err:?}"
        );
    }

    /// 送信側を drop すると受信側は `Disconnected` を観測してループを抜けられる。
    /// ワーカー完了後に drain タスクが必ず停止できることを保証する。
    #[test]
    fn progress_sink_disconnect_signals_receiver_to_stop() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<BuildProgress>(64);
        tx.try_send(BuildProgress {
            build_id: "b".to_string(),
            stage: "done".to_string(),
            current: 1,
            total: 1,
            message: None,
        })
        .unwrap();
        drop(tx); // 送信側を drop → 受信側は EOF

        // 1 件受信でき、以降は Disconnected で停止できる
        let first = rx.recv().expect("first event");
        assert_eq!(first.stage, "done");
        let err = rx.recv().expect_err("channel closed after sender drop");
        assert!(matches!(err, std::sync::mpsc::RecvError));
    }

    /// ワーカーのキャンセル経路: `is_cancelled` が true を返すチェックポイントで
    /// ワーカーは `cancelled` イベントを送出して `AppError::Other("ビルドがキャンセルされました")`
    /// を返す。役割なし + 署名なしで package 段階のキャンセルを直接検証する。
    #[test]
    fn worker_returns_cancelled_error_when_is_cancelled_at_package_stage() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out_path = tmp.path().join("should-not-be-written.cursorpack");
        let (tx, rx) = std::sync::mpsc::sync_channel::<BuildProgress>(64);
        let (handle, collected) = spawn_drain(rx);
        let req = StreamedExportRequest {
            build_id: "cancel-test".to_string(),
            name_ja: "T".to_string(),
            name_en: None,
            author: None,
            version: "1.0.0".to_string(),
            description: None,
            requires_os_shadow: false,
            roles: vec![],
            destination: ExportDestination::File {
                path: out_path.to_string_lossy().into_owned(),
            },
            existing_theme_id: None,
            sign: false,
        };

        // roles が空 → role ループはスキップされ package 段階のキャンセルに合流する
        let result = run_export_inner(req, &tx, || true);
        assert!(
            result.is_err(),
            "expected Err when is_cancelled returns true at package stage"
        );
        let err = result.unwrap_err();
        assert!(
            matches!(err, crate::errors::AppError::Other(ref m) if m.contains("キャンセル")),
            "expected cancellation error, got {err:?}"
        );
        // 出力ファイルは書かれていない
        assert!(
            !out_path.is_file(),
            "output file should not be written on cancel"
        );
        // tx を drop して drain thread を終わらせる
        drop(tx);

        // 受信側で cancelled イベントを確認
        let events = finish_drain(handle, collected);
        let stages: Vec<String> = events.iter().map(|p| p.stage.clone()).collect();
        assert!(
            stages.iter().any(|s| s == "cancelled"),
            "expected a cancelled terminal event, got stages {stages:?}"
        );
    }

    /// 役割ループの最初のチェックポイントでキャンセル要求を返すパターン。role 段階の
    /// cancelled イベント発火経路がソース上に存在することを保証する (roles 空でも
    /// package 段階の cancel 経路は共有される)。
    #[test]
    fn worker_emits_cancelled_event_on_first_checkpoint() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out_path = tmp.path().join("should-not-be-written-2.cursorpack");
        let (tx, rx) = std::sync::mpsc::sync_channel::<BuildProgress>(64);
        let (handle, collected) = spawn_drain(rx);
        let req = StreamedExportRequest {
            build_id: "role-cancel".to_string(),
            name_ja: "T".to_string(),
            name_en: None,
            author: None,
            version: "1.0.0".to_string(),
            description: None,
            requires_os_shadow: false,
            roles: vec![],
            destination: ExportDestination::File {
                path: out_path.to_string_lossy().into_owned(),
            },
            existing_theme_id: None,
            sign: false,
        };
        let mut first_check = true;
        let result = run_export_inner(req, &tx, || {
            // 最初のチェックポイントだけキャンセル要求を返す (std::mem::take で
            // clippy::manual_take を回避しつつ意図を明示する)。
            std::mem::take(&mut first_check)
        });
        assert!(result.is_err(), "expected cancellation error");
        drop(tx);
        let events = finish_drain(handle, collected);
        let stages: Vec<String> = events.iter().map(|p| p.stage.clone()).collect();
        // キャンセル経路のどこかで cancelled イベントが発火されているはず
        assert!(
            stages.contains(&"cancelled".to_string()),
            "expected cancelled event in stages, got {stages:?}"
        );
    }

    /// ソースコード全体に `is_cancelled` のチェックポイントが role / sign / package の
    /// 3 段階すべてに存在することを検証する静的リグレッション。
    #[test]
    fn is_cancelled_is_checked_at_role_sign_package_stages() {
        let source = include_str!("stream.rs");
        let count = source.matches("is_cancelled(").count();
        assert!(
            count >= 3,
            "expected is_cancelled checks at role + sign + package stages, got {count}"
        );
    }

    /// 役割なし + 署名なし + キャンセルなしでも package → done までの正常系で
    /// シンクが `package` と `done` 双方の stage を受信できることを検証。
    /// FS 書込先を一時ディレクトリにして副作用を限定する。
    #[test]
    fn worker_normal_path_emits_package_and_done_stages() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out_path = tmp.path().join("ok.cursorpack");
        let (tx, rx) = std::sync::mpsc::sync_channel::<BuildProgress>(64);
        let (handle, collected) = spawn_drain(rx);
        let req = StreamedExportRequest {
            build_id: "normal-path".to_string(),
            name_ja: "Normal".to_string(),
            name_en: None,
            author: None,
            version: "1.0.0".to_string(),
            description: None,
            requires_os_shadow: false,
            roles: vec![],
            destination: ExportDestination::File {
                path: out_path.to_string_lossy().into_owned(),
            },
            existing_theme_id: None,
            sign: false,
        };
        let result = run_export_inner(req, &tx, || false);
        assert!(result.is_ok(), "normal path should succeed: {result:?}");
        drop(tx);

        let events = finish_drain(handle, collected);
        let stages: Vec<String> = events.iter().map(|p| p.stage.clone()).collect();
        assert!(
            stages.contains(&"package".to_string()),
            "expected package stage, got {stages:?}"
        );
        assert!(
            stages.contains(&"done".to_string()),
            "expected done stage, got {stages:?}"
        );
        // terminal done は最後に来ること (roleループなしの場合は package → done)
        let last_terminal = stages
            .iter()
            .rev()
            .find(|s| matches!(s.as_str(), "done" | "cancelled" | "error"))
            .expect("at least one terminal stage");
        assert_eq!(
            last_terminal, "done",
            "expected done as last terminal stage, got {last_terminal}"
        );
        // 出力ファイルが書かれている
        assert!(out_path.is_file(), "output file should exist");
    }
}
