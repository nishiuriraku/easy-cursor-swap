//! EasyCursorSwap レジストリ操作モジュール
//!
//! Windows レジストリへのカーソル設定の読み書き、適用トランザクション、
//! パニックボタン復旧機能を提供する。
//!
//! 主要な操作対象:
//! - HKCU\Control Panel\Cursors (カーソル設定)
//! - HKCU\Control Panel\Cursors\Schemes (スキーム一覧)
//! - SystemParametersInfoW(SPI_SETCURSORS) (即時反映)

use crate::config::ConfigManager;
use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Windows カーソル役割の全17種
/// レジストリ値名をそのまま使用
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CursorRole {
    Arrow,       // 通常の選択
    Help,        // ヘルプの選択
    AppStarting, // バックグラウンドで作業中
    Wait,        // 待ち状態
    Crosshair,   // 領域の選択
    IBeam,       // テキストの選択
    NWPen,       // 手書き
    No,          // 利用不可
    SizeNS,      // 上下に拡大/縮小
    SizeWE,      // 左右に拡大/縮小
    SizeNWSE,    // 斜めに拡大/縮小 1
    SizeNESW,    // 斜めに拡大/縮小 2
    SizeAll,     // 移動
    UpArrow,     // 代替選択
    Hand,        // リンクの選択
    Pin,         // 場所の選択
    Person,      // 人の選択
}

impl CursorRole {
    /// 全17種の役割を返す
    pub fn all() -> &'static [CursorRole] {
        &[
            CursorRole::Arrow,
            CursorRole::Help,
            CursorRole::AppStarting,
            CursorRole::Wait,
            CursorRole::Crosshair,
            CursorRole::IBeam,
            CursorRole::NWPen,
            CursorRole::No,
            CursorRole::SizeNS,
            CursorRole::SizeWE,
            CursorRole::SizeNWSE,
            CursorRole::SizeNESW,
            CursorRole::SizeAll,
            CursorRole::UpArrow,
            CursorRole::Hand,
            CursorRole::Pin,
            CursorRole::Person,
        ]
    }

    /// レジストリ値名を返す
    pub fn registry_name(&self) -> &'static str {
        match self {
            CursorRole::Arrow => "Arrow",
            CursorRole::Help => "Help",
            CursorRole::AppStarting => "AppStarting",
            CursorRole::Wait => "Wait",
            CursorRole::Crosshair => "Crosshair",
            CursorRole::IBeam => "IBeam",
            CursorRole::NWPen => "NWPen",
            CursorRole::No => "No",
            CursorRole::SizeNS => "SizeNS",
            CursorRole::SizeWE => "SizeWE",
            CursorRole::SizeNWSE => "SizeNWSE",
            CursorRole::SizeNESW => "SizeNESW",
            CursorRole::SizeAll => "SizeAll",
            CursorRole::UpArrow => "UpArrow",
            CursorRole::Hand => "Hand",
            CursorRole::Pin => "Pin",
            CursorRole::Person => "Person",
        }
    }

    /// 日本語表示名を返す
    pub fn display_name_ja(&self) -> &'static str {
        match self {
            CursorRole::Arrow => "通常の選択",
            CursorRole::Help => "ヘルプの選択",
            CursorRole::AppStarting => "バックグラウンドで作業中",
            CursorRole::Wait => "待ち状態",
            CursorRole::Crosshair => "領域の選択",
            CursorRole::IBeam => "テキストの選択",
            CursorRole::NWPen => "手書き",
            CursorRole::No => "利用不可",
            CursorRole::SizeNS => "上下に拡大/縮小",
            CursorRole::SizeWE => "左右に拡大/縮小",
            CursorRole::SizeNWSE => "斜めに拡大/縮小 1",
            CursorRole::SizeNESW => "斜めに拡大/縮小 2",
            CursorRole::SizeAll => "移動",
            CursorRole::UpArrow => "代替選択",
            CursorRole::Hand => "リンクの選択",
            CursorRole::Pin => "場所の選択",
            CursorRole::Person => "人の選択",
        }
    }

    /// 英語表示名を返す
    pub fn display_name_en(&self) -> &'static str {
        match self {
            CursorRole::Arrow => "Normal Select",
            CursorRole::Help => "Help Select",
            CursorRole::AppStarting => "Working in Background",
            CursorRole::Wait => "Busy",
            CursorRole::Crosshair => "Precision Select",
            CursorRole::IBeam => "Text Select",
            CursorRole::NWPen => "Handwriting",
            CursorRole::No => "Unavailable",
            CursorRole::SizeNS => "Vertical Resize",
            CursorRole::SizeWE => "Horizontal Resize",
            CursorRole::SizeNWSE => "Diagonal Resize 1",
            CursorRole::SizeNESW => "Diagonal Resize 2",
            CursorRole::SizeAll => "Move",
            CursorRole::UpArrow => "Alternate Select",
            CursorRole::Hand => "Link Select",
            CursorRole::Pin => "Location Select",
            CursorRole::Person => "Person Select",
        }
    }

    /// Schemes 文字列内でのインデックス順序を返す
    pub fn scheme_index(&self) -> usize {
        match self {
            CursorRole::Arrow => 0,
            CursorRole::Help => 1,
            CursorRole::AppStarting => 2,
            CursorRole::Wait => 3,
            CursorRole::Crosshair => 4,
            CursorRole::IBeam => 5,
            CursorRole::NWPen => 6,
            CursorRole::No => 7,
            CursorRole::SizeNS => 8,
            CursorRole::SizeWE => 9,
            CursorRole::SizeNWSE => 10,
            CursorRole::SizeNESW => 11,
            CursorRole::SizeAll => 12,
            CursorRole::UpArrow => 13,
            CursorRole::Hand => 14,
            CursorRole::Pin => 15,
            CursorRole::Person => 16,
        }
    }
}

