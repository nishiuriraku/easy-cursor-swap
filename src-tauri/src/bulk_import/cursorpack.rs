//! `.cursorpack` を Creator 用に解凍してロール毎に PNG プレビューを抽出する。
//!
//! 通常の `import_cursorpack` は `~/.custom_cursors/<UUID>/` に展開してライブラリ入りさせるが、
//! Creator の「既存パックを取り込んで編集」フローではディスク書き込みせず
//! メモリ上で PNG バイトを取り出す必要がある。本モジュールはその専用パイプライン。

use super::{BulkImportProgress, ParseCursorpackRequest, ParsedCursorpack, ParsedRole};
use crate::config::{
    DEFAULT_MAX_IMAGE_FILE_SIZE, DEFAULT_MAX_PACK_COMPRESSED_SIZE,
    DEFAULT_MAX_PACK_UNCOMPRESSED_SIZE,
};
use crate::errors::AppError;
use crate::theme::types::AniFrameData;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use tauri::{AppHandle, Emitter};
use zip::ZipArchive;

use super::CursorpackMetadata;

/// `.cursorpack` の theme.json から CursorpackMetadata を組み立てる。
fn metadata_from_theme(meta: &crate::theme::ThemeMetadata) -> CursorpackMetadata {
    use crate::theme::LocalizedString;
    let name_ja = match &meta.name {
        LocalizedString::Simple(s) => Some(s.clone()),
        LocalizedString::Localized(m) => m.get("ja").or_else(|| m.get("default")).cloned(),
    };
    let name_en = match &meta.name {
        LocalizedString::Simple(_) => None,
        LocalizedString::Localized(m) => m.get("en").cloned(),
    };
    let description = meta.description.as_ref().and_then(|d| match d {
        LocalizedString::Simple(s) => Some(s.clone()),
        LocalizedString::Localized(m) => m
            .get("ja")
            .or_else(|| m.get("en"))
            .or_else(|| m.get("default"))
            .cloned(),
    });
    CursorpackMetadata {
        id: Some(meta.id.to_string()),
        name_ja,
        name_en,
        author: meta.author.clone(),
        version: Some(meta.version.clone()),
        description,
    }
}

/// `ParsedIcoCurEntry.image` を PNG バイト列にエンコード。
fn encode_entry_to_png(entry: &crate::cursor::ParsedIcoCurEntry) -> Result<Vec<u8>, AppError> {
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        entry.image.as_raw(),
        entry.image.width(),
        entry.image.height(),
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| AppError::ImageProcessing(format!("PNG エンコード失敗: {}", e)))?;
    Ok(buf)
}

/// `parse_ani_role` の戻り値。`.ani` ロールから抽出した primary プレビューと
/// フレーム列、(あれば) 展開先絶対パスを束ねたもの。
struct AniRoleParseResult {
    primary_size: u32,
    primary_png: Vec<u8>,
    hotspot: crate::theme::types::Hotspot,
    ani: AniFrameData,
    ani_source_path: Option<String>,
}

