//! EasyCursorSwap レジストリ操作モジュール
//!
//! `HKCU\Control Panel\Cursors` 配下のカーソル設定の読み書き、適用トランザクション、
//! パニックボタン復旧、Windows 既存スキームの列挙・適用を提供する。
//!
//! 構成:
//!
//! | サブモジュール | 役割 |
//! |---|---|
//! | [`roles`]  | 17 種カーソル役割 (`CursorRole`) の enum と表示名・index マップ |
//! | [`env`]    | `%SystemRoot%` 等の環境変数展開 / UTF-16 エンコード |
//! | [`scheme`] | `WindowsScheme` 構造体と Schemes 値のパース / シリアライズ pure 関数群 |
//!
//! 本ファイル ([`mod`]) には [`RegistryManager`] と [`paths_match_current_registry`]、
//! および直接レジストリ I/O を行う部分を集約している。

pub mod env;
pub mod roles;
pub mod scheme;

pub use env::expand_env_vars;
pub use roles::CursorRole;
pub use scheme::WindowsScheme;

use crate::config::ConfigManager;
use crate::errors::{AppError, AppResult};
use env::encode_utf16_with_nul;
use scheme::{
    build_scheme_value, compute_apply_values, parse_scheme_value, sanitize_scheme_name,
    scheme_is_app_managed,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
                    // Windows がスキーム適用時に書き込んだ %SYSTEMROOT%\... を
                    // 展開して比較・読込で扱いやすくする。`paths_match_current_registry`
                    // が `WindowsScheme.cursor_paths` (展開済み) と
                    // 突き合わせるため、両側を同じ形式に揃える必要がある。
                    values.insert(name.to_string(), expand_env_vars(&val));
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

    /// 指定テーマディレクトリを指す `HKCU\Control Panel\Cursors\Schemes` 値を削除する。
    ///
    /// テーマ削除時に呼ばれ、Windows のマウスのプロパティ → ポインター → "デザイン"
    /// ドロップダウンに削除済みテーマのスキーム名が残り続ける問題を解消する。
    ///
    /// 「テーマを指す」の判定は [`scheme_is_app_managed`] を再利用する: スキームの
    /// 非空パスが **すべて** `<theme_dir>` 配下を指す場合に、EasyCursorSwap が
    /// `register_scheme` で書いたエントリとみなして削除する。1 つでも別ディレクトリ
    /// (Windows 既定 / 他テーマ) を含むスキームはユーザー手動編集の可能性があるため
    /// 触らない (= 巻き添え削除を防ぐ)。
    ///
    /// パス比較は ASCII 小文字化済みの prefix に対する `starts_with` で行うため、
    /// 末尾にパス区切り `\` を必ず付けて兄弟ディレクトリの誤検出を防ぐ。
    ///
    /// 戻り値: 削除に成功した Schemes 値の数。Schemes キー自体が存在しない場合は
    /// `Ok(0)` (= 成功扱い: そもそも掃除する対象がない)。
    pub fn unregister_schemes_for_theme(theme_dir: &std::path::Path) -> AppResult<usize> {
        use winreg::enums::*;
        use winreg::RegKey;

        let raw = theme_dir.to_string_lossy().to_lowercase();
        if raw.is_empty() {
            return Ok(0);
        }
        let mut prefix_lower = raw;
        if !prefix_lower.ends_with('\\') && !prefix_lower.ends_with('/') {
            prefix_lower.push('\\');
        }

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let schemes_key = match hkcu
            .open_subkey_with_flags("Control Panel\\Cursors\\Schemes", KEY_READ | KEY_WRITE)
        {
            Ok(k) => k,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => {
                return Err(AppError::Registry(format!(
                    "Schemes キーを開けません: {}",
                    e
                )))
            }
        };

        // 列挙中に削除すると挙動が崩れる winreg もあるので、一旦 (name, value) を
        // 集めてから走査する。
        let entries: Vec<(String, String)> = schemes_key
            .enum_values()
            .filter_map(|r| r.ok())
            .filter_map(|(name, _)| {
                schemes_key
                    .get_value::<String, _>(&name)
                    .ok()
                    .map(|v| (name, v))
            })
            .collect();

        let mut removed = 0usize;
        for (name, value) in entries {
            let scheme = parse_scheme_value(&name, &value);
            if scheme_is_app_managed(&scheme, &prefix_lower) {
                match schemes_key.delete_value(&name) {
                    Ok(()) => {
                        removed += 1;
                        tracing::info!(
                            "Schemes から '{}' を削除 (テーマディレクトリ削除に伴うクリーンアップ)",
                            name
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Schemes 値 '{}' の削除に失敗: {}", name, e);
                    }
                }
            }
        }
        Ok(removed)
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
                    // 期待される偽陽性 (無応答ウィンドウへの WM_SETTINGCHANGE
                    // ブロードキャストタイムアウト)。レジストリ書き込みは成功している。
                    tracing::debug!(
                        "SystemParametersInfoW(SPI_SETCURSORS) BOOL=FALSE / GetLastError=0 (broadcast timeout, ignored)"
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
                    // SPI_SETCURSORS と同じ偽陽性 (broadcast timeout)。
                    tracing::debug!(
                        "SystemParametersInfoW(SPI_SETCURSORSHADOW) BOOL=FALSE / GetLastError=0 (broadcast timeout, ignored)"
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