/// レジストリのスナップショット（適用トランザクション用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySnapshot {
    /// スキーマバージョン
    pub schema_version: u32,
    /// 各役割のカーソルファイルパス
    pub original_values: HashMap<String, String>,
    /// スナップショット取得日時
    pub applied_at: String,
    /// 適用対象のテーマID
    pub target_theme_id: Option<String>,
}

/// レジストリ操作を管理するマネージャー
pub struct RegistryManager;

impl RegistryManager {
    /// 現在のカーソル設定をレジストリから読み取る
    pub fn read_current_cursors() -> AppResult<HashMap<String, String>> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let cursors_key = hkcu
            .open_subkey("Control Panel\\Cursors")
            .map_err(|e| AppError::Registry(format!("Cursors キーを開けません: {}", e)))?;

        let mut values = HashMap::new();
        for role in CursorRole::all() {
            let name = role.registry_name();
            match cursors_key.get_value::<String, _>(name) {
                Ok(val) => {
                    values.insert(name.to_string(), val);
                }
                Err(_) => {
                    // 値が存在しない場合は空文字列
                    values.insert(name.to_string(), String::new());
                }
            }
        }

        Ok(values)
    }

    /// 適用前のスナップショットをディスクに保存する
    /// クラッシュ時の復旧に使用
    pub fn save_pending_snapshot(
        values: &HashMap<String, String>,
        theme_id: Option<&str>,
    ) -> AppResult<()> {
        let snapshot = RegistrySnapshot {
            schema_version: 1,
            original_values: values.clone(),
            applied_at: chrono::Utc::now().to_rfc3339(),
            target_theme_id: theme_id.map(|s| s.to_string()),
        };

        let cursors_dir = ConfigManager::cursors_dir()?;
        let snapshot_path = cursors_dir.join("_pending_apply.snapshot");
        let content = serde_json::to_string_pretty(&snapshot)?;
        fs::write(&snapshot_path, content)?;

        Ok(())
    }

    /// pending スナップショットを削除する（適用成功時に呼ぶ）
    pub fn remove_pending_snapshot() -> AppResult<()> {
        let cursors_dir = ConfigManager::cursors_dir()?;
        let snapshot_path = cursors_dir.join("_pending_apply.snapshot");
        if snapshot_path.exists() {
            fs::remove_file(&snapshot_path)?;
        }
        Ok(())
    }

    /// pending スナップショットが残っているか確認する（起動時チェック）
    pub fn check_pending_snapshot() -> AppResult<Option<RegistrySnapshot>> {
        let cursors_dir = ConfigManager::cursors_dir()?;
        let snapshot_path = cursors_dir.join("_pending_apply.snapshot");
        if snapshot_path.exists() {
            let content = fs::read_to_string(&snapshot_path)?;
            let snapshot: RegistrySnapshot = serde_json::from_str(&content)?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    /// カーソル設定をレジストリに書き込み、即時反映する
    /// トランザクション保護付き
    ///
    /// **17 役割すべて**を書き換える:
    ///   - `cursor_paths` に含まれる役割: そのパスを書き込み
    ///   - 含まれない役割: 空文字列を書き込み (Windows 既定にフォールバック)
    ///
    /// この方針により、前回適用テーマのパスが空きスロットに残留して
    /// 次のテーマと混在する不具合を防ぐ。
    pub fn apply_cursors(cursor_paths: &HashMap<String, PathBuf>) -> AppResult<()> {
        use winreg::enums::*;
        use winreg::RegKey;

        // 1. 現在の設定をスナップショット保存
        let current_values = Self::read_current_cursors()?;
        Self::save_pending_snapshot(&current_values, None)?;

        // 2. レジストリ書き込み
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let cursors_key = hkcu
            .open_subkey_with_flags("Control Panel\\Cursors", KEY_WRITE)
            .map_err(|e| AppError::Registry(format!("Cursors キーを開けません: {}", e)))?;

        let entries = compute_apply_values(cursor_paths);
        for (name, value) in &entries {
            if let Err(e) = cursors_key.set_value(name, value) {
                // 書き込み失敗時はスナップショットから復元
                tracing::error!("レジストリ書き込み失敗 ({}): {}", name, e);
                let _ = Self::restore_from_snapshot(&current_values);
                Self::remove_pending_snapshot()?;
                return Err(AppError::Registry(format!(
                    "カーソル {} の書き込みに失敗: {}",
                    name, e
                )));
            }
        }

        // 3. SystemParametersInfoW で即時反映
        Self::notify_cursor_change()?;

        // 4. スナップショット削除（成功）
        Self::remove_pending_snapshot()?;

        tracing::info!(
            "カーソル設定を適用しました (上書き={} / 既定継承={})",
            cursor_paths.len(),
            17 - cursor_paths.len()
        );
        Ok(())
    }

    /// 適用したテーマを `Control Panel\Cursors\Schemes\<scheme_name>` に登録する。
    ///
    /// これにより Windows のコントロールパネル
    /// (マウスのプロパティ → ポインター → 配色) のドロップダウンに
    /// 自分のテーマが表示されるようになる。
    ///
    /// 値は `REG_EXPAND_SZ` で書き込み、17 役割を scheme_index 順にカンマ区切りする。
    /// 失敗してもユーザー体験への影響は限定的なので、tracing::warn で記録するのみで
    /// 上位層に伝播させる呼び出し元 / 静かに無視する呼び出し元を選べるよう Result を返す。
    pub fn register_scheme(
        scheme_name: &str,
        cursor_paths: &HashMap<String, PathBuf>,
    ) -> AppResult<()> {
        use winreg::enums::*;
        use winreg::RegKey;
        use winreg::RegValue;

        let safe_name = sanitize_scheme_name(scheme_name);
        if safe_name.is_empty() {
            return Err(AppError::Registry(
                "scheme_name が空です (制御文字のみ等)".to_string(),
            ));
        }

        let value_str = build_scheme_value(cursor_paths);
        let bytes = encode_utf16_with_nul(&value_str);

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (schemes_key, _disp) = hkcu
            .create_subkey("Control Panel\\Cursors\\Schemes")
            .map_err(|e| AppError::Registry(format!("Schemes キー作成失敗: {}", e)))?;

        let reg_value = RegValue {
            bytes,
            vtype: REG_EXPAND_SZ,
        };
        schemes_key
            .set_raw_value(&safe_name, &reg_value)
            .map_err(|e| AppError::Registry(format!("Schemes 書き込み失敗: {}", e)))?;

        tracing::info!(
            "Schemes に '{}' を登録しました (REG_EXPAND_SZ, 上書き役割={})",
            safe_name,
            cursor_paths.len()
        );
        Ok(())
    }

    /// Windows 既定カーソルにリセットする（パニックボタン）
    pub fn reset_to_windows_default() -> AppResult<()> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let cursors_key = hkcu
            .open_subkey_with_flags("Control Panel\\Cursors", KEY_WRITE)
            .map_err(|e| AppError::Registry(format!("Cursors キーを開けません: {}", e)))?;

        // 全役割を空文字列に設定 → Windows 既定にフォールバック
        for role in CursorRole::all() {
            let name = role.registry_name();
            let _ = cursors_key.set_value(name, &"");
        }

        // スキーム名も「Windows 既定」に設定
        let _ = cursors_key.set_value("", &"Windows Default");

        Self::notify_cursor_change()?;

        tracing::info!("Windows 既定カーソルにリセットしました");
        Ok(())
    }

    /// スナップショットからレジストリを復元する
    fn restore_from_snapshot(values: &HashMap<String, String>) -> AppResult<()> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let cursors_key = hkcu
            .open_subkey_with_flags("Control Panel\\Cursors", KEY_WRITE)
            .map_err(|e| AppError::Registry(format!("復元時にキーを開けません: {}", e)))?;

        for (name, value) in values {
            let _ = cursors_key.set_value(name, value);
        }

        Self::notify_cursor_change()?;
        Ok(())
    }

    /// SystemParametersInfoW を呼び出してカーソル変更を即時反映する
    #[cfg(windows)]
    fn notify_cursor_change() -> AppResult<()> {
        use windows::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_SETCURSORS,
        };

        unsafe {
            let result = SystemParametersInfoW(
                SPI_SETCURSORS,
                0,
                None,
                SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
            );
            // BOOL=FALSE だが GetLastError=0 の場合、これは SPIF_SENDCHANGE が
            // 内部で行う WM_SETTINGCHANGE のブロードキャストで応答しない
            // トップレベルウィンドウがあったときに発生する偽陽性。
            // レジストリ書き込みは完了済みでカーソル自体は反映されるため、
            // 警告ログのみ残して成功扱いとする。
            if let Err(e) = result {
                if e.code().is_ok() {
                    tracing::warn!(
                        "SystemParametersInfoW(SPI_SETCURSORS) が BOOL=FALSE を返したが \
                         GetLastError=0。WM_SETTINGCHANGE ブロードキャストの \
                         偽陽性として継続"
                    );
                } else {
                    return Err(AppError::Registry(format!(
                        "SystemParametersInfoW の呼び出しに失敗: {}",
                        e
                    )));
                }
            }
        }
        Ok(())
    }

    #[cfg(not(windows))]
    fn notify_cursor_change() -> AppResult<()> {
        // Windows 以外ではスキップ
        tracing::warn!("Windows 以外の環境では SystemParametersInfoW は使用できません");
        Ok(())
    }

    /// OS 標準ポインター影 (`SPI_SETCURSORSHADOW`) の ON/OFF を切り替える。
    /// テーマの `requires_os_shadow` フラグが false のとき OFF にして、
    /// 画像に焼き込まれた影との二重表示を防ぐ。
    #[cfg(windows)]
    pub fn set_cursor_shadow(enabled: bool) -> AppResult<()> {
        use windows::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE,
        };

        // SPI_SETCURSORSHADOW = 0x101D。windows-rs の定数が使えるが念のため数値で指定。
        const SPI_SETCURSORSHADOW: u32 = 0x101D;

        // SPI_SETCURSORSHADOW は uiParam に BOOL 値 (0/1) を渡す仕様。
        // pvParam は使わないので NULL でよい。
        let ui_param: u32 = if enabled { 1 } else { 0 };

        unsafe {
            let result = SystemParametersInfoW(
                windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_ACTION(
                    SPI_SETCURSORSHADOW,
                ),
                ui_param,
                None,
                SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
            );
            // SPI_SETCURSORS と同じく WM_SETTINGCHANGE ブロードキャスト
            // タイムアウトの偽陽性を許容する。
            if let Err(e) = result {
                if e.code().is_ok() {
                    tracing::warn!(
                        "SystemParametersInfoW(SPI_SETCURSORSHADOW) が BOOL=FALSE を \
                         返したが GetLastError=0。ブロードキャスト偽陽性として継続"
                    );
                } else {
                    return Err(AppError::Registry(format!(
                        "SPI_SETCURSORSHADOW の呼び出しに失敗: {}",
                        e
                    )));
                }
            }
        }
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn set_cursor_shadow(_enabled: bool) -> AppResult<()> {
        Ok(())
    }

    /// 初回起動時のスナップショットを保存する
    pub fn save_initial_snapshot() -> AppResult<()> {
        let cursors_dir = ConfigManager::cursors_dir()?;
        let snapshot_path = cursors_dir.join("_initial_snapshot.json");

        // 既に存在する場合は上書きしない
        if snapshot_path.exists() {
            return Ok(());
        }

        let values = Self::read_current_cursors()?;
        let snapshot = RegistrySnapshot {
            schema_version: 1,
            original_values: values,
            applied_at: chrono::Utc::now().to_rfc3339(),
            target_theme_id: None,
        };

        let content = serde_json::to_string_pretty(&snapshot)?;
        fs::write(&snapshot_path, content)?;

        tracing::info!("初回スナップショットを保存しました");
        Ok(())
    }

    /// 初回スナップショットからカーソル設定を復元する
    pub fn restore_from_initial_snapshot() -> AppResult<()> {
        let cursors_dir = ConfigManager::cursors_dir()?;
        let snapshot_path = cursors_dir.join("_initial_snapshot.json");

        if !snapshot_path.exists() {
            return Err(AppError::Registry(
                "初回スナップショットが見つかりません".to_string(),
            ));
        }

        let content = fs::read_to_string(&snapshot_path)?;
        let snapshot: RegistrySnapshot = serde_json::from_str(&content)?;

        Self::restore_from_snapshot(&snapshot.original_values)?;

        tracing::info!("初回スナップショットからカーソル設定を復元しました");
        Ok(())
    }

    /// `HKCU\Control Panel\Cursors\Schemes` に保存されたカーソルスキームを列挙する。
    ///
    /// マウスのプロパティ → ポインター タブの「配色」ドロップダウンに表示される
    /// ユーザー保存スキームと同じ集合。EasyCursorSwap が `register_scheme` で
    /// 書き込んだものも含まれる。
    ///
    /// 各値は `REG_EXPAND_SZ` で `path1,path2,...,path17` の形式。空のスロットは
    /// 「Windows 既定継承」を意味する。`%SystemRoot%` 等の環境変数は OS 側で
    /// 自動展開された絶対パスとして返される。
    ///
    /// 全スロット空のスキーム (= 何も上書きしない) は UI 表示する意味がないので除外する。
    /// Schemes キー自体が存在しない (一度もカスタムスキームを保存していない) 場合は
    /// 空配列を返す。
    pub fn list_windows_schemes() -> AppResult<Vec<WindowsScheme>> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let schemes_key = match hkcu.open_subkey("Control Panel\\Cursors\\Schemes") {
            Ok(k) => k,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                return Err(AppError::Registry(format!(
                    "Schemes キーを開けません: {}",
                    e
                )))
            }
        };

        // EasyCursorSwap が `register_scheme` で書き込んだスキームは ~/.custom_cursors/ 配下の
        // パスを指す。同じテーマがローカルライブラリ (= get_themes) と Windows スキームの双方に
        // 出てしまう二重表示を避けるため、自前管理のスキームはこの段階で取り除く。
        // パス比較は OS のケース非依存比較で行う (Windows のドライブレター/フォルダ名は
        // 大文字小文字区別なし)。
        let app_cursors_dir = ConfigManager::cursors_dir().ok();
        let app_prefix_lower = app_cursors_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_lowercase());

        let mut out: Vec<WindowsScheme> = Vec::new();
        for entry in schemes_key.enum_values() {
            let (name, _raw) = match entry {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("Schemes 値の列挙に失敗: {}", e);
                    continue;
                }
            };
            let value: String = match schemes_key.get_value::<String, _>(&name) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Schemes 値 '{}' の読み取りに失敗: {}", name, e);
                    continue;
                }
            };
            let mut scheme = parse_scheme_value(&name, &value);
            if !scheme.cursor_paths.values().any(|p| !p.is_empty()) {
                continue;
            }
            if let Some(prefix) = app_prefix_lower.as_deref() {
                if scheme_is_app_managed(&scheme, prefix) {
                    tracing::debug!("Schemes '{}' は EasyCursorSwap 管理下のため除外", name);
                    continue;
                }
            }
            scheme.is_active = paths_match_current_registry(&scheme.cursor_paths);
            out.push(scheme);
        }

        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Windows スキームを現在のカーソル設定として適用する。
    ///
    /// 既存の `apply_cursors` をラップし、HKCU\Control Panel\Cursors の
    /// 各役割値を Schemes 値に基づいて書き戻す。スナップショット保護と
    /// SPI_SETCURSORS による即時反映は `apply_cursors` 側で担保される。
    pub fn apply_windows_scheme(scheme: &WindowsScheme) -> AppResult<()> {
        let cursor_paths: HashMap<String, PathBuf> = scheme
            .cursor_paths
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.clone(), PathBuf::from(v)))
            .collect();
        Self::apply_cursors(&cursor_paths)?;

        // 既定スキーム名 (`(Default)` 値) も書き換え、コントロールパネルの
        // ドロップダウンで現在のスキームが正しく表示されるようにする。
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(cursors_key) = hkcu.open_subkey_with_flags("Control Panel\\Cursors", KEY_WRITE) {
            let _ = cursors_key.set_value("", &scheme.name);
        }
        tracing::info!("Windows スキーム '{}' を適用しました", scheme.name);
        Ok(())
    }
}