/// `.ani` ロールを解析して `AniRoleParseResult` を返す。
/// `ani_extract_dir` が指定されていればロールの元バイトを `<dir>/<role-filename>` に
/// 書き出し、その絶対パスを返す (export 時の rewrite_ani_with_hotspot ソースに使う)。
fn parse_ani_role(
    role_id: &str,
    file_in_zip: &str,
    bytes: &[u8],
    ani_extract_dir: Option<&Path>,
) -> Result<AniRoleParseResult, AppError> {
    let parsed = crate::cursor::parse_ani(bytes)?;
    let frame0 = parsed.frames.first().ok_or_else(|| {
        AppError::ImageProcessing(format!("ロール {} の .ani にフレームがありません", role_id))
    })?;
    let primary_png = {
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        image::ImageEncoder::write_image(
            encoder,
            frame0.image.as_raw(),
            frame0.image.width(),
            frame0.image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| AppError::ImageProcessing(format!("PNG エンコード失敗: {}", e)))?;
        buf
    };

    // 全フレームを PNG 化
    let mut frame_pngs: Vec<Vec<u8>> = Vec::with_capacity(parsed.frames.len());
    for f in &parsed.frames {
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        if image::ImageEncoder::write_image(
            encoder,
            f.image.as_raw(),
            f.image.width(),
            f.image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .is_err()
        {
            continue;
        }
        frame_pngs.push(buf);
    }
    let per_step_durations_ms: Vec<u32> = parsed
        .per_step_rate_jiffies
        .iter()
        .map(|j| ((*j as u64 * 1000) / 60) as u32)
        .collect();

    // export 用にバイトを展開
    let ani_source_path = if let Some(dir) = ani_extract_dir {
        std::fs::create_dir_all(dir)?;
        // file_in_zip は ZIP エントリ名で外部由来 → `sanitize_archive_path_pub`
        // 経由でパストラバーサルや絶対パスを拒否し、basename だけを取り出す
        // (CLAUDE.md "Archive sanitisation" 不変条件の文言と完全一致)。
        let sanitized = crate::theme::sanitize_archive_path_pub(file_in_zip)?;
        let fname = sanitized
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("{}.ani", role_id));
        let out_path = dir.join(&fname);
        std::fs::write(&out_path, bytes)?;
        Some(out_path.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(AniRoleParseResult {
        primary_size: frame0.image.width(),
        primary_png,
        hotspot: crate::theme::types::Hotspot::from_px(
            frame0.hotspot_x,
            frame0.hotspot_y,
            frame0.image.width(),
        ),
        ani: AniFrameData {
            frame_pngs,
            sequence: parsed.sequence,
            per_step_durations_ms,
            is_legacy_raw_dib: parsed.is_legacy_raw_dib,
        },
        ani_source_path,
    })
}

pub fn parse_cursorpack_inner(bytes: &[u8]) -> Result<ParsedCursorpack, AppError> {
    parse_cursorpack_inner_with_extract(bytes, None)
}

/// ZIP エントリを「申告サイズに依存せず」実バイト上限まで読み込むヘルパー。
///
/// in-memory パイプライン (Creator 取り込み) なので `take(上限 +1)` で打ち切り、
/// 実伸長サイズが個別上限を超えたら Err。size ガードは `parse_ico_cur` 等の
/// フォーマット解析より前に効かせるため、読込点ごとにこれを通す。
/// `label` はエラーメッセージ用 (例: "ロール Arrow のファイル size.cur")。
fn read_entry_capped<R: std::io::Read>(entry: &mut R, label: &str) -> Result<Vec<u8>, AppError> {
    let mut buf = Vec::new();
    entry
        .by_ref()
        .take(DEFAULT_MAX_IMAGE_FILE_SIZE + 1)
        .read_to_end(&mut buf)
        .map_err(|e| AppError::InvalidCursorpack {
            reason: format!("{} の読み込みに失敗: {}", label, e),
        })?;
    if buf.len() as u64 > DEFAULT_MAX_IMAGE_FILE_SIZE {
        return Err(AppError::InvalidCursorpack {
            reason: format!(
                "{} の実サイズが上限 {} bytes を超えています",
                label, DEFAULT_MAX_IMAGE_FILE_SIZE
            ),
        });
    }
    Ok(buf)
}

/// `parse_cursorpack_inner` の `.ani` 展開先指定版。
/// `ani_extract_dir` を渡すと、`.ani` ロールのバイトをそこに書き出して
/// 各 ParsedRole の `ani_source_path` に絶対パスを格納する。
pub fn parse_cursorpack_inner_with_extract(
    bytes: &[u8],
    ani_extract_dir: Option<&Path>,
) -> Result<ParsedCursorpack, AppError> {
    // 1) 圧縮サイズ上限 (zip 爆弾の入口防御)。フォーマット解析より前に置く。
    if bytes.len() as u64 > DEFAULT_MAX_PACK_COMPRESSED_SIZE {
        return Err(AppError::InvalidCursorpack {
            reason: format!(
                ".cursorpack 圧縮サイズ {} bytes が上限 {} を超えています",
                bytes.len(),
                DEFAULT_MAX_PACK_COMPRESSED_SIZE
            ),
        });
    }

    let cursor = std::io::Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| AppError::InvalidCursorpack {
        reason: format!("ZIP オープン失敗: {}", e),
    })?;

    // 累積展開サイズ (zip 爆弾の最終防衛線)。実読込バイト数で加算する。
    let mut total: u64 = 0;

    // theme.json を読む
    let theme: crate::theme::ThemeMetadata = {
        let mut entry = archive
            .by_name("theme.json")
            .map_err(|_| AppError::InvalidCursorpack {
                reason: "theme.json が見つかりません".to_string(),
            })?;
        // theme.json も個別上限の対象 (実バイトで打ち切り)。
        let raw = read_entry_capped(&mut entry, "theme.json")?;
        total = total.saturating_add(raw.len() as u64);
        let buf = String::from_utf8(raw).map_err(|e| AppError::InvalidCursorpack {
            reason: format!("theme.json が UTF-8 ではありません: {}", e),
        })?;
        serde_json::from_str(&buf).map_err(|e| AppError::InvalidCursorpack {
            reason: format!("theme.json 解析失敗: {}", e),
        })?
    };

    let metadata = metadata_from_theme(&theme);

    // 各ロールを抽出
    let mut roles: HashMap<String, ParsedRole> = HashMap::new();
    for (role_id, def) in &theme.cursors {
        // primary ファイルを読む (個別上限を実バイトで打ち切り、累積へ加算)
        let primary_bytes = {
            let mut entry =
                archive
                    .by_name(&def.file)
                    .map_err(|_| AppError::InvalidCursorpack {
                        reason: format!(
                            "ロール {} のファイル {} が ZIP 内にありません",
                            role_id, def.file
                        ),
                    })?;
            let buf = read_entry_capped(
                &mut entry,
                &format!("ロール {} のファイル {}", role_id, def.file),
            )?;
            total = total.saturating_add(buf.len() as u64);
            if total > DEFAULT_MAX_PACK_UNCOMPRESSED_SIZE {
                return Err(AppError::InvalidCursorpack {
                    reason: format!(
                        "展開後合計サイズが上限 {} bytes を超えました",
                        DEFAULT_MAX_PACK_UNCOMPRESSED_SIZE
                    ),
                });
            }
            buf
        };

        // 拡張子で `.ani` を判定。.cur / .ico は従来通り parse_ico_cur に流す。
        let is_ani = Path::new(&def.file)
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| e.eq_ignore_ascii_case("ani"))
            .unwrap_or(false);

        if is_ani {
            let r = parse_ani_role(role_id, &def.file, &primary_bytes, ani_extract_dir)?;
            // .ani には sized オーバーライドの概念がないので空 HashMap
            roles.insert(
                role_id.clone(),
                ParsedRole {
                    asset: crate::theme::types::CursorAssetDescriptor {
                        png_bytes: r.primary_png,
                        width: r.primary_size,
                        height: r.primary_size,
                        hotspot: r.hotspot,
                    },
                    sized_png_bytes: HashMap::new(),
                    ani: Some(r.ani),
                    ani_source_path: r.ani_source_path,
                },
            );
            continue;
        }

        let parsed = crate::cursor::parse_ico_cur(&primary_bytes)?;
        let (largest, primary_png) = crate::cursor::pick_largest_as_png(&parsed)?;

        // primary 内の各解像度を sized_png_bytes に詰める
        let mut sized: HashMap<u32, Vec<u8>> = HashMap::new();
        for entry in &parsed.entries {
            if let Ok(png) = encode_entry_to_png(entry) {
                sized.insert(entry.width, png);
            }
        }

        // size_overrides の各解像度も追加で読む
        if let Some(overrides) = &def.size_overrides {
            for (size_str, ov) in overrides {
                if let Ok(size) = size_str.parse::<u32>() {
                    let mut entry = match archive.by_name(&ov.file) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    // サイズ超過は握り潰さず Err (攻撃を素通りさせない)。
                    // エントリ欠落・破損は従来どおり continue でスキップ。
                    let buf = read_entry_capped(
                        &mut entry,
                        &format!("ロール {} の size_override {}", role_id, ov.file),
                    )?;
                    total = total.saturating_add(buf.len() as u64);
                    if total > DEFAULT_MAX_PACK_UNCOMPRESSED_SIZE {
                        return Err(AppError::InvalidCursorpack {
                            reason: format!(
                                "展開後合計サイズが上限 {} bytes を超えました",
                                DEFAULT_MAX_PACK_UNCOMPRESSED_SIZE
                            ),
                        });
                    }
                    if let Ok(parsed_ov) = crate::cursor::parse_ico_cur(&buf) {
                        if let Some(matching) = parsed_ov.entries.iter().find(|e| e.width == size) {
                            if let Ok(png) = encode_entry_to_png(matching) {
                                sized.insert(size, png);
                            }
                        }
                    }
                }
            }
        }

        roles.insert(
            role_id.clone(),
            ParsedRole {
                asset: crate::theme::types::CursorAssetDescriptor {
                    png_bytes: primary_png,
                    width: largest.width,
                    height: largest.height,
                    hotspot: def.hotspot,
                },
                sized_png_bytes: sized,
                ani: None,
                ani_source_path: None,
            },
        );
    }

    Ok(ParsedCursorpack { metadata, roles })
}

