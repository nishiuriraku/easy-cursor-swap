//! 実 `.ani` を Apply パイプラインで HKCU に書き込み、即座にロールバックして
//! Apply 動作 (rewrite → registry 書込 → SPI_SETCURSORS → 復元) を一気通貫で検証する CLI。
//!
//! 用途:
//!   - sample-cursor 配下の旧/新形式 `.ani` で「`HKCU\Control Panel\Cursors\Arrow`
//!     に値が書かれ、SPI_SETCURSORS が成功する」ことを目視確認の前段として自動検証する。
//!   - 必ず Arrow の元の値を退避してから書き換え、終了前 (panic 時も Drop で) に
//!     復元する。途中で kill された場合は `~/.custom_cursors/_pending_apply.snapshot`
//!     が残るので、次回 EasyCursorSwap 起動時に通常のロールバック経路で復旧する。
//!
//! 使い方:
//! ```text
//! cargo run --manifest-path src-tauri/Cargo.toml --bin apply_ani_verify -- <path-to.ani>
//! ```
//!
//! stdout に JSON で結果を 1 行出す。終了コード 0 = 成功。

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use app_lib::cursor::ani::parse_ani;
use app_lib::cursor::ani_write::rewrite_ani_to_path;
use app_lib::registry::RegistryManager;

/// Arrow の元値を保持し、Drop で必ず HKCU に書き戻すガード。
struct ArrowRestoreGuard {
    original: String,
}

impl ArrowRestoreGuard {
    fn snapshot() -> Result<Self, String> {
        let map = RegistryManager::read_current_cursors()
            .map_err(|e| format!("Arrow 値の読み取り失敗: {e:?}"))?;
        let original = map.get("Arrow").cloned().unwrap_or_default();
        Ok(Self { original })
    }
}

impl Drop for ArrowRestoreGuard {
    fn drop(&mut self) {
        // 直接 HKCU\Control Panel\Cursors\Arrow を書き戻し、SPI_SETCURSORS で反映する。
        // RegistryManager::apply_cursors を経由しないのは、ペンディングスナップショット
        // の整合性を二重に取らないようにするため。
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey_with_flags("Control Panel\\Cursors", KEY_WRITE) {
            let _ = key.set_value("Arrow", &self.original);
        }

        // SPI_SETCURSORS を発火するために apply_cursors の "Arrow のみ" 呼び出しを使う。
        // 退避値が空文字列なら Windows 既定。空ならスキップしてもよいが、念のため通知のみ。
        #[cfg(windows)]
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_SETCURSORS,
            };
            let _ = SystemParametersInfoW(
                SPI_SETCURSORS,
                0,
                None,
                SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
            );
        }
        eprintln!("[restore] Arrow を元値に戻しました: '{}'", self.original);
    }
}