/// 現在の `HKCU\Control Panel\Cursors` のロール毎パスと、与えられた候補
/// (`expected`) のパス集合が「実質的に一致」しているかを判定する。
///
/// 「一致」の定義:
///   - `expected` の非空エントリすべてについて、現在のレジストリの同じ役割が
///     ASCII case-insensitive で同一パスを持つ
///   - `expected` が空エントリ (= 既定継承) のロールは比較対象外
///   - `expected` が完全に空 (どのロールにもパスを設定しない) なら false
///
/// レジストリ書き換えやテーマ適用後、ユーザーが Windows 側で別スキームに
/// 切り替えた場合、`active_theme_id` と実態が乖離する。これを検出する用途。
pub fn paths_match_current_registry(expected: &HashMap<String, String>) -> bool {
    let non_empty: Vec<(&String, &String)> =
        expected.iter().filter(|(_, v)| !v.is_empty()).collect();
    if non_empty.is_empty() {
        return false;
    }
    let current = match RegistryManager::read_current_cursors() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("paths_match_current_registry: read failed: {}", e);
            return false;
        }
    };
    non_empty.iter().all(|(role, path)| {
        current
            .get(*role)
            .map(|c| c.eq_ignore_ascii_case(path))
            .unwrap_or(false)
    })
}

