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
    /// ## なぜ 3 段階の通知が必要か
    ///
    /// Windows 設定アプリの「マウスポインターとタッチ」スライダーが内部で行っているのと
    /// 同じ通知シーケンスを再現する。`SystemParametersInfoW(SPI_SETCURSORS, ...)` 単体では
    /// シェル / WebView2 / 一部サードパーティアプリでカーソルが視覚的に更新されない
    /// ケースが OS バージョン (特に Win11 22H2+) で報告されている:
    ///
    /// 1. **`HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize`** にスライダー位置 (1〜15) を
    ///    DWORD で書く。Settings アプリが UI の状態保持用に書いている値で、これが欠けると
    ///    Settings 側の表示と乖離する。
    /// 2. **`HKCU\Control Panel\Cursors\CursorBaseSize`** に DWORD (32〜256) を書く。
    ///    USER32 の cursor renderer がカーソル画像のラスタライズ時に参照する canonical 値。
    /// 3. **`SystemParametersInfoW(SPI_SETCURSORS, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE)`** を
    ///    呼ぶ。kernel に「カーソルキャッシュを破棄して registry から再ロードしろ」と
    ///    指示する。SPIF_SENDCHANGE で WM_SETTINGCHANGE が broadcast されるが、lParam は
    ///    OS 実装依存。
    /// 4. **`SendNotifyMessageW(HWND_BROADCAST, WM_SETTINGCHANGE, 0, L"Cursors")`** を
    ///    明示的に送る (non-blocking)。step 3 の broadcast を無視するアプリ (lParam が
    ///    異なる場合がある) にも「Cursors セクションを再読み込み」を強制する。
    ///
    /// broadcast 偽陽性は [`Self::is_broadcast_false_positive`] で吸収する。
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
        let cursors_key = hkcu
            .open_subkey_with_flags("Control Panel\\Cursors", KEY_WRITE)
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

        // (3) SystemParametersInfoW(SPI_SETCURSORS) で kernel に再ロード指示
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
            // SPI_SETCURSORS と同じ WM_SETTINGCHANGE ブロードキャスト偽陽性が
            // CursorBaseSize 拡大時にこそ頻発する (シェルがカーソルキャッシュを
            // 再構築している最中で受信側の HWND が一時的に無効化される)。
            match &result {
                Ok(()) => tracing::debug!("SPI_SETCURSORS 成功"),
                Err(e) if Self::is_broadcast_false_positive(e) => {
                    tracing::debug!(
                        "SPI_SETCURSORS broadcast 偽陽性 (HRESULT={:#010x}, ignored)",
                        e.code().0
                    );
                }
                Err(e) => {
                    return Err(AppError::Registry(format!(
                        "SPI_SETCURSORS の呼び出しに失敗: {}",
                        e
                    )));
                }
            }
        }

        // (4) 明示的に WM_SETTINGCHANGE(lParam=L"Cursors") を broadcast。
        //     SPIF_SENDCHANGE が送る broadcast の lParam は OS 実装依存で、
        //     L"Cursors" を待ち受けるアプリ (shell / WebView2 含む) が無視する
        //     ケースがあるため、明示送信で完全に上書きする。
        //     SendNotifyMessageW は非ブロッキングで失敗してもログに留めるだけ
        //     (registry 書込は既に完了しているため、broadcast 失敗で IPC 全体を
        //     失敗扱いにはしない)。
        Self::broadcast_setting_change_cursors();

        Ok(clamped)
    }

    /// `WM_SETTINGCHANGE(lParam=L"Cursors")` を `HWND_BROADCAST` 宛に非ブロッキングで送る。
    /// `set_cursor_base_size` step 4 の実装本体。
    #[cfg(windows)]
    fn broadcast_setting_change_cursors() {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            SendNotifyMessageW, HWND_BROADCAST, WM_SETTINGCHANGE,
        };
        // NUL-terminated UTF-16 で "Cursors" を作る。
        let section: Vec<u16> = OsStr::new("Cursors")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: section の生存期間は SendNotifyMessageW 呼び出し中のみ必要
        // (non-blocking とはいえ Windows は message を queue にコピーするまで lParam を
        // 解釈するため、コール完了まで `section` は drop されない)。
        let _ = unsafe {
            SendNotifyMessageW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                WPARAM(0),
                LPARAM(section.as_ptr() as isize),
            )
        };
        // HWND_BROADCAST は実用上ノードのカーネルセマンティクスで個別の HWND を
        // 返さないため、ここでは値ではなく副作用 (各 top-level window への post) のみ確認。
        tracing::debug!("WM_SETTINGCHANGE(lParam=L\"Cursors\") broadcast 送信");
        // 型 import の dead-code 警告抑止 (SendNotifyMessageW のシグネチャと
        // ジェネリック解決で HWND/WPARAM/LPARAM が必要)。
        let _phantom: (Option<HWND>, WPARAM, LPARAM) = (None, WPARAM(0), LPARAM(0));
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
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(cursors_key) =
                hkcu.open_subkey_with_flags("Control Panel\\Cursors", KEY_WRITE)
            {
                match self.cursor_base_size {
                    Some(v) => {
                        let _ = cursors_key.set_value("CursorBaseSize", &v);
                    }
                    None => {
                        let _ = cursors_key.delete_value("CursorBaseSize");
                    }
                }
            }
            if let Ok(a11y_key) =
                hkcu.open_subkey_with_flags("SOFTWARE\\Microsoft\\Accessibility", KEY_WRITE)
            {
                match self.accessibility_cursor_size {
                    Some(v) => {
                        let _ = a11y_key.set_value("CursorSize", &v);
                    }
                    None => {
                        let _ = a11y_key.delete_value("CursorSize");
                    }
                }
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
