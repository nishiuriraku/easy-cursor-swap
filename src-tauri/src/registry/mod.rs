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
use winreg::enums::RegType;
use winreg::RegValue;

/// 所有 `Vec<u8>` から `winreg::RegValue` を構築する小さなヘルパー。
///
/// winreg 0.56+ の `RegValue.bytes` は `Cow<'_, [u8]>`; `Vec<u8>` から `.into()` で
/// `Cow::Owned` に変換する。`Cow::Owned` はバッキングストアを所有するので返り値の
/// ライフタイムは `'static`。この変換ロジックを 1 箇所に閉じ込めることで、winreg
/// 側の API 形状が将来また変わったときも修正点を限定できる。
#[inline]
fn to_reg_value(bytes: Vec<u8>, vtype: RegType) -> RegValue<'static> {
    RegValue {
        bytes: bytes.into(),
        vtype,
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

        // REG_EXPAND_SZ で UTF-16 LE バイト列を書き込む。winreg 版差を吸収する
        // `Vec<u8>` → `Cow<'_, [u8]>` 変換は `to_reg_value` ヘルパに集約してある。
        let reg_value = to_reg_value(bytes, REG_EXPAND_SZ);
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

    /// `SystemParametersInfoW(SPI_SETCURSORS / SPI_SETCURSORSHADOW)` が返した
    /// `windows::core::Error` が「レジストリ書き込みは成功しているが SPIF_SENDCHANGE の
    /// ブロードキャストで偽陽性が出た」ケースかどうかを判定する。
    ///
    /// 偽陽性として扱う HRESULT:
    ///  - `0x00000000` (S_OK / GetLastError=0): broadcast timeout (応答しないウィンドウ)
    ///  - `0x80070006` (`HRESULT_FROM_WIN32(ERROR_INVALID_HANDLE=6)`):
    ///    `SPI_SETCURSORS` の引数自体は HWND を取らないため、これは内部の
    ///    `SendMessageTimeout(HWND_BROADCAST, WM_SETTINGCHANGE, …)` で受信側
    ///    (シェル / アクセシビリティサービス等) のカーネルハンドルがライフサイクル
    ///    境界で無効化された場合に観測される。`CursorBaseSize` 拡大時に
    ///    シェルがカーソルキャッシュを再構築している最中で再現しやすい。
    ///  - `0x80070578` (`HRESULT_FROM_WIN32(ERROR_INVALID_WINDOW_HANDLE=1400)`):
    ///    HWND_BROADCAST 中に破棄/初期化途中の HWND を踏んだ場合
    ///    (初回起動直後など、ウィンドウ生成が同時並行している環境で発生)
    #[cfg(windows)]
    fn is_broadcast_false_positive(err: &windows::core::Error) -> bool {
        const HRESULT_INVALID_HANDLE: i32 = 0x80070006u32 as i32;
        const HRESULT_INVALID_WINDOW_HANDLE: i32 = 0x80070578u32 as i32;
        let code = err.code();
        code.is_ok() || code.0 == HRESULT_INVALID_HANDLE || code.0 == HRESULT_INVALID_WINDOW_HANDLE
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
            // BOOL=FALSE が返るが実際にはレジストリ書き込みが完了している
            // 偽陽性のパターンが 3 つある。SPIF_SENDCHANGE が内部で行う
            // WM_SETTINGCHANGE の HWND_BROADCAST 経路で発生する:
            //   1. GetLastError=0 (HRESULT=0x00000000): 応答しないトップレベル
            //      ウィンドウがあったときの broadcast timeout
            //   2. GetLastError=6 (HRESULT=0x80070006 ERROR_INVALID_HANDLE):
            //      ブロードキャスト受信側のカーネルハンドルが境界で無効化された
            //      とき。`CursorBaseSize` 拡大中にシェルがカーソルキャッシュを
            //      再構築している場合に再現しやすい。
            //   3. GetLastError=1400 (HRESULT=0x80070578 ERROR_INVALID_WINDOW_HANDLE):
            //      初回起動直後など、ブロードキャスト先に破棄中・初期化途中の
            //      HWND があったとき
            // いずれもカーソル自体は反映されるため、debug ログで成功扱いにする。
            if let Err(e) = result {
                if Self::is_broadcast_false_positive(&e) {
                    tracing::debug!(
                        "SystemParametersInfoW(SPI_SETCURSORS) broadcast 偽陽性 (HRESULT={:#010x}, ignored)",
                        e.code().0
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
            // SPI_SETCURSORS と同じく WM_SETTINGCHANGE ブロードキャスト系
            // 偽陽性 (timeout / 無効 HWND) を許容する。詳細は notify_cursor_change を参照。
            if let Err(e) = result {
                if Self::is_broadcast_false_positive(&e) {
                    tracing::debug!(
                        "SystemParametersInfoW(SPI_SETCURSORSHADOW) broadcast 偽陽性 (HRESULT={:#010x}, ignored)",
                        e.code().0
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

    /// `HKCU\Control Panel\Cursors\CursorBaseSize` (REG_DWORD) を書き換えて
    /// Windows のアクセシビリティ「マウスポインターとタッチ」のサイズスライダーと
    /// 等価な設定を反映する。
    ///
    /// 引数 `size` は書き込む DWORD 値。範囲外の値は
    /// `clamp_cursor_base_size` で [MIN_CURSOR_BASE_SIZE, MAX_CURSOR_BASE_SIZE]
    /// (32〜256) にクランプされる。戻り値は実際に書き込まれた値。
    ///
    /// ## 反映機構: SetSystemCursor を経由した一方向書込み
    ///
    /// 本関数はアプリを single source of truth として、Windows レジストリとカーネル
    /// カーソルテーブルに **書込みのみ** を行う。`SPI_SETCURSORS` や明示
    /// `WM_SETTINGCHANGE(L"Cursors")` の broadcast は **意図的に行わない** —
    /// それらは自分自身の [`cursor_watcher`][crate::cursor_watcher] が echo として
    /// 受信し、focus 戻り時の auto-refresh と組み合わせると Win↔アプリ往復で
    /// カーソルが徐々に肥大化する双方向同期ループを引き起こすため
    /// (詳細は `docs/superpowers/specs/2026-05-22-cursor-size-architecture-redesign.md`)。
    ///
    /// シーケンス:
    ///
    /// 1. **`HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize`** にスライダー位置 (1〜15) を
    ///    DWORD で書く (Settings アプリの UI 状態保持用、best-effort)。
    /// 2. **`HKCU\Control Panel\Cursors\CursorBaseSize`** に DWORD (32〜256) を書く
    ///    (永続化 — 次回ログオン時にもサイズが保たれる)。
    /// 3. **[`Self::apply_system_cursors_at_size`]** で 14 種の OCR_* 役割について
    ///    `LoadImageW` + `SetSystemCursor` を実行 — 全アプリ・全 HDC で即時視覚反映。
    ///    `NWPen` / `Pin` / `Person` の 3 役割は OCR_* 定数が存在しないため即時反映対象外
    ///    (永続化のみ。次回テーマ適用 / ログオン時に反映される)。
    ///
    /// 本機能は「テーマ適用」とは独立した設定として扱うため、
    /// `_pending_apply.snapshot` には参加しない (= cursor 役割の transactional
    /// apply とは別系統)。テーマを切り替えてもサイズは保持される。
    #[cfg(windows)]
    pub fn set_cursor_base_size(size: u32) -> AppResult<u32> {
        use winreg::enums::*;
        use winreg::RegKey;

        let clamped = clamp_cursor_base_size(size);
        let slider_pos = base_size_to_slider_position(clamped);

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // (1) HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize にスライダー位置を書く。
        //     キー自体が存在しない環境 (clean install 直後 etc.) もあるので create_subkey で
        //     不存在時は作成する。失敗してもメインの書込 (step 2) は続行する (best-effort)。
        match hkcu.create_subkey("SOFTWARE\\Microsoft\\Accessibility") {
            Ok((a11y_key, _disp)) => {
                let slider_dword: u32 = u32::from(slider_pos);
                if let Err(e) = a11y_key.set_value("CursorSize", &slider_dword) {
                    tracing::warn!(
                        "Accessibility\\CursorSize 書込失敗 (best-effort、続行): {}",
                        e
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Accessibility キー open/create 失敗 (best-effort、続行): {}",
                    e
                );
            }
        }

        // (2) HKCU\Control Panel\Cursors\CursorBaseSize を書く (canonical 値)。
        //
        // **KEY_READ | KEY_WRITE 両方が必須**:
        //   - `set_value("CursorBaseSize", ...)` には KEY_WRITE
        //   - 直後の read-back 検証 (`get_value`) と、(4) `apply_system_cursors_at_size`
        //     内の各役割パス取得 (`get_value`) に KEY_READ が必要
        //
        // KEY_WRITE 単独だと `set_value` は成功する一方、同じハンドルでの `get_value` が
        // permission denied で Err になり、本コードは `.ok()` / `Err(_) => continue` で
        // 握り潰すため「書込は成功、読取は静かに失敗」という悪い fail mode に陥る。
        // この結果、過去3回 (41574f7 / cee1398 / d96296c) の修整は本命の機構を実装した
        // にもかかわらず効かなかった (`SetSystemCursor 適用: 0/14`、`read_back=None`)。
        let cursors_key = hkcu
            .open_subkey_with_flags("Control Panel\\Cursors", KEY_READ | KEY_WRITE)
            .map_err(|e| AppError::Registry(format!("Cursors キーを開けません: {}", e)))?;
        cursors_key
            .set_value("CursorBaseSize", &clamped)
            .map_err(|e| AppError::Registry(format!("CursorBaseSize 書込失敗: {}", e)))?;

        // 書込検証 (PII redact なし: DWORD 値そのものは PII でない、UI 設定値)
        let read_back: Option<u32> = cursors_key.get_value("CursorBaseSize").ok();
        tracing::info!(
            "CursorBaseSize 書込: target={} read_back={:?} slider={}",
            clamped,
            read_back,
            slider_pos
        );

        // cursors_key は (4) の SetSystemCursor 用にもう一度だけ使う必要があるので
        // ここでは drop しない。

        // (3) LoadImageW + SetSystemCursor で全 OCR_* 役割を明示サイズで再ロード。
        //     視覚反映の本命経路。これだけで全アプリ・全 HDC のカーソルが即時に
        //     指定サイズへ差し替わる。SPI_SETCURSORS や WM_SETTINGCHANGE の broadcast は
        //     意図的に行わない (docstring 参照)。
        match Self::apply_system_cursors_at_size(&cursors_key, clamped) {
            Ok(applied) => {
                tracing::info!(
                    "SetSystemCursor 適用: {}/14 OCR_* 役割 @ {}px",
                    applied,
                    clamped
                );
            }
            Err(e) => {
                tracing::warn!(
                    "SetSystemCursor 一括適用失敗 (続行 — registry 書込は完了済み): {}",
                    e
                );
            }
        }

        Ok(clamped)
    }

    /// `CursorBaseSize` 書込後の本命: 14 種の OCR_* 役割について `LoadImageW` で
    /// `target_size` ピクセルのカーソルを生成し、`SetSystemCursor` で kernel の
    /// cursor table を直接差し替える。
    ///
    /// `SPI_SETCURSORS` は実行時に `CursorBaseSize` の DWORD 値を再評価しないため、
    /// 視覚反映を即時実現する唯一の経路がこれ。Windows 設定アプリの
    /// 「マウスポインターとタッチ」スライダーが内部で行っているのと同じ動作。
    ///
    /// 戻り値は `SetSystemCursor` が成功した役割の数 (期待値: 14)。
    /// `NWPen` / `Pin` / `Person` は対応する `OCR_*` 定数が存在しないため
    /// 即時反映できず、サイレントスキップする (DWORD 永続化済みなので次回テーマ
    /// 適用 / ログオン時に反映される)。
    ///
    /// 個別役割の `LoadImageW` / `SetSystemCursor` 失敗は best-effort (debug ログ
    /// のみ、続行)。`LR_SHARED` は使わない — `SetSystemCursor` は HCURSOR の
    /// 所有権を OS に移譲して関数内で destroy する仕様のため、`LR_LOADFROMFILE`
    /// 単独で都度ロードする。
    #[cfg(windows)]
    fn apply_system_cursors_at_size(
        cursors_key: &winreg::RegKey,
        target_size: u32,
    ) -> AppResult<usize> {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::{
            LoadImageW, SetSystemCursor, HCURSOR, IMAGE_CURSOR, LR_LOADFROMFILE,
        };

        let mut applied: usize = 0;
        for role in CursorRole::all() {
            let ocr_id = match role_to_ocr_id(*role) {
                Some(id) => id,
                None => {
                    tracing::debug!(
                        "role '{}' は OCR_* マッピングなし — SetSystemCursor スキップ",
                        role.registry_name()
                    );
                    continue;
                }
            };

            // 値が存在しない役割は Windows 既定継承なので、レジストリの空文字列も
            // 「ファイルパスなし」として扱いスキップ。
            let raw_path: String = match cursors_key.get_value(role.registry_name()) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if raw_path.is_empty() {
                continue;
            }

            // REG_EXPAND_SZ で書かれた %SystemRoot% を実パスに展開。
            let expanded = expand_env_vars(&raw_path);

            // UTF-16 NUL 終端の wide string に変換 (LoadImageW のシグネチャ要件)。
            let wide: Vec<u16> = expanded.encode_utf16().chain(std::iter::once(0)).collect();

            // LoadImageW: HMODULE=None (=ファイルからロード), name=ファイルパス,
            // type=IMAGE_CURSOR, cx/cy=target_size (明示サイズ), fuLoad=LR_LOADFROMFILE。
            // 失敗 (パス不正 / ファイル無し / .ani サイズ非対応など) は debug ログで続行。
            let handle = unsafe {
                LoadImageW(
                    None,
                    PCWSTR(wide.as_ptr()),
                    IMAGE_CURSOR,
                    target_size as i32,
                    target_size as i32,
                    LR_LOADFROMFILE,
                )
            };
            let raw_handle = match handle {
                Ok(h) => h,
                Err(e) => {
                    tracing::debug!(
                        "LoadImageW 失敗 role='{}' (続行): {}",
                        role.registry_name(),
                        e
                    );
                    continue;
                }
            };

            // HANDLE -> HCURSOR (windows-rs では別 newtype のため明示変換)。
            // SAFETY: LoadImageW が IMAGE_CURSOR で返した handle は HCURSOR として扱える。
            let hcursor = HCURSOR(raw_handle.0);

            // SetSystemCursor: 成功時は hcursor の所有権を OS 側に移譲し、関数内で
            // destroy されるため呼出側で destroy 不要 (= ここで二重 free しない)。
            match unsafe { SetSystemCursor(hcursor, ocr_id) } {
                Ok(()) => applied += 1,
                Err(e) => {
                    tracing::debug!(
                        "SetSystemCursor 失敗 role='{}' (続行): {}",
                        role.registry_name(),
                        e
                    );
                }
            }
        }

        Ok(applied)
    }

    #[cfg(not(windows))]
    pub fn set_cursor_base_size(size: u32) -> AppResult<u32> {
        Ok(clamp_cursor_base_size(size))
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

/// CursorBaseSize DWORD の最小値 (= Windows 既定 32 px)。
pub const MIN_CURSOR_BASE_SIZE: u32 = 32;

/// CursorBaseSize DWORD の最大値 (= Windows 設定アプリ slider 15 = 256 px)。
pub const MAX_CURSOR_BASE_SIZE: u32 = 256;

/// Windows 設定アプリ「マウスポインターとタッチ」のサイズスライダーは
/// 16 px 刻みで CursorBaseSize を変える (32 / 48 / 64 / ... / 256)。
pub const CURSOR_BASE_SIZE_STEP: u32 = 16;

/// スライダー位置の最小値 (Windows 設定アプリと同じ 1 始まり)。
pub const MIN_CURSOR_SIZE_SLIDER: u8 = 1;

/// スライダー位置の最大値。`MIN_CURSOR_BASE_SIZE + STEP * (MAX_SLIDER - 1) = 256` を満たす。
pub const MAX_CURSOR_SIZE_SLIDER: u8 = 15;

/// 任意の入力値を CursorBaseSize として有効な範囲 [32, 256] にクランプする。
///
/// 端数 (例: 40) はそのまま許容する — Windows は 16 px 刻み以外でも一応動作するが、
/// `.cur` の埋め込みサイズ (32/48/64/96/128/256) と一致しないため Windows 側で
/// 線形補間がかかり、ピクセルアートのカーソルではややぼやけて見える。本アプリの
/// UI スライダーは 16 px 刻みでしか書かないので通常は埋め込みサイズと一致する。
pub fn clamp_cursor_base_size(size: u32) -> u32 {
    size.clamp(MIN_CURSOR_BASE_SIZE, MAX_CURSOR_BASE_SIZE)
}

/// Windows 設定アプリ流のスライダー位置 (1〜15) を CursorBaseSize DWORD に変換する。
///
/// 関係式: `size = MIN_CURSOR_BASE_SIZE + STEP * (slider - 1)`
/// 例: slider=1 → 32 / slider=2 → 48 / slider=3 → 64 / ... / slider=15 → 256
///
/// 範囲外のスライダー位置はクランプ後に変換する (UI が 1〜15 を強制しているが
/// 念のため backend 側でもサニタイズ)。
pub fn slider_position_to_base_size(slider: u8) -> u32 {
    let s = slider.clamp(MIN_CURSOR_SIZE_SLIDER, MAX_CURSOR_SIZE_SLIDER);
    MIN_CURSOR_BASE_SIZE + CURSOR_BASE_SIZE_STEP * u32::from(s - 1)
}

/// CursorBaseSize DWORD を最も近いスライダー位置 (1〜15) に変換する。
///
/// 16 px 刻みに揃っていない値 (例: 40) は四捨五入で最近接スライダーにスナップする。
/// 範囲外の値はクランプ後に変換する。UI で「OS の現在値をスライダーに反映する」
/// 用途。
pub fn base_size_to_slider_position(size: u32) -> u8 {
    let clamped = clamp_cursor_base_size(size);
    // 四捨五入: +STEP/2 してから整数除算
    let offset = clamped - MIN_CURSOR_BASE_SIZE;
    let slider_zero = (offset + CURSOR_BASE_SIZE_STEP / 2) / CURSOR_BASE_SIZE_STEP;
    let slider = slider_zero as u8 + MIN_CURSOR_SIZE_SLIDER;
    slider.clamp(MIN_CURSOR_SIZE_SLIDER, MAX_CURSOR_SIZE_SLIDER)
}

/// `CursorRole` を `SetSystemCursor` 用の `OCR_*` 定数 (= `SYSTEM_CURSOR_ID`) に
/// マップする。`NWPen` / `Pin` / `Person` には対応する `OCR_*` が存在しないため
/// `None` を返す (= `SetSystemCursor` で即時反映できない)。
///
/// マッピング根拠: Windows SDK の `winuser.h` で定義されている 14 種の `OCR_*` 定数
/// (`OCR_NORMAL` / `OCR_IBEAM` / `OCR_WAIT` / `OCR_CROSS` / `OCR_UP` / `OCR_SIZE*` ×6 /
/// `OCR_NO` / `OCR_HAND` / `OCR_APPSTARTING` / `OCR_HELP`)。
///
/// `windows` crate (0.62) は各定数を `SYSTEM_CURSOR_ID(u32)` newtype として export
/// しているので、`SetSystemCursor` のシグネチャ `(HCURSOR, SYSTEM_CURSOR_ID) -> Result<()>`
/// にそのまま渡せる。
#[cfg(windows)]
fn role_to_ocr_id(
    role: CursorRole,
) -> Option<windows::Win32::UI::WindowsAndMessaging::SYSTEM_CURSOR_ID> {
    use windows::Win32::UI::WindowsAndMessaging::{
        OCR_APPSTARTING, OCR_CROSS, OCR_HAND, OCR_HELP, OCR_IBEAM, OCR_NO, OCR_NORMAL, OCR_SIZEALL,
        OCR_SIZENESW, OCR_SIZENS, OCR_SIZENWSE, OCR_SIZEWE, OCR_UP, OCR_WAIT,
    };
    Some(match role {
        CursorRole::Arrow => OCR_NORMAL,
        CursorRole::Help => OCR_HELP,
        CursorRole::AppStarting => OCR_APPSTARTING,
        CursorRole::Wait => OCR_WAIT,
        CursorRole::Crosshair => OCR_CROSS,
        CursorRole::IBeam => OCR_IBEAM,
        CursorRole::No => OCR_NO,
        CursorRole::SizeNS => OCR_SIZENS,
        CursorRole::SizeWE => OCR_SIZEWE,
        CursorRole::SizeNWSE => OCR_SIZENWSE,
        CursorRole::SizeNESW => OCR_SIZENESW,
        CursorRole::SizeAll => OCR_SIZEALL,
        CursorRole::UpArrow => OCR_UP,
        CursorRole::Hand => OCR_HAND,
        // OCR_* 定数が存在しない3役割は即時反映対象外。
        CursorRole::NWPen | CursorRole::Pin | CursorRole::Person => return None,
    })
}

#[cfg(test)]
mod size_helpers_tests {
    use super::*;

    /// スライダー位置 → DWORD の境界値と全有効値のテスト。
    /// Windows 設定アプリの仕様 (1=32, 15=256) と一致することを保証する。
    #[test]
    fn slider_to_base_size_covers_full_range() {
        assert_eq!(slider_position_to_base_size(1), 32);
        assert_eq!(slider_position_to_base_size(2), 48);
        assert_eq!(slider_position_to_base_size(3), 64);
        assert_eq!(slider_position_to_base_size(5), 96);
        assert_eq!(slider_position_to_base_size(7), 128);
        assert_eq!(slider_position_to_base_size(15), 256);
    }

    /// 範囲外スライダーは MIN/MAX にクランプされる。
    #[test]
    fn slider_to_base_size_clamps_out_of_range() {
        assert_eq!(slider_position_to_base_size(0), 32);
        assert_eq!(slider_position_to_base_size(16), 256);
        assert_eq!(slider_position_to_base_size(u8::MAX), 256);
    }

    /// DWORD → スライダーの境界値ラウンドトリップ。
    #[test]
    fn base_size_to_slider_round_trip_at_aligned_values() {
        for s in MIN_CURSOR_SIZE_SLIDER..=MAX_CURSOR_SIZE_SLIDER {
            let size = slider_position_to_base_size(s);
            assert_eq!(
                base_size_to_slider_position(size),
                s,
                "slider {} round-trip via size {}",
                s,
                size
            );
        }
    }

    /// 16 px 刻みに揃っていない中間値は四捨五入で最近接スライダーにスナップする。
    #[test]
    fn base_size_to_slider_snaps_to_nearest() {
        // 32 .. 40 → slider 1 (32 寄り)
        assert_eq!(base_size_to_slider_position(32), 1);
        assert_eq!(base_size_to_slider_position(39), 1);
        // 40 は 32 と 48 の中点、四捨五入で 48 = slider 2
        assert_eq!(base_size_to_slider_position(40), 2);
        assert_eq!(base_size_to_slider_position(47), 2);
        assert_eq!(base_size_to_slider_position(48), 2);
        // 56 は 48 と 64 の中点 → slider 3
        assert_eq!(base_size_to_slider_position(56), 3);
    }

    /// 範囲外 DWORD はクランプ後に変換される。
    #[test]
    fn base_size_to_slider_clamps_out_of_range() {
        assert_eq!(base_size_to_slider_position(0), 1);
        assert_eq!(base_size_to_slider_position(31), 1);
        assert_eq!(base_size_to_slider_position(257), 15);
        assert_eq!(base_size_to_slider_position(u32::MAX), 15);
    }

    /// `role_to_ocr_id` のマッピング契約:
    /// - 14 種の標準カーソル役割は OCR_* 定数にマップされる (Some)
    /// - NWPen / Pin / Person は対応する OCR_* が存在しない (None)
    ///
    /// この契約が変わると `apply_system_cursors_at_size` の挙動 (即時反映できる
    /// 役割数) が変わるため、回帰検出の意味で固定する。
    #[cfg(windows)]
    #[test]
    fn role_to_ocr_id_covers_expected_roles() {
        // OCR_* マッピングが存在する 14 役割
        for role in [
            CursorRole::Arrow,
            CursorRole::Help,
            CursorRole::AppStarting,
            CursorRole::Wait,
            CursorRole::Crosshair,
            CursorRole::IBeam,
            CursorRole::No,
            CursorRole::SizeNS,
            CursorRole::SizeWE,
            CursorRole::SizeNWSE,
            CursorRole::SizeNESW,
            CursorRole::SizeAll,
            CursorRole::UpArrow,
            CursorRole::Hand,
        ] {
            assert!(
                role_to_ocr_id(role).is_some(),
                "{:?} は OCR_* にマップされるべき",
                role
            );
        }
        // OCR_* マッピングがない 3 役割 (Windows 10+ の追加 / 手書きペン専用)
        for role in [CursorRole::NWPen, CursorRole::Pin, CursorRole::Person] {
            assert!(
                role_to_ocr_id(role).is_none(),
                "{:?} には OCR_* 定数が存在しないので None を返すべき",
                role
            );
        }
        // 全 17 役割で合計 14 + 3 = 17 — 抜け漏れがないこと
        let mapped = CursorRole::all()
            .iter()
            .filter(|r| role_to_ocr_id(**r).is_some())
            .count();
        let unmapped = CursorRole::all()
            .iter()
            .filter(|r| role_to_ocr_id(**r).is_none())
            .count();
        assert_eq!(mapped, 14, "OCR_* にマップされる役割は 14 種であるべき");
        assert_eq!(unmapped, 3, "OCR_* にマップされない役割は 3 種であるべき");
    }

    /// clamp_cursor_base_size の境界。
    #[test]
    fn clamp_at_boundaries() {
        assert_eq!(clamp_cursor_base_size(0), 32);
        assert_eq!(clamp_cursor_base_size(32), 32);
        assert_eq!(clamp_cursor_base_size(48), 48);
        assert_eq!(clamp_cursor_base_size(256), 256);
        assert_eq!(clamp_cursor_base_size(1000), 256);
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows::core::{Error as WinError, HRESULT};

    /// `is_broadcast_false_positive` は SPIF_SENDCHANGE 起因の偽陽性を
    /// 拾い、それ以外の Win32 エラーは伝播させる必要がある。
    #[test]
    fn broadcast_false_positive_accepts_s_ok() {
        // GetLastError=0 (broadcast timeout) → HRESULT 0x00000000
        let err = WinError::new(HRESULT(0), "");
        assert!(RegistryManager::is_broadcast_false_positive(&err));
    }

    #[test]
    fn broadcast_false_positive_accepts_invalid_handle() {
        // GetLastError=6 → HRESULT_FROM_WIN32 = 0x80070006
        // 「マウスポインターとタッチ」で CursorBaseSize を拡大中に
        // テーマ適用するとシェル側がカーソルキャッシュ再構築中で
        // broadcast 受信側のカーネルハンドルが一瞬無効化されるケース。
        let err = WinError::new(HRESULT(0x80070006u32 as i32), "");
        assert!(RegistryManager::is_broadcast_false_positive(&err));
    }

    #[test]
    fn broadcast_false_positive_accepts_invalid_window_handle() {
        // GetLastError=1400 → HRESULT_FROM_WIN32 = 0x80070578
        let err = WinError::new(HRESULT(0x80070578u32 as i32), "");
        assert!(RegistryManager::is_broadcast_false_positive(&err));
    }

    #[test]
    fn broadcast_false_positive_rejects_other_errors() {
        // E_FAIL (0x80004005) は本物の失敗として伝播させる
        let err = WinError::new(HRESULT(0x80004005u32 as i32), "");
        assert!(!RegistryManager::is_broadcast_false_positive(&err));
        // ERROR_ACCESS_DENIED (5) のような他の Win32 系も伝播させる
        let err = WinError::new(HRESULT(0x80070005u32 as i32), "");
        assert!(!RegistryManager::is_broadcast_false_positive(&err));
        // ERROR_INVALID_HANDLE (6) のすぐ隣の値 ERROR_INVALID_DATA (13) は
        // 偽陽性ではないので伝播させる (HRESULT 0x8007000D)
        let err = WinError::new(HRESULT(0x8007000Du32 as i32), "");
        assert!(!RegistryManager::is_broadcast_false_positive(&err));
    }

    /// 失敗時にも必ず HKCU の Schemes 値を掃除するための RAII ガード。
    /// アサート失敗で panic しても `Drop` が走るので、ローカルマシンに
    /// テストスキームが残らない。
    struct SchemeCleanup {
        name: String,
    }

    impl Drop for SchemeCleanup {
        fn drop(&mut self) {
            use winreg::enums::*;
            use winreg::RegKey;
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(schemes_key) =
                hkcu.open_subkey_with_flags("Control Panel\\Cursors\\Schemes", KEY_WRITE)
            {
                let _ = schemes_key.delete_value(&self.name);
            }
        }
    }

    /// `register_scheme` が `HKCU\Control Panel\Cursors\Schemes` に書き込んだ値を
    /// 読み戻して、型 (`REG_EXPAND_SZ`) とバイト列が `encode_utf16_with_nul` で
    /// 生成した期待値と一致することを確認する。
    ///
    /// このテストは `to_reg_value` ヘルパ (= `Vec<u8>` → `Cow::Owned` 変換) を
    /// 実 HKCU に対して end-to-end で行使する唯一の動線。`Cow::Borrowed` への
    /// 退行や bytes 順序の取り違えが将来発生したら CI でここが落ちる。
    ///
    /// HKCU のみ書き込み、`Drop` で必ず掃除する。HKLM には一切触らない。
    #[test]
    fn register_scheme_writes_expand_sz_round_trip() {
        use winreg::enums::*;
        use winreg::RegKey;

        // UUID 化で同時並行テストとの衝突を避ける。先頭プレフィックスでテスト
        // 用と分かる名前にしておけば、Drop が走らず残留した場合の手動掃除も簡単。
        let scheme_name = format!("ecs_test_scheme_{}", uuid::Uuid::new_v4());
        let _cleanup = SchemeCleanup {
            name: scheme_name.clone(),
        };

        // 17 役割のうち一部だけ埋めた cursor_paths を渡す。中身のパスは
        // ファイルが存在しなくても registry 書き込み自体は通る (Windows 側で
        // 参照される時点で初めてファイル存在チェックが走るため)。
        let mut paths = HashMap::new();
        paths.insert(
            "Arrow".to_string(),
            PathBuf::from("C:\\test\\round_trip\\arrow.cur"),
        );
        paths.insert(
            "Hand".to_string(),
            PathBuf::from("C:\\test\\round_trip\\hand.cur"),
        );

        // 書き込み。
        RegistryManager::register_scheme(&scheme_name, &paths).expect("register_scheme failed");

        // 期待値: `build_scheme_value` の出力を UTF-16 LE + NUL でエンコードした
        // バイト列がそのまま REG_EXPAND_SZ として格納されているはず。
        let expected_value_str = build_scheme_value(&paths);
        let expected_bytes = encode_utf16_with_nul(&expected_value_str);

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let schemes_key = hkcu
            .open_subkey("Control Panel\\Cursors\\Schemes")
            .expect("Schemes キーが開けない");
        let raw = schemes_key
            .get_raw_value(&scheme_name)
            .expect("書き込んだ値が読み戻せない");

        assert_eq!(
            raw.vtype, REG_EXPAND_SZ,
            "REG_EXPAND_SZ で書き込まれているべき"
        );
        assert_eq!(
            raw.bytes.as_ref(),
            expected_bytes.as_slice(),
            "書き込まれたバイト列が encode_utf16_with_nul の出力と一致するべき"
        );

        // _cleanup の Drop でテストスキームは削除される。
    }

    /// テスト終了時に CursorBaseSize と Accessibility\CursorSize を元に戻す RAII ガード。
    /// 両方の値は単一値で UUID 分離できないため、テスト前に読み取った値を保存し、
    /// Drop で必ず復元する。テスト機のユーザー設定を破壊しないためのセーフティ。
    struct CursorBaseSizeCleanup {
        cursor_base_size: Option<u32>,
        accessibility_cursor_size: Option<u32>,
    }

    impl CursorBaseSizeCleanup {
        fn capture() -> Self {
            use winreg::enums::*;
            use winreg::RegKey;
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let cursor_base_size = hkcu
                .open_subkey("Control Panel\\Cursors")
                .ok()
                .and_then(|k| k.get_value::<u32, _>("CursorBaseSize").ok());
            let accessibility_cursor_size = hkcu
                .open_subkey("SOFTWARE\\Microsoft\\Accessibility")
                .ok()
                .and_then(|k| k.get_value::<u32, _>("CursorSize").ok());
            Self {
                cursor_base_size,
                accessibility_cursor_size,
            }
        }
    }

    impl Drop for CursorBaseSizeCleanup {
        fn drop(&mut self) {
            use winreg::enums::*;
            use winreg::RegKey;

            // 元の DWORD 値があれば set_cursor_base_size 経由で「完全復元」する。
            // これは DWORD 書込 + SetSystemCursor までフルパイプラインを通すので、
            // 視覚的にもユーザーの元のサイズに戻る (テスト機の cursors を 256px のまま
            // 放置しないため重要 — SetSystemCursor の効果はセッション終了まで残る)。
            //
            // 元値が None (= 未設定 = Windows 既定 32px 相当) のときは値そのものは
            // 削除しつつ、視覚反映のために 32px で SetSystemCursor を流す。
            let restored_size = self.cursor_base_size.unwrap_or(MIN_CURSOR_BASE_SIZE);
            let _ = RegistryManager::set_cursor_base_size(restored_size);

            // set_cursor_base_size は値を書く動作なので、「元から値が無かった」
            // ケースでは書込んだ値を削除し直す (= 真の原状回復)。
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if self.cursor_base_size.is_none() {
                if let Ok(cursors_key) =
                    hkcu.open_subkey_with_flags("Control Panel\\Cursors", KEY_WRITE)
                {
                    let _ = cursors_key.delete_value("CursorBaseSize");
                }
            }
            if self.accessibility_cursor_size.is_none() {
                if let Ok(a11y_key) =
                    hkcu.open_subkey_with_flags("SOFTWARE\\Microsoft\\Accessibility", KEY_WRITE)
                {
                    let _ = a11y_key.delete_value("CursorSize");
                }
            } else if let (Some(orig), Ok(a11y_key)) = (
                self.accessibility_cursor_size,
                hkcu.open_subkey_with_flags("SOFTWARE\\Microsoft\\Accessibility", KEY_WRITE),
            ) {
                // set_cursor_base_size は base_size_to_slider_position で再計算する
                // ため、四捨五入で元のスライダー位置とズレるケースがありうる。
                // 元のスライダー値を正確に書き戻す。
                let _ = a11y_key.set_value("CursorSize", &orig);
            }
        }
    }

    /// `set_cursor_base_size` が以下を end-to-end で実施することを確認する:
    ///
    /// 1. `HKCU\Control Panel\Cursors\CursorBaseSize` に DWORD でクランプ済値を書く
    /// 2. `HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize` にスライダー位置 (1〜15) を書く
    /// 3. 範囲外入力でも (1) でクランプされ (2) のスライダー位置も対応する
    ///
    /// (3) と (4) の SystemParametersInfoW / WM_SETTINGCHANGE broadcast は副作用なので
    /// unit test で直接検証できないが、registry 書込まで完走することで本番動線の
    /// 大半をカバーする。
    ///
    /// HKCU のみ書込、Drop で 2 キー両方を元の値に復元するためテスト機の
    /// ユーザー設定を壊さない。
    #[test]
    fn set_cursor_base_size_writes_dword_round_trip() {
        use winreg::enums::*;
        use winreg::RegKey;

        let _cleanup = CursorBaseSizeCleanup::capture();

        // 通常値の round-trip (slider=3 = 64px)
        let written =
            RegistryManager::set_cursor_base_size(64).expect("set_cursor_base_size(64) failed");
        assert_eq!(written, 64, "clamp 内なので入力値がそのまま返るべき");

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let cursors_key = hkcu
            .open_subkey("Control Panel\\Cursors")
            .expect("Cursors キーが開けない");
        let raw: u32 = cursors_key
            .get_value("CursorBaseSize")
            .expect("CursorBaseSize が読み戻せない");
        assert_eq!(
            raw, 64,
            "CursorBaseSize が DWORD として書き込まれているべき"
        );

        // Accessibility\CursorSize にも slider 位置 3 が書かれているはず
        let a11y_key = hkcu
            .open_subkey("SOFTWARE\\Microsoft\\Accessibility")
            .expect("Accessibility キーが開けない (set_cursor_base_size が作成しているはず)");
        let slider_raw: u32 = a11y_key
            .get_value("CursorSize")
            .expect("Accessibility\\CursorSize が読み戻せない");
        assert_eq!(
            slider_raw, 3,
            "Accessibility\\CursorSize に slider 位置 3 が書き込まれているべき (64px ↔ slider 3)"
        );

        // 範囲外 → クランプ
        let written =
            RegistryManager::set_cursor_base_size(1000).expect("set_cursor_base_size(1000) failed");
        assert_eq!(written, MAX_CURSOR_BASE_SIZE);
        let raw: u32 = cursors_key
            .get_value("CursorBaseSize")
            .expect("CursorBaseSize が読み戻せない");
        assert_eq!(raw, MAX_CURSOR_BASE_SIZE);
        let slider_raw: u32 = a11y_key
            .get_value("CursorSize")
            .expect("Accessibility\\CursorSize が読み戻せない");
        assert_eq!(
            slider_raw,
            u32::from(MAX_CURSOR_SIZE_SLIDER),
            "256px は slider 15 に対応するべき"
        );

        // _cleanup の Drop で両方の値が元に戻る。
    }
}