fn run(input: PathBuf) -> Result<serde_json::Value, String> {
    if !input.is_file() {
        return Err(format!(".ani が見つかりません: {}", input.display()));
    }

    // 1. parse_ani でホットスポットを取得
    let bytes = std::fs::read(&input).map_err(|e| format!("読み込み失敗: {e}"))?;
    let parsed = parse_ani(&bytes).map_err(|e| format!("parse_ani 失敗: {e:?}"))?;
    let hotspot = (
        parsed.frames[0].hotspot_x as u16,
        parsed.frames[0].hotspot_y as u16,
    );
    eprintln!(
        "[probe] frames={}, legacy_raw_dib={}, hotspot=({}, {}), bytes={}",
        parsed.frames.len(),
        parsed.is_legacy_raw_dib,
        hotspot.0,
        hotspot.1,
        bytes.len()
    );

    // 2. ~/.custom_cursors/_verify/Arrow.ani に rewrite コピー
    let home = dirs::home_dir().ok_or("home dir 取得失敗")?;
    let dst_dir = home.join(".custom_cursors").join("_verify");
    std::fs::create_dir_all(&dst_dir).map_err(|e| format!("dir 作成失敗: {e}"))?;
    let dst = dst_dir.join("Arrow.ani");
    let stats = rewrite_ani_to_path(&input, &dst, hotspot)
        .map_err(|e| format!("rewrite_ani_to_path 失敗: {e:?}"))?;
    eprintln!(
        "[rewrite] wrote={} bytes (legacy_normalized={})",
        stats.bytes_written, stats.was_legacy_normalized
    );

    // 書いた結果が parse 可能か確認
    let written = std::fs::read(&dst).map_err(|e| format!("書込結果の読み込み失敗: {e}"))?;
    let parsed2 = parse_ani(&written).map_err(|e| format!("書込結果の parse 失敗: {e:?}"))?;
    if parsed2.frames[0].hotspot_x as u16 != hotspot.0
        || parsed2.frames[0].hotspot_y as u16 != hotspot.1
    {
        return Err(format!(
            "書き込み後のホットスポット不一致: expected=({}, {}), got=({}, {})",
            hotspot.0, hotspot.1, parsed2.frames[0].hotspot_x, parsed2.frames[0].hotspot_y
        ));
    }

    // 3. 現在の Arrow を退避 (Drop で必ず復元)
    let guard = ArrowRestoreGuard::snapshot()?;
    eprintln!("[snapshot] original Arrow = '{}'", guard.original);

    // 4. Arrow のみ apply_cursors で書き換え
    use std::collections::HashMap;
    let mut paths: HashMap<String, PathBuf> = HashMap::new();
    paths.insert("Arrow".to_string(), dst.clone());
    RegistryManager::apply_cursors(&paths).map_err(|e| format!("apply_cursors 失敗: {e:?}"))?;

    // 5. HKCU を読み戻して反映を確認
    let after = RegistryManager::read_current_cursors()
        .map_err(|e| format!("適用後の Arrow 読込失敗: {e:?}"))?;
    let after_arrow = after.get("Arrow").cloned().unwrap_or_default();
    let dst_str = dst.to_string_lossy().to_string();
    let registry_ok = after_arrow.eq_ignore_ascii_case(&dst_str);
    eprintln!(
        "[verify] HKCU\\...\\Cursors\\Arrow = '{}' (expected = '{}', match = {})",
        after_arrow, dst_str, registry_ok
    );

    if !registry_ok {
        return Err(format!(
            "適用後の HKCU 値が一致しません: got='{}', expected='{}'",
            after_arrow, dst_str
        ));
    }

    drop(guard); // 明示的に復元 (Drop でも走るが、JSON を出す前に走らせる)

    // 6. 復元後の値も確認
    let restored = RegistryManager::read_current_cursors()
        .map_err(|e| format!("復元後の Arrow 読込失敗: {e:?}"))?;
    let restored_arrow = restored.get("Arrow").cloned().unwrap_or_default();
    eprintln!("[verify] after restore Arrow = '{}'", restored_arrow);

    Ok(serde_json::json!({
        "input": input.display().to_string(),
        "frames": parsed.frames.len(),
        "legacy_raw_dib": parsed.is_legacy_raw_dib,
        "hotspot": [hotspot.0, hotspot.1],
        "bytes_in": bytes.len(),
        "bytes_out": stats.bytes_written,
        "legacy_normalized": stats.was_legacy_normalized,
        "applied_path": dst_str,
        "hkcu_after_apply": after_arrow,
        "hkcu_after_restore": restored_arrow,
        "registry_apply_match": registry_ok,
        "ok": true,
    }))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: apply_ani_verify <path-to.ani>");
        return ExitCode::from(2);
    }
    let input = PathBuf::from(&args[1]);

    match run(input) {
        Ok(v) => {
            println!("{}", serde_json::to_string(&v).unwrap_or_default());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[error] {e}");
            ExitCode::FAILURE
        }
    }
}
