//! 同梱用 9 つの公式テーマを `sample-cursor/ecs-*/` の PNG から
//! `sample-cursor/*.cursorpack` に一括書き出しする CLI。
//!
//! 用途:
//!   - Mint テーマ (~/.custom_cursors/6d364941-…/theme.json) と同じ 17 ロールの
//!     ホットスポット比率・命名規則 ("EasyCursorSwap <Title>") を踏襲して、
//!     公式テーマ群を再生成する。
//!
//! 使い方:
//!   cargo run --manifest-path src-tauri/Cargo.toml --bin build_sample_packs

use app_lib::cursor::{build_cur_from_png, ResizeMethod};
use app_lib::theme::types::{CursorDefinition, Hotspot, LocalizedString, Ratio01, ThemeMetadata};
use app_lib::theme::ThemeManager;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use uuid::Uuid;

/// 素材 PNG の一辺長さ (px)。`sample-cursor/ecs-*/` の PNG はすべて 128×128。
const SAMPLE_SIZE: u32 = 128;

/// (Role, hotspot_x, hotspot_y) - Mint テーマと完全一致させる 17 ロール定義。
const HOTSPOTS: &[(&str, f32, f32)] = &[
    ("Person", 0.5, 0.5),
    ("Help", 0.12283384, 0.13111621),
    ("Pin", 0.5101937, 0.88035166),
    ("UpArrow", 0.5254842, 0.075050965),
    ("Crosshair", 0.5, 0.5),
    ("No", 0.5, 0.5),
    ("SizeNS", 0.5, 0.5),
    ("SizeAll", 0.5, 0.5),
    ("SizeNWSE", 0.5, 0.5),
    ("Arrow", 0.12283384, 0.11837411),
    ("NWPen", 0.10754332, 0.88035166),
    ("IBeam", 0.5, 0.5),
    ("SizeWE", 0.5, 0.5),
    ("AppStarting", 0.117737, 0.110728845),
    ("Wait", 0.5, 0.5),
    ("SizeNESW", 0.5, 0.5),
    ("Hand", 0.3216106, 0.14130989),
];

/// (folder_name, display_title, file_prefix)。
///
/// - folder_name: `sample-cursor/<folder>/` の名前 + 出力 `.cursorpack` の stem
/// - display_title: theme.json の `name` に入る文字列。"EasyCursorSwap <title>" になる
/// - file_prefix: 各 PNG ファイル名のプレフィックス (`<prefix>__<Role>.png`)
const THEMES: &[(&str, &str, &str)] = &[
    ("ecs-carbon", "Carbon", "carbon"),
    ("ecs-coral-sunset", "Coral Sunset", "coral-sunset"),
    ("ecs-mono-ghost", "Mono Ghost", "mono-ghost"),
    ("ecs-neon-wire", "Neon Wire", "neon-wire"),
    ("ecs-paper-white", "Paper White", "paper-white"),
    ("ecs-sakura", "Sakura", "sakura"),
    ("ecs-solid-ink", "Solid Ink", "solid-ink"),
    ("ecs-terminal-green", "Terminal Green", "terminal-green"),
    ("ecs-violet-dusk", "Violet Dusk", "violet-dusk"),
];

fn localized_name(title: &str) -> LocalizedString {
    let full = format!("EasyCursorSwap {}", title);
    let mut m = HashMap::new();
    m.insert("ja".into(), full.clone());
    m.insert("en".into(), full);
    LocalizedString::Localized(m)
}

fn build_one(
    sample_root: &Path,
    output_dir: &Path,
    folder: &str,
    title: &str,
    prefix: &str,
) -> Result<(PathBuf, u64, Uuid), String> {
    let src_dir = sample_root.join(folder);
    if !src_dir.is_dir() {
        return Err(format!("{} が存在しません", src_dir.display()));
    }

    let mut cursors_meta: HashMap<String, CursorDefinition> = HashMap::new();
    let mut cursor_bytes: HashMap<String, Vec<u8>> = HashMap::new();

    for (role, hx, hy) in HOTSPOTS {
        let png_path = src_dir.join(format!("{}__{}.png", prefix, role));
        let png_bytes = std::fs::read(&png_path)
            .map_err(|e| format!("{} の読込失敗: {}", png_path.display(), e))?;

        let hotspot_norm = Hotspot {
            x: Ratio01::new(*hx),
            y: Ratio01::new(*hy),
        };
        let (hx_px, hy_px) = hotspot_norm.to_px(SAMPLE_SIZE);

        let cur_bytes =
            build_cur_from_png(&png_bytes, hx_px, hy_px, ResizeMethod::Lanczos, None, None)
                .map_err(|e| format!("{}::{} の .cur 生成失敗: {}", folder, role, e))?;

        cursors_meta.insert(
            role.to_string(),
            CursorDefinition {
                file: format!("cursors/{}.cur", role),
                hotspot: hotspot_norm,
                resize_method: "lanczos".to_string(),
                size_overrides: None,
            },
        );
        cursor_bytes.insert(role.to_string(), cur_bytes);
    }

    let id = Uuid::new_v4();
    let mut metadata = ThemeMetadata {
        schema_version: 2,
        id,
        name: localized_name(title),
        version: "1.0.0".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        requires_os_shadow: false,
        cursors: cursors_meta,
        author: Some("nishiuriraku".to_string()),
        license: None,
        homepage: None,
        description: None,
        min_app_version: None,
        signature: None,
        tags: Vec::new(),
    };

    let output_path = output_dir.join(format!("{}.cursorpack", folder));
    let size = ThemeManager::export_cursorpack(&mut metadata, &cursor_bytes, &output_path)
        .map_err(|e| format!("{} の cursorpack 書出失敗: {}", folder, e))?;

    Ok((output_path, size, id))
}

fn main() -> ExitCode {
    // CARGO_MANIFEST_DIR は src-tauri/。リポジトリルートはその親。
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let repo_root = PathBuf::from(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let sample_root = repo_root.join("sample-cursor");
    let output_dir = sample_root.clone();

    if !sample_root.is_dir() {
        eprintln!("error: {} が見つかりません", sample_root.display());
        return ExitCode::FAILURE;
    }

    let mut failed = 0usize;
    let mut total_size = 0u64;
    for (folder, title, prefix) in THEMES {
        match build_one(&sample_root, &output_dir, folder, title, prefix) {
            Ok((path, size, id)) => {
                total_size += size;
                println!(
                    "[ok] EasyCursorSwap {:<16} {:>8} bytes  id={}  → {}",
                    title,
                    size,
                    id,
                    path.display()
                );
            }
            Err(e) => {
                eprintln!("[err] {}: {}", folder, e);
                failed += 1;
            }
        }
    }

    println!(
        "\n{} themes built, {} bytes total ({} failed)",
        THEMES.len() - failed,
        total_size,
        failed
    );

    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
