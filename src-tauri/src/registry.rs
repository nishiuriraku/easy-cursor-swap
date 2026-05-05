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
            SystemParametersInfoW, SPI_SETCURSORS, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE,
        };

        unsafe {
            let result = SystemParametersInfoW(
                SPI_SETCURSORS,
                0,
                None,
                SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
            );
            if let Err(e) = result {
                return Err(AppError::Registry(format!(
                    "SystemParametersInfoW の呼び出しに失敗: {}",
                    e
                )));
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
            if let Err(e) = result {
                return Err(AppError::Registry(format!(
                    "SPI_SETCURSORSHADOW の呼び出しに失敗: {}",
                    e
                )));
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
}

/// 17 役割それぞれに書き込むレジストリ値を計算する。
///
/// `cursor_paths` に含まれる役割は絶対パス、含まれない役割は空文字列を返す。
/// 空文字列は Windows のレジストリにおいて「既定カーソルへフォールバック」を意味する。
///
/// `apply_cursors` から切り出した純粋関数。レジストリに依存しないので単体テスト可能。
fn compute_apply_values(
    cursor_paths: &HashMap<String, PathBuf>,
) -> Vec<(&'static str, String)> {
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
        let by_name: HashMap<&str, &str> = entries
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

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
}