/// Windows のカーソルスキーム 1 件 (HKCU\Control Panel\Cursors\Schemes の値 1 つ分) を表す。
///
/// `cursor_paths` のキーは `CursorRole::registry_name` と同じ 17 種類の役割名。
/// 値は絶対パス (環境変数展開済み) で、空文字列なら「OS 既定継承」を意味する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsScheme {
    /// Schemes キーでの値名 (= スキーム名)。日本語名や記号も含み得る。
    pub name: String,
    /// 役割名 → 絶対パス (空文字列はスキップ可能)。
    pub cursor_paths: HashMap<String, String>,
    /// 17 役割中、実際にカーソルファイルを上書きする数 (空でないスロット数)。
    pub role_count: usize,
    /// このスキームが現在 `HKCU\Control Panel\Cursors` の値と一致しているか。
    /// `paths_match_current_registry` を経由して `list_windows_schemes` 内で
    /// 集計するため、UI 側は別 IPC で実態問い合わせをする必要がない。
    #[serde(default)]
    pub is_active: bool,
}

/// スキームが EasyCursorSwap 管理下かどうかを判定する。
///
/// 非空の cursor_paths が **すべて** `~/.custom_cursors/` 配下を指していれば
/// アプリが書き込んだものとみなす。1 つでも他のディレクトリ
/// (`%SystemRoot%\cursors\` 等) を含む場合はユーザーが手動で編集した可能性が
/// あるので除外しない。
///
/// 比較は ASCII 小文字化した接頭辞で行う。Windows のパスは大文字小文字を
/// 区別しないため、`%USERPROFILE%` 展開後のパスケース揺れに対応する。
fn scheme_is_app_managed(scheme: &WindowsScheme, app_prefix_lower: &str) -> bool {
    let non_empty: Vec<&String> = scheme
        .cursor_paths
        .values()
        .filter(|p| !p.is_empty())
        .collect();
    if non_empty.is_empty() {
        return false;
    }
    non_empty
        .iter()
        .all(|p| p.to_lowercase().starts_with(app_prefix_lower))
}