#[tauri::command]
pub async fn parse_cursorpack_for_creator(
    app: AppHandle,
    req: ParseCursorpackRequest,
) -> Result<ParsedCursorpack, AppError> {
    let job_id = req.job_id.clone();
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _ = app_clone.emit(
            "bulk-import-progress",
            BulkImportProgress {
                job_id: job_id.clone(),
                stage: "extract",
                current: 0,
                total: 1,
                message: None,
            },
        );
        let bytes = std::fs::read(&req.path).map_err(|e| AppError::InvalidCursorpack {
            reason: format!("読み込み失敗: {}", e),
        })?;
        // 圧縮上限の二重チェック (parse_cursorpack_inner_with_extract 側でも検査するが、
        // ファイル経路でも早期に弾いておく。二重でも害なし)。
        if bytes.len() as u64 > DEFAULT_MAX_PACK_COMPRESSED_SIZE {
            return Err(AppError::InvalidCursorpack {
                reason: format!(
                    ".cursorpack 圧縮サイズ {} bytes が上限 {} を超えています",
                    bytes.len(),
                    DEFAULT_MAX_PACK_COMPRESSED_SIZE
                ),
            });
        }
        // `.ani` ロールのバイトは export 時に rewrite_ani_with_hotspot で再利用するため、
        // `<cursorpack>.extracted/` に書き出してパスを ParsedRole.ani_source_path に格納する。
        // ディレクトリは cursorpack と同じ寿命 (一時テーマ複製では tempDir() 配下) なので
        // 通常のテーマ保存までは生きている。
        let extract_dir = {
            let p = Path::new(&req.path);
            let suffix = format!(
                "{}.extracted",
                p.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("cursorpack")
            );
            p.parent().map(|d| d.join(suffix))
        };
        let r = parse_cursorpack_inner_with_extract(&bytes, extract_dir.as_deref())?;
        let _ = app_clone.emit(
            "bulk-import-progress",
            BulkImportProgress {
                job_id,
                stage: "done",
                current: 1,
                total: 1,
                message: None,
            },
        );
        Ok(r)
    })
    .await
    .map_err(|e| AppError::InvalidCursorpack {
        reason: format!("join 失敗: {}", e),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../sample-icon");
        p
    }

    #[test]
    fn parse_cursorpack_basic_returns_roles() {
        let mut p = fixture_dir();
        p.push("easy-cursor-swap-mint.cursorpack");
        if !p.is_file() {
            eprintln!("skipping: cursorpack fixture not present");
            return;
        }
        let bytes = std::fs::read(&p).expect("fixture must exist");
        let parsed = parse_cursorpack_inner(&bytes).expect("parse should succeed");
        assert!(!parsed.roles.is_empty(), "should extract at least 1 role");
        assert!(
            parsed.roles.contains_key("Arrow"),
            "Arrow role should be present"
        );
        // creator.vue ?editPath 経路で sourceThemeId にそのまま代入されるため、
        // metadata.id が必ず UUID 文字列で返ることを契約として固定する。
        // 過去にここが欠落していた結果、SaveDestinationModal の「上書き / 複製」
        // セクションが永久に出ないバグになっていた。
        let id = parsed
            .metadata
            .id
            .as_deref()
            .expect("metadata.id must be Some");
        assert!(
            uuid::Uuid::parse_str(id).is_ok(),
            "metadata.id should be a parseable UUID, got {id:?}"
        );
    }

    // ── F-11: parse_cursorpack_inner サイズガード回帰テスト ─────────────
    //
    // parse_cursorpack_inner_with_extract の検査順序:
    //   圧縮上限 → ZipArchive::new → theme.json (read_entry_capped)→
    //   ロール毎に primary を read_entry_capped (個別+累積)→ parse_ico_cur。
    // 以前はこの 3 段ガードが皆無で、巨大 .cursorpack を無防備にメモリ展開していた。

    use crate::theme::types::{CursorDefinition, ThemeMetadata};
    use crate::theme::LocalizedString;
    use std::collections::HashMap;

    /// 有効な theme.json + 任意の追加エントリで .cursorpack バイト列を作る。
    /// `cursors` に role→file を指定すると theme.json の cursors に反映される。
    /// `add_entries` クロージャで悪性ロールファイルなどを追加する。
    fn build_pack(
        cursors: HashMap<String, String>,
        add_entries: impl FnOnce(&mut zip::ZipWriter<std::io::Cursor<&mut Vec<u8>>>),
    ) -> Vec<u8> {
        use std::io::Write;

        let cursor_defs: HashMap<String, CursorDefinition> = cursors
            .into_iter()
            .map(|(role, file)| {
                (
                    role,
                    CursorDefinition {
                        file,
                        hotspot: crate::theme::types::Hotspot::ZERO,
                        resize_method: "lanczos".to_string(),
                        size_overrides: None,
                    },
                )
            })
            .collect();

        let metadata = ThemeMetadata {
            schema_version: 1,
            id: uuid::Uuid::new_v4(),
            name: LocalizedString::Simple("Guard Test Pack".into()),
            version: "1.0.0".into(),
            created_at: "2026-06-10T00:00:00Z".into(),
            requires_os_shadow: false,
            cursors: cursor_defs,
            author: None,
            license: None,
            homepage: None,
            description: None,
            min_app_version: None,
            signature: None,
            tags: Vec::new(),
            source: crate::theme::types::ThemeSource::Local,
            cloned_from_marketplace_id: None,
        };
        let metadata_json = serde_json::to_vec_pretty(&metadata).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(cursor);
            let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("theme.json", opts).unwrap();
            zip.write_all(&metadata_json).unwrap();
            add_entries(&mut zip);
            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn parse_rejects_oversized_compressed_pack() {
        // 圧縮上限超: ZipArchive 解析より前に弾く。中身は ZIP ですらなくてよい。
        let oversized = vec![0u8; (DEFAULT_MAX_PACK_COMPRESSED_SIZE + 1) as usize];
        let result = parse_cursorpack_inner(&oversized);
        assert!(
            result.is_err(),
            "圧縮上限を超える .cursorpack は拒否されるべき"
        );
    }

    #[test]
    fn parse_rejects_oversized_role_file() {
        // 個別ファイル上限超: theme.json の Arrow.file が指す 11MB ゼロ列を同梱。
        // read_entry_capped が parse_ico_cur より前に実バイトで弾く。
        let mut cursors = HashMap::new();
        cursors.insert("Arrow".to_string(), "cursors/huge.cur".to_string());
        let payload = vec![0u8; (DEFAULT_MAX_IMAGE_FILE_SIZE + 1) as usize];
        let bytes = build_pack(cursors, |zip| {
            use std::io::Write;
            let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("cursors/huge.cur", opts).unwrap();
            zip.write_all(&payload).unwrap();
        });

        let result = parse_cursorpack_inner(&bytes);
        assert!(
            result.is_err(),
            "個別ファイル上限を超えるロールファイルは拒否されるべき"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("サイズ") && msg.contains("上限"),
            "サイズ上限超過系のエラーメッセージであるべき: {msg}"
        );
    }

    #[test]
    fn parse_accepts_pack_within_limits() {
        // 正常系の対照: 上限内の小さなロールファイルなら圧縮/個別ガードは通過する
        // (parse_ico_cur 段で不正フォーマットとして弾かれることはあっても、
        //  サイズガードでは弾かれないことを確認する)。
        let mut cursors = HashMap::new();
        cursors.insert("Arrow".to_string(), "cursors/tiny.cur".to_string());
        let bytes = build_pack(cursors, |zip| {
            use std::io::Write;
            let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("cursors/tiny.cur", opts).unwrap();
            zip.write_all(b"not a real cur but small").unwrap();
        });

        let result = parse_cursorpack_inner(&bytes);
        // サイズガードでは弾かれない: もし Err でもサイズ系メッセージではないこと。
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                !(msg.contains("圧縮サイズ") || msg.contains("実サイズが上限")),
                "上限内なのにサイズガードで弾かれてはいけない: {msg}"
            );
        }
    }
}