/// `path1,path2,...,path17` 形式の Schemes 値を `WindowsScheme` に分解する。
///
/// `build_scheme_value` の逆操作。区切りはカンマ、要素数が 17 未満なら不足分を
/// 空文字列で埋める (古い OS や手書きの不完全なエントリへの耐性)。
/// パス自体は `path` を改変せずそのまま保持する (`%SystemRoot%` 等は呼び出し側が
/// 既に展開済みの想定)。
fn parse_scheme_value(name: &str, value: &str) -> WindowsScheme {
    let parts: Vec<&str> = value.split(',').collect();
    let mut roles: Vec<&CursorRole> = CursorRole::all().iter().collect();
    roles.sort_by_key(|r| r.scheme_index());

    let mut cursor_paths: HashMap<String, String> = HashMap::new();
    let mut role_count: usize = 0;
    for (i, role) in roles.iter().enumerate() {
        let raw = parts.get(i).copied().unwrap_or("").trim().to_string();
        if !raw.is_empty() {
            role_count += 1;
        }
        cursor_paths.insert(role.registry_name().to_string(), raw);
    }
    WindowsScheme {
        name: name.to_string(),
        cursor_paths,
        role_count,
        is_active: false,
    }
}

/// 17 役割それぞれに書き込むレジストリ値を計算する。
///
/// `cursor_paths` に含まれる役割は絶対パス、含まれない役割は空文字列を返す。
/// 空文字列は Windows のレジストリにおいて「既定カーソルへフォールバック」を意味する。
///
/// `apply_cursors` から切り出した純粋関数。レジストリに依存しないので単体テスト可能。
fn compute_apply_values(cursor_paths: &HashMap<String, PathBuf>) -> Vec<(&'static str, String)> {
    CursorRole::all()
        .iter()
        .map(|role| {
            let name = role.registry_name();
            let value = cursor_paths
                .get(name)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            (name, value)
        })
        .collect()
}

/// `Schemes` レジストリ値文字列を構築する。
///
/// 仕様 (Windows コントロールパネル準拠):
///   - 17 役割を `scheme_index` 順に並べる
///   - 区切り文字は **カンマ** `,` (Windows 既定スキームの慣例)
///   - 未指定役割は空文字列を入れる (= 該当役割は OS 既定継承)
///
/// 戻り値の文字列は `REG_EXPAND_SZ` で書き込むことを前提とし、
/// `%SystemRoot%` 等の環境変数展開を許容する。
fn build_scheme_value(cursor_paths: &HashMap<String, PathBuf>) -> String {
    let mut roles: Vec<&CursorRole> = CursorRole::all().iter().collect();
    roles.sort_by_key(|r| r.scheme_index());
    roles
        .iter()
        .map(|role| {
            cursor_paths
                .get(role.registry_name())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Schemes 値名 (= スキーム名) のサニタイズ。
///
/// レジストリ値名は最大 16383 文字だが、UI 整合のため 255 字までに切る。
/// 制御文字 / バックスラッシュ / スラッシュは除去 (キーパス区切りとの混同回避)。
fn sanitize_scheme_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_control() && *c != '\\' && *c != '/')
        .take(255)
        .collect::<String>()
        .trim()
        .to_string()
}

/// 文字列を NUL 終端付き UTF-16 LE バイト列にエンコードする。
/// REG_EXPAND_SZ / REG_SZ の生バイト書き込み用。
fn encode_utf16_with_nul(s: &str) -> Vec<u8> {
    let units: Vec<u16> = s.encode_utf16().chain(std::iter::once(0u16)).collect();
    let mut out = Vec::with_capacity(units.len() * 2);
    for w in units {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_apply_values_returns_all_17_roles() {
        let map: HashMap<String, PathBuf> = HashMap::new();
        let entries = compute_apply_values(&map);
        assert_eq!(entries.len(), 17);
        // 全部空文字列のはず
        assert!(entries.iter().all(|(_, v)| v.is_empty()));
    }

    #[test]
    fn compute_apply_values_writes_specified_paths_and_empty_for_others() {
        let mut map: HashMap<String, PathBuf> = HashMap::new();
        map.insert("Arrow".to_string(), PathBuf::from("C:\\cursors\\arrow.cur"));
        map.insert("IBeam".to_string(), PathBuf::from("C:\\cursors\\ibeam.cur"));

        let entries = compute_apply_values(&map);
        let by_name: HashMap<&str, &str> = entries.iter().map(|(k, v)| (*k, v.as_str())).collect();

        // 指定した役割は書き込み値あり
        assert_eq!(by_name["Arrow"], "C:\\cursors\\arrow.cur");
        assert_eq!(by_name["IBeam"], "C:\\cursors\\ibeam.cur");
        // 指定していない役割は空文字列 (Windows 既定継承)
        assert_eq!(by_name["SizeAll"], "");
        assert_eq!(by_name["Wait"], "");
        assert_eq!(by_name["Hand"], "");
    }

    #[test]
    fn compute_apply_values_ignores_unknown_role_names() {
        // 未知のキーは 17 役割の出力に影響しない
        let mut map: HashMap<String, PathBuf> = HashMap::new();
        map.insert("UnknownRole".to_string(), PathBuf::from("C:\\foo.cur"));
        let entries = compute_apply_values(&map);
        assert_eq!(entries.len(), 17);
        assert!(entries.iter().all(|(_, v)| v.is_empty()));
    }

    #[test]
    fn build_scheme_value_emits_17_comma_separated_slots_in_index_order() {
        let mut map: HashMap<String, PathBuf> = HashMap::new();
        map.insert("Arrow".to_string(), PathBuf::from("C:\\a.cur"));
        map.insert("IBeam".to_string(), PathBuf::from("C:\\i.cur"));
        map.insert("Person".to_string(), PathBuf::from("C:\\p.cur"));

        let value = build_scheme_value(&map);
        let parts: Vec<&str> = value.split(',').collect();
        assert_eq!(parts.len(), 17, "全 17 スロットが必要");
        // scheme_index: Arrow=0, Help=1, AppStarting=2, Wait=3, Crosshair=4, IBeam=5, ..., Person=16
        assert_eq!(parts[0], "C:\\a.cur");
        assert_eq!(parts[5], "C:\\i.cur");
        assert_eq!(parts[16], "C:\\p.cur");
        // 未指定スロットは空文字列
        assert_eq!(parts[1], "");
        assert_eq!(parts[12], ""); // SizeAll
    }

    #[test]
    fn build_scheme_value_all_empty_for_empty_map() {
        let map: HashMap<String, PathBuf> = HashMap::new();
        let value = build_scheme_value(&map);
        // 16 個のカンマ + 0 個のパス = ",,,,,,,,,,,,,,,,"
        assert_eq!(value.matches(',').count(), 16);
        assert!(value.split(',').all(|s| s.is_empty()));
    }

    #[test]
    fn sanitize_scheme_name_strips_control_and_path_separators() {
        assert_eq!(sanitize_scheme_name("My Theme"), "My Theme");
        assert_eq!(sanitize_scheme_name("Foo\\Bar"), "FooBar");
        assert_eq!(sanitize_scheme_name("a/b"), "ab");
        assert_eq!(sanitize_scheme_name("with\nnewline"), "withnewline");
        assert_eq!(sanitize_scheme_name("   trim me   "), "trim me");
    }

    #[test]
    fn sanitize_scheme_name_caps_at_255_chars() {
        let huge = "x".repeat(1000);
        let s = sanitize_scheme_name(&huge);
        assert_eq!(s.len(), 255);
    }

    #[test]
    fn encode_utf16_with_nul_appends_null_terminator() {
        let bytes = encode_utf16_with_nul("Hi");
        // UTF-16 LE: 'H'=0x0048, 'i'=0x0069, NUL=0x0000
        assert_eq!(bytes, vec![0x48, 0x00, 0x69, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn encode_utf16_with_nul_handles_japanese() {
        let bytes = encode_utf16_with_nul("あ"); // U+3042
        assert_eq!(bytes, vec![0x42, 0x30, 0x00, 0x00]);
    }

    #[test]
    fn parse_scheme_value_distributes_paths_in_index_order() {
        // build_scheme_value のラウンドトリップ
        let mut input: HashMap<String, PathBuf> = HashMap::new();
        input.insert("Arrow".into(), PathBuf::from("C:\\a.cur"));
        input.insert("IBeam".into(), PathBuf::from("C:\\i.cur"));
        input.insert("Person".into(), PathBuf::from("C:\\p.cur"));
        let value = build_scheme_value(&input);

        let scheme = parse_scheme_value("My Theme", &value);
        assert_eq!(scheme.name, "My Theme");
        assert_eq!(scheme.role_count, 3);
        assert_eq!(scheme.cursor_paths.get("Arrow").unwrap(), "C:\\a.cur");
        assert_eq!(scheme.cursor_paths.get("IBeam").unwrap(), "C:\\i.cur");
        assert_eq!(scheme.cursor_paths.get("Person").unwrap(), "C:\\p.cur");
        assert_eq!(scheme.cursor_paths.get("Hand").unwrap(), "");
        assert_eq!(scheme.cursor_paths.len(), 17);
    }

    #[test]
    fn parse_scheme_value_pads_missing_slots_with_empty() {
        // 13 要素しか無くても 17 役割マップが返る
        let scheme = parse_scheme_value("Short", "a,b,c,d,e,f,g,h,i,j,k,l,m");
        assert_eq!(scheme.cursor_paths.len(), 17);
        assert_eq!(scheme.cursor_paths.get("Pin").unwrap(), "");
        assert_eq!(scheme.cursor_paths.get("Person").unwrap(), "");
        assert_eq!(scheme.role_count, 13);
    }

    #[test]
    fn scheme_is_app_managed_when_all_paths_under_app_dir() {
        let prefix = "c:\\users\\me\\.custom_cursors";
        let scheme = parse_scheme_value(
            "MyTheme",
            "C:\\Users\\me\\.custom_cursors\\abc\\arrow.cur,\
             C:\\Users\\me\\.custom_cursors\\abc\\ibeam.cur",
        );
        assert!(scheme_is_app_managed(&scheme, prefix));
    }

    #[test]
    fn scheme_is_app_managed_returns_false_when_one_path_is_outside() {
        let prefix = "c:\\users\\me\\.custom_cursors";
        let scheme = parse_scheme_value(
            "Mixed",
            "C:\\Users\\me\\.custom_cursors\\abc\\arrow.cur,\
             %SystemRoot%\\cursors\\ibeam.cur",
        );
        // 1 つでも外部パスがあれば「ユーザーが手で混ぜた」可能性として除外しない。
        assert!(!scheme_is_app_managed(&scheme, prefix));
    }

    #[test]
    fn scheme_is_app_managed_handles_case_insensitive_drive_letter() {
        let prefix = "c:\\users\\me\\.custom_cursors";
        let scheme = parse_scheme_value("UpperCase", "C:\\Users\\Me\\.Custom_Cursors\\arrow.cur");
        assert!(scheme_is_app_managed(&scheme, prefix));
    }

    #[test]
    fn scheme_is_app_managed_returns_false_when_all_empty() {
        let prefix = "c:\\users\\me\\.custom_cursors";
        let scheme = parse_scheme_value("AllEmpty", ",,,,,,,,,,,,,,,,");
        assert!(!scheme_is_app_managed(&scheme, prefix));
    }

    #[test]
    fn parse_scheme_value_trims_whitespace_and_counts_correctly() {
        let scheme = parse_scheme_value("Spaced", " C:\\a.cur ,  ,C:\\c.cur,,,,,,,,,,,,,,");
        assert_eq!(scheme.cursor_paths.get("Arrow").unwrap(), "C:\\a.cur");
        assert_eq!(scheme.cursor_paths.get("Help").unwrap(), "");
        assert_eq!(scheme.cursor_paths.get("AppStarting").unwrap(), "C:\\c.cur");
        assert_eq!(scheme.role_count, 2);
    }
}
