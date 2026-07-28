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
//! | [`snapshot`] | pending / initial スナップショット I/O (atomic temp-write/rename) |
//! | [`transaction`] | snapshot → mutation → notify → commit / rollback ヘルパー |
//!
//! 本ファイル ([`mod`]) には [`RegistryManager`] と [`paths_match_current_registry`]、
//! および直接レジストリ I/O を行う部分を集約している。

pub mod env;
pub mod roles;
pub mod scheme;
pub mod snapshot;
pub mod transaction;

pub use env::expand_env_vars;
pub use roles::CursorRole;
pub use scheme::WindowsScheme;
pub use snapshot::{PendingSnapshotState, RegistrySnapshot};

use crate::config::ConfigManager;
use crate::errors::{AppError, AppResult};
use env::encode_utf16_with_nul;
use scheme::{
    build_scheme_value, compute_apply_values, parse_scheme_value, sanitize_scheme_name,
    scheme_is_app_managed,
};
use std::collections::HashMap;
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

// 注: `RegistrySnapshot` 構造体は Wave 2AB Task 8 で `snapshot` モジュールへ
// 移動した。`pub use snapshot::RegistrySnapshot;` で再エクスポートしているため
// 既存呼び出し側 (他モジュール / テスト) は変更不要。

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
    ///
    /// Wave 2AB Task 8: 実装は `snapshot::save_pending_snapshot` に移譲。
    /// `RegistryManager` は旧呼び出しコードとの後方互換のための façade。
    pub fn save_pending_snapshot(
        values: &HashMap<String, String>,
        theme_id: Option<&str>,
    ) -> AppResult<()> {
        snapshot::save_pending_snapshot(values, theme_id)
    }

    /// pending スナップショットを削除する（適用成功時に呼ぶ）
    pub fn remove_pending_snapshot() -> AppResult<()> {
        snapshot::remove_pending_snapshot()
    }

    /// pending スナップショットが残っているか確認する（起動時チェック）
    ///
    /// Wave 2AB Task 8: 旧実装は JSON パース失敗で `Err` を返していたが、
    /// 破損を「握り潰して何もしない」事故が起きやすかった。新実装は
    /// `snapshot::inspect_pending_snapshot` の `PendingSnapshotState::Valid`
    /// だけを `Some` として返し、それ以外 (`Absent` / `Unreadable`) は
    /// `None` に潰す。新規コードでは `inspect_pending_snapshot` を直接呼ぶ
    /// ことが望ましい (起動時リカバリは Valid / Unreadable を区別せず
    /// Windows 既定リセットに倒す方針)。
    pub fn check_pending_snapshot() -> AppResult<Option<RegistrySnapshot>> {
        match snapshot::inspect_pending_snapshot()? {
            snapshot::PendingSnapshotState::Valid(snap) => Ok(Some(snap)),
            snapshot::PendingSnapshotState::Absent
            | snapshot::PendingSnapshotState::Unreadable { .. } => Ok(None),
        }
    }

    /// pending スナップショットの状態を 3 状態 (`Absent` / `Valid` / `Unreadable`)
    /// で返す。`main.rs` の起動時リカバリが直接呼ぶ推奨 API。`check_pending_snapshot`
    /// は旧 API 互換のためのラッパー (= Valid / それ以外 を二値化)。
    pub fn inspect_pending_snapshot() -> AppResult<PendingSnapshotState> {
        snapshot::inspect_pending_snapshot()
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
    ///
    /// Wave 1C: 内部実装を [`crate::registry::transaction::run_cursor_transaction`]
    /// に委譲する。snapshot → mutation → notify → commit / rollback の契約は
    /// transaction モジュール側で一元管理される。
    pub fn apply_cursors(cursor_paths: &HashMap<String, PathBuf>) -> AppResult<()> {
        let entries = compute_apply_values(cursor_paths);
        let write_values: HashMap<String, String> = entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let spec = crate::registry::transaction::TransactionSpec {
            mode: crate::registry::transaction::TransactionMode::NormalTransactional,
            theme_id: None,
            write_values: &write_values,
            default_scheme_name: None,
        };
        crate::registry::transaction::run_cursor_transaction(&spec)
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
    /// Windows 既定カーソルにリセットする（パニックボタン）
    ///
    /// Wave 2AB Task 8: 内部実装を transaction ヘルパーに委譲する。
    /// モードは `EmergencyBestEffort` (= snapshot 失敗を警告のみで続行)
    /// を維持し、緊急リセットの目的ならば安全側 (= 続行) に倒す方針を変えない。
    /// `(Default)` 値 (= スキーム名表示用) には "Windows Default" を書く。
    pub fn reset_to_windows_default() -> AppResult<()> {
        // 17 役割すべてを空文字列にする (= Windows 既定継承)。
        // Wave 2AB Task 8 レビュー反映: 空 HashMap を渡すと transaction::write_all_roles
        // が何もしない (= パニックボタンが機能不全) だったため、CursorRole::all() を
        // 明示的に populate して全役割のレジストリ値を空文字列に揃える。
        let mut write_values: HashMap<String, String> = HashMap::new();
        for role in CursorRole::all() {
            write_values.insert(role.registry_name().to_string(), String::new());
        }
        let spec = crate::registry::transaction::TransactionSpec {
            mode: crate::registry::transaction::TransactionMode::EmergencyBestEffort,
            theme_id: None,
            write_values: &write_values,
            default_scheme_name: Some("Windows Default"),
        };
        crate::registry::transaction::run_cursor_transaction(&spec)?;
        tracing::info!("Windows 既定カーソルにリセットしました");
        Ok(())
    }

    /// スナップショットからレジストリを復元する (private ラッパー)。
    ///
    /// Wave 2AB Task 8: `restore_from_snapshot_pub` への薄いラッパー。
    /// 旧名 (`restore_from_snapshot`) はテスト内部 (`CursorValuesCleanup::drop`)
    /// から呼ばれるため残している。新規コードは `restore_from_snapshot_pub`
    /// を直接呼ぶこと (= per-role エラー収集が効く)。
    #[allow(dead_code)]
    fn restore_from_snapshot(values: &HashMap<String, String>) -> AppResult<()> {
        Self::restore_from_snapshot_pub(values)
    }

    /// `restore_from_snapshot` の公開版。Wave 1C の `transaction` モジュールから
    /// ロールバック経路で呼ばれる。
    ///
    /// Wave 2AB Task 8: **per-role 書込エラーを収集して 1 つの `AppError::Registry`
    /// として返す**。旧実装は `let _ = cursors_key.set_value(name, value);` で
    /// 個別エラーを握り潰していたが、それだとロールバックが部分失敗 (= 一部役割
    /// だけ書き戻し失敗) しても気づけない。各役割の失敗を `tracing::warn!` で
    /// 記録しつつ、最終的に 1 つの Err にまとめて伝播する。PII redaction は
    /// ロール名 (= 役割レジストリ名 = "Arrow" 等、PII ではない) のみ含むので
    /// redact 不要。
    pub fn restore_from_snapshot_pub(values: &HashMap<String, String>) -> AppResult<()> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let cursors_key = hkcu
            .open_subkey_with_flags("Control Panel\\Cursors", KEY_WRITE)
            .map_err(|e| AppError::Registry(format!("復元時にキーを開けません: {}", e)))?;

        // 各役割ごとに書込を試行し、失敗した役割を (name, error) で集める。
        // 1 役割でも失敗したら残りの役割はスキップせず、できる範囲まで書き戻す
        // (=「半分だけ復元」状態を避ける) 設計だが、呼び出し側 (`transaction`)
        // は最終エラーを 1 つの `AppError::Registry` として受け取る。
        let mut failures: Vec<(String, String)> = Vec::new();
        for (name, value) in values {
            if let Err(e) = cursors_key.set_value(name, value) {
                tracing::warn!(
                    "復元時のレジストリ書込失敗 (role={}, エラー記録のみ続行): {}",
                    name,
                    e
                );
                failures.push((name.clone(), e.to_string()));
            }
        }

        // 即時反映はベストエフォート (旧実装と同じ)。書込が 0 件のとき
        // (= snapshot の中身が空) は SPI 偽陽性で失敗することがあるが、
        // ロールバックとしては mutation が全て成功 / 失敗どちらでも
        // SPI は試行して害がないため、呼び出し側の復元契約には影響しない。
        if let Err(e) = Self::notify_cursor_change_pub() {
            tracing::warn!(
                "復元時の notify_cursor_change 失敗 (レジストリ書込は完了): {}",
                e
            );
        }

        if failures.is_empty() {
            Ok(())
        } else {
            // 失敗した役割をまとめて 1 つの Err にする。最初の失敗を先頭に置き、
            // 残りはデバッグ用に `; ...` で連結する (ロール名は PII ではない)。
            let primary = &failures[0];
            let mut msg = format!(
                "復元時のレジストリ書込失敗: role={} ({})",
                primary.0, primary.1
            );
            for (name, err) in failures.iter().skip(1) {
                msg.push_str(&format!("; role={} ({})", name, err));
            }
            Err(AppError::Registry(msg))
        }
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
    /// commit 経路で呼ばれる。`apply_cursors` などからは元の private 版が使われる。
    #[cfg(windows)]
    pub fn notify_cursor_change_pub() -> AppResult<()> {
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
        Self::notify_cursor_change_pub()
    }

    #[cfg(not(windows))]
    pub fn notify_cursor_change_pub() -> AppResult<()> {
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
    /// マウスポインターサイズを更新する。Windows 設定 UI 側「マウスポインターとタッチ」
    /// スライダーが触る canonical キーと同じ key を書くが、`Accessibility\*` の併存値は
    /// 書かない (v2 invariant — specs/2026-05-23-cursor-size-redesign-v2)。よって両者の
    /// スライダー位置は意図的に同期しない。
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
    /// 1. **`HKCU\Control Panel\Cursors\CursorBaseSize`** に DWORD (32〜256) を書く
    ///    (永続化 — 次回ログオン時にもサイズが保たれる)。
    /// 2. **[`Self::apply_system_cursors_at_size`]** で 14 種の OCR_* 役割について
    ///    `LoadImageW` + `SetSystemCursor` を実行 — 全アプリ・全 HDC で即時視覚反映。
    ///    `NWPen` / `Pin` / `Person` の 3 役割は OCR_* 定数が存在しないため即時反映対象外
    ///    (永続化のみ。次回テーマ適用 / ログオン時に反映される)。
    ///
    /// `HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize` は **書かない**
    /// (Invariant v2 / 2026-05-23 redesign — 下記参照)。
    ///
    /// 本機能は「テーマ適用」とは独立した設定として扱うため、
    /// `_pending_apply.snapshot` には参加しない (= cursor 役割の transactional
    /// apply とは別系統)。テーマを切り替えてもサイズは保持される。
    ///
    /// ## Defensive guard: Windows accessibility eoa pipeline 状態の検出
    ///
    /// 本関数は冒頭で `HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize` を読み、値が
    /// 1 以外ならば即時 [`AppError::Registry`] を返す。Windows Accessibility のスライダー
    /// が 2 以上に設定されていると Windows は eoa pipeline (動的生成された
    /// `*_eoa.cur` ファイルを `%LOCALAPPDATA%\Microsoft\Windows\Cursors\` に置く経路)
    /// を使用しており、本関数の `LoadImageW` + `SetSystemCursor` 経路では視覚反映できない
    /// ためである。UI 側でも `cursor_size_slider != 1` のとき slider を `disabled` に
    /// するが、IPC が他経路から呼ばれた場合の安全網として Rust 側でも拒否する。
    ///
    /// ## Invariant (v2 / 2026-05-23 redesign)
    ///
    /// 本関数は `HKCU\SOFTWARE\Microsoft\Accessibility\*` を **書かない**。
    /// 該当 namespace は Windows Settings UI の専用領域で、アプリが書くと eoa pipeline
    /// が誤作動する (specs/2026-05-23-cursor-size-redesign-v2)。
    #[cfg(windows)]
    pub fn set_cursor_base_size(size: u32) -> AppResult<u32> {
        use winreg::enums::*;
        use winreg::RegKey;

        // (0) Defensive guard: Windows accessibility eoa pipeline がアクティブな
        //     状態 (CursorSize > 1) では本関数を呼ばない。通常は UI 側で slider を
        //     disabled にして防御するが、IPC が他経路から直接呼ばれた場合 / UI に
        //     bug があった場合の安全網。
        //
        //     eoa active 中は cursor file paths が `%LOCALAPPDATA%\Microsoft\Windows\Cursors\`
        //     配下の動的生成 .cur に切替わっており、CursorBaseSize 書込や
        //     LoadImageW + SetSystemCursor では視覚反映できない。Windows Settings UI
        //     経由でしか操作できないため、ここで Err を返す。
        //
        // **fail-closed ポリシー**: `NotFound` (Accessibility キー / CursorSize 値が
        // 未作成 = ユーザーが Windows Settings の eoa を一度も触っていない) のみ
        // 「eoa 非アクティブ」とみなし 1 にフォールバック。それ以外のエラー
        // (permission denied / 型不一致 / I/O 異常 等) は **eoa 状態を判定不能** な
        // ので書込を中止する。`.unwrap_or(1)` で全エラーを 1 に丸めると、
        // 過去 commit 229038e の `KEY_WRITE` フラグ不足バグと同種の "本来 eoa active
        // なのに guard を通過して書込み → cursor 破綻" シナリオが再現するため、
        // 防衛的に Err を返す。
        let current_slider: u32 = match RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"SOFTWARE\Microsoft\Accessibility")
            .and_then(|k| k.get_value::<u32, _>("CursorSize"))
        {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => 1,
            Err(e) => {
                tracing::warn!(
                    "Accessibility\\CursorSize 読取失敗 (eoa 状態判定不能 — fail-closed で書込中止): kind={:?}, err={}",
                    e.kind(),
                    e
                );
                return Err(AppError::Registry(format!(
                    "Accessibility\\CursorSize の読み取りに失敗しました ({}); \
                     eoa pipeline の状態を判定できないため CursorBaseSize 書込を中止します。",
                    e
                )));
            }
        };
        if current_slider != 1 {
            return Err(AppError::Registry(format!(
                "Windows accessibility cursor pipeline is active (CursorSize={}); refuse to \
                 write to avoid interference with eoa pipeline. User must reset to size 1 via \
                 Windows Accessibility Settings before in-app slider can apply.",
                current_slider
            )));
        }

        let clamped = clamp_cursor_base_size(size);
        let slider_pos = base_size_to_slider_position(clamped);

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // **Invariant (specs/2026-05-23-cursor-size-redesign-v2):**
        // `HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize` は **Windows Settings UI 専用**
        // のキーで、アプリからは絶対に書かない。書くと Win11 の eoa pipeline がアクティブ化し、
        // `%LOCALAPPDATA%\Microsoft\Windows\Cursors\*_eoa.cur` を期待される状態になるが、
        // それらのファイルは Settings UI 経由でしか生成されないためカーソルが破綻する。
        // 過去 commit `9d16c2b` 〜 `c3863aa` で「Accessibility と CursorBaseSize を sync 書込」
        // していたが、これが lockout バグの根本原因だった (specs 詳細参照)。
        //
        // slider_pos は log 用にだけ計算しており、書込みは行わない (戻り値は clamped)。

        // (1) HKCU\Control Panel\Cursors\CursorBaseSize を書く (canonical 値)。
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

        // (2) LoadImageW + SetSystemCursor で全 OCR_* 役割を明示サイズで再ロード。
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
            DestroyCursor, LoadImageW, SetSystemCursor, HCURSOR, IMAGE_CURSOR, LR_LOADFROMFILE,
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
            // **失敗時は呼出側に所有権が残る** ため明示的に DestroyCursor で解放
            // しないと最大 14 個 × 数 KB の HCURSOR がプロセス寿命の間リークする。
            match unsafe { SetSystemCursor(hcursor, ocr_id) } {
                Ok(()) => applied += 1,
                Err(e) => {
                    tracing::debug!(
                        "SetSystemCursor 失敗 role='{}' (続行): {}",
                        role.registry_name(),
                        e
                    );
                    // SAFETY: hcursor は LoadImageW で生成され、Ok 経路に入らな
                    // かったので所有権がここに残っている。二重 free にはならない。
                    let _ = unsafe { DestroyCursor(hcursor) };
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
    ///
    /// Wave 2AB Task 8: 実装は `snapshot::save_initial_snapshot` に移譲。
    /// `RegistryManager` は旧呼び出しコードとの後方互換のための façade。
    pub fn save_initial_snapshot() -> AppResult<()> {
        snapshot::save_initial_snapshot()
    }

    /// 初回スナップショットからカーソル設定を復元する
    ///
    /// Wave 2AB Task 8: トランザクションヘルパー経由で復元する。
    /// 「installation 前の状態に戻す」=「ユーザーが意図的にデフォルトにした状態」
    /// なので snapshot 保護 (= NormalTransactional) を適用する。
    /// `active_theme_id` クリアと `cursor-changed` 発火は呼び出し側
    /// (`commands::system::reset_with_cleanup`) で行う。
    pub fn restore_from_initial_snapshot() -> AppResult<()> {
        let snapshot = snapshot::load_initial_snapshot()?;
        let spec = crate::registry::transaction::TransactionSpec {
            mode: crate::registry::transaction::TransactionMode::NormalTransactional,
            theme_id: None,
            write_values: &snapshot.original_values,
            default_scheme_name: None,
        };
        crate::registry::transaction::run_cursor_transaction(&spec)?;
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
    ///
    /// Wave 2AB Task 8: `(Default)` 値の書き換えもトランザクション経由にする
    /// ため、`apply_cursors` (= 17 役割書込) と `(Default)` 書込を 2 回の
    /// transaction に分ける。17 役割書込が snapshot 保護込みで安全側に倒れた
    /// 後、`(Default)` 書込だけ別 transaction (NormalTransactional, default_scheme_name)
    /// で行う。
    pub fn apply_windows_scheme(scheme: &WindowsScheme) -> AppResult<()> {
        let cursor_paths: HashMap<String, PathBuf> = scheme
            .cursor_paths
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.clone(), PathBuf::from(v)))
            .collect();
        Self::apply_cursors(&cursor_paths)?;

        // 既定スキーム名 (`(Default)` 値) を書き換え。コントロールパネルの
        // ドロップダウンで現在のスキームが正しく表示されるようにする。
        // 別 transaction で snapshot 保護し、失敗時のロールバック経路を残す。
        let empty = HashMap::new();
        let spec = crate::registry::transaction::TransactionSpec {
            mode: crate::registry::transaction::TransactionMode::NormalTransactional,
            theme_id: None,
            write_values: &empty,
            default_scheme_name: Some(&scheme.name),
        };
        crate::registry::transaction::run_cursor_transaction(&spec)?;
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
    use std::sync::{Mutex, OnceLock};
    use windows::core::{Error as WinError, HRESULT};

    /// `CursorBaseSize` / `Accessibility\CursorSize` は UUID 分離できないグローバル
    /// レジストリ値。これらを触るテスト (`set_cursor_base_size_*`) は並列実行すると
    /// 互いに干渉するため、このミューテックスを先頭で取得してシリアライズする。
    fn cursor_size_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// `apply_cursors` / `restore_from_snapshot` は `HKCU\Control Panel\Cursors` の
    /// 17 役割という UUID 分離できないグローバル値を書き換える。これらを触るテストを
    /// 並列実行すると互いの書込・復元が干渉するため、このミューテックスを先頭で取得して
    /// シリアライズする (`cursor_size_test_lock` と同型)。
    ///
    /// Wave 1C: `crate::registry::transaction::tests` からも同じ lock を取得する
    /// 必要があるので `pub` 化した。
    pub fn apply_cursors_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// パニック時でも環境変数を確実に復元するための RAII ガード
    /// (keystore.rs のテスト用 `EnvGuard` と同じ流儀)。`new` で旧値を退避し、
    /// `Drop` で元の値に戻す (旧値が無ければ削除)。
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        /// `key` の現在値を退避して `value` をセットする。
        fn new(key: &'static str, value: &std::path::Path) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    /// `apply_cursors` / `restore_from_snapshot` が実機の `HKCU\Control Panel\Cursors`
    /// 17 役割を書き換えるため、テスト前の全役割値を退避し `Drop` で必ず書き戻す RAII ガード。
    /// アサート失敗で panic しても `Drop` が走るので、テスト機のカーソル設定を破壊しない。
    ///
    /// 復元は `restore_from_snapshot` を通すことで「DWORD/文字列書込 + SPI 再通知」まで
    /// フルパイプラインを行い、視覚的にも元のカーソルへ戻す。`Drop` 内では結果を無視し
    /// (復元失敗しても二次 panic を起こさない)、`Drop` 内 panic を禁止する。
    struct CursorValuesCleanup {
        original_values: HashMap<String, String>,
    }

    impl CursorValuesCleanup {
        /// 現在の 17 役割値を退避する。読み取りに失敗した場合は空マップを保持し、
        /// `Drop` を no-op にする (退避できないものは書き戻さない)。
        fn capture() -> Self {
            let original_values = RegistryManager::read_current_cursors().unwrap_or_default();
            Self { original_values }
        }
    }

    impl Drop for CursorValuesCleanup {
        fn drop(&mut self) {
            // 結果は無視する: 復元失敗で panic すると Drop 連鎖が壊れるため。
            let _ = RegistryManager::restore_from_snapshot(&self.original_values);
        }
    }

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

    /// `set_cursor_base_size` が以下を end-to-end で実施することを確認する (v2):
    ///
    /// 1. `HKCU\Control Panel\Cursors\CursorBaseSize` に DWORD でクランプ済値を書く
    /// 2. `HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize` は **絶対に触らない**
    ///    (invariant: app は Accessibility\* を書かない / specs/2026-05-23-cursor-size-redesign-v2)
    /// 3. 範囲外入力は (1) でクランプされる
    ///
    /// SystemParametersInfoW / SetSystemCursor 等の副作用は unit test で検証不能なので、
    /// registry 書込の有無のみを確認する。
    ///
    /// HKCU のみ書込、Drop で 2 キー両方を元の値に復元するためテスト機の
    /// ユーザー設定を壊さない。
    #[test]
    fn set_cursor_base_size_writes_dword_round_trip() {
        use winreg::enums::*;
        use winreg::RegKey;

        // グローバル registry 値を触るテストは並列実行不可。ロックでシリアライズ。
        let _lock = cursor_size_test_lock();
        let _cleanup = CursorBaseSizeCleanup::capture();

        // defensive guard を通過するために CursorSize=1 (eoa pipeline 非アクティブ) を
        // 事前にセットする。_cleanup::drop で元の値に復元される。
        // 同時に、テスト後の比較用にこの「事前セット値」を保持する。
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (a11y_pre, _) = hkcu
            .create_subkey("SOFTWARE\\Microsoft\\Accessibility")
            .expect("Accessibility キー作成失敗");
        a11y_pre
            .set_value("CursorSize", &1u32)
            .expect("CursorSize=1 書込失敗");

        // 通常値の round-trip (64px)
        let written =
            RegistryManager::set_cursor_base_size(64).expect("set_cursor_base_size(64) failed");
        assert_eq!(written, 64, "clamp 内なので入力値がそのまま返るべき");

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

        // **invariant 確認**: Accessibility\CursorSize は事前セット値 1 のまま (= app が触っていない)。
        let a11y_key = hkcu
            .open_subkey("SOFTWARE\\Microsoft\\Accessibility")
            .expect("Accessibility キーが開けない");
        let slider_raw: u32 = a11y_key
            .get_value("CursorSize")
            .expect("Accessibility\\CursorSize が読み戻せない");
        assert_eq!(
            slider_raw, 1,
            "set_cursor_base_size は Accessibility\\CursorSize を書き換えてはいけない \
             (specs/2026-05-23-cursor-size-redesign-v2 invariant)"
        );

        // 範囲外 → クランプ。Accessibility は依然として 1 のまま。
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
            slider_raw, 1,
            "2 回目の set_cursor_base_size 後も Accessibility\\CursorSize は 1 のまま"
        );

        // _cleanup の Drop で両方の値が元に戻る。
    }

    /// defensive guard: `Accessibility\CursorSize != 1` のとき
    /// `set_cursor_base_size` が Err を返すことを確認する。
    ///
    /// テスト前に Accessibility/CursorSize=4 をセットし、テスト後に
    /// `CursorBaseSizeCleanup::drop` で復元 (= 既存 guard が同じ key を扱うため流用)。
    #[test]
    fn set_cursor_base_size_rejects_when_accessibility_active() {
        use winreg::enums::*;
        use winreg::RegKey;

        // グローバル registry 値を触るテストは並列実行不可。ロックでシリアライズ。
        let _lock = cursor_size_test_lock();
        let _cleanup = CursorBaseSizeCleanup::capture();

        // Accessibility\CursorSize = 4 を事前にセット (eoa pipeline 状態を模擬)
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (a11y_key, _) = hkcu
            .create_subkey("SOFTWARE\\Microsoft\\Accessibility")
            .expect("Accessibility キー作成失敗");
        a11y_key
            .set_value("CursorSize", &4u32)
            .expect("CursorSize=4 書込失敗");

        // この状態で set_cursor_base_size を呼ぶと Err になるべき
        let result = RegistryManager::set_cursor_base_size(96);
        assert!(
            matches!(result, Err(AppError::Registry(_))),
            "Accessibility\\CursorSize != 1 のとき set_cursor_base_size は Err を返すべき, got {:?}",
            result
        );
    }

    /// `apply_cursors` の成功パスを end-to-end で行使する特性化テスト。
    ///
    /// 確認する性質:
    ///  1. `cursor_paths` で指定した役割は、その文字列が `HKCU\Control Panel\Cursors`
    ///     に書き込まれる (`read_current_cursors` で一致)。
    ///  2. 指定しなかった役割は空文字列で埋まる (= Windows 既定継承)。
    ///  3. 成功時には pending スナップショットが削除されている
    ///     (`check_pending_snapshot() == Ok(None)`)。
    ///
    /// ロック順序は `apply_cursors_test_lock → cursors_dir_override_lock` で固定し、
    /// 他テストとのデッドロックを避ける。`CUSTOM_CURSORS_DIR_OVERRIDE` を TempDir に
    /// 向けることで実機の `~/.custom_cursors` を汚さず、`CursorValuesCleanup` で
    /// HKCU の 17 役割値を退避・復元する。
    ///
    /// パスには実在する Windows 既定カーソル (`C:\Windows\Cursors\*.cur`) を使う。
    /// レジストリ書込自体はファイル存在を見ないが、`apply_cursors` 末尾の
    /// `notify_cursor_change` (`SystemParametersInfoW(SPI_SETCURSORS)`) は OS が
    /// 実際にカーソルを再ロードするため、存在しないパスだと
    /// `ERROR_FILE_NOT_FOUND` (0x80070002) で失敗する。この HRESULT は本番コードが
    /// 偽陽性として扱う対象に含まれないので、テスト側で実在ファイルを指す必要がある。
    /// `%SystemRoot%` を使わない絶対パスなので env 展開の影響も受けない。
    #[test]
    fn apply_cursors_success_writes_specified_roles_and_clears_others() {
        use tempfile::TempDir;

        // ロック順序を固定: apply 系ロック → override ロック。
        let _apply_lock = apply_cursors_test_lock();
        let _override_lock = crate::config::cursors_dir_override_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let tmp = TempDir::new().unwrap();
        let _env_guard = EnvGuard::new("CUSTOM_CURSORS_DIR_OVERRIDE", tmp.path());

        // HKCU の 17 役割を退避 (Drop で確実に復元)。
        let _cleanup = CursorValuesCleanup::capture();

        // 一部役割だけ埋めた cursor_paths。実在する Windows 既定カーソルを指す
        // (SPI_SETCURSORS の再ロードを通すため)。
        const ARROW_CUR: &str = "C:\\Windows\\Cursors\\aero_arrow.cur";
        const IBEAM_CUR: &str = "C:\\Windows\\Cursors\\beam_i.cur";
        let mut paths: HashMap<String, PathBuf> = HashMap::new();
        paths.insert("Arrow".to_string(), PathBuf::from(ARROW_CUR));
        paths.insert("IBeam".to_string(), PathBuf::from(IBEAM_CUR));

        RegistryManager::apply_cursors(&paths).expect("apply_cursors が成功するべき");

        // 1 + 2: 指定役割は一致、未指定役割は空文字列。
        let current =
            RegistryManager::read_current_cursors().expect("read_current_cursors が成功するべき");
        assert_eq!(
            current.get("Arrow").map(String::as_str),
            Some(ARROW_CUR),
            "指定した Arrow のパスが書き込まれているべき"
        );
        assert_eq!(
            current.get("IBeam").map(String::as_str),
            Some(IBEAM_CUR),
            "指定した IBeam のパスが書き込まれているべき"
        );
        assert_eq!(
            current.get("Wait").map(String::as_str),
            Some(""),
            "未指定の Wait は空文字列 (既定継承) になるべき"
        );
        assert_eq!(
            current.get("Hand").map(String::as_str),
            Some(""),
            "未指定の Hand は空文字列 (既定継承) になるべき"
        );

        // 3: 成功時に pending スナップショットは削除済み。
        let pending = RegistryManager::check_pending_snapshot()
            .expect("check_pending_snapshot が成功するべき");
        assert!(
            pending.is_none(),
            "apply_cursors 成功後は pending スナップショットが削除されているべき, got {pending:?}"
        );
    }

    /// pending スナップショットの保存→確認→削除のライフサイクル往復を、
    /// レジストリに一切触れずに検証する特性化テスト。
    ///
    /// 確認する性質:
    ///  1. `save_pending_snapshot` 後、TempDir 内に `_pending_apply.snapshot` が存在する。
    ///  2. `check_pending_snapshot` が `original_values` / `target_theme_id` /
    ///     `schema_version` を往復で保持する。
    ///  3. `remove_pending_snapshot` 後は `check_pending_snapshot() == Ok(None)`。
    ///
    /// `CUSTOM_CURSORS_DIR_OVERRIDE` を TempDir に向けるだけで完結するため、
    /// `apply_cursors_test_lock` は不要 (HKCU を触らない)。`cursors_dir_override_lock`
    /// のみ取得する。
    #[test]
    fn pending_snapshot_lifecycle_round_trip() {
        use tempfile::TempDir;

        let _override_lock = crate::config::cursors_dir_override_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let tmp = TempDir::new().unwrap();
        let _env_guard = EnvGuard::new("CUSTOM_CURSORS_DIR_OVERRIDE", tmp.path());

        // 並列テスト実行時 (cargo test / cargo llvm-cov) の他テストが残した
        // `_pending_apply.snapshot` 残骸を明示削除して自分の save と
        // 混線させない。cursors_dir_override_lock 自体は Mutex で直列化
        // されているが、`save` → `check` の瞬間にも OS 側のファイル
        // I/O race で他テストの save を読み戻す事例があったため、ここで
        // 明示的にクリーンスタートを切る。
        let _ = RegistryManager::remove_pending_snapshot();

        let mut values: HashMap<String, String> = HashMap::new();
        values.insert("Arrow".to_string(), "C:\\snap\\arrow.cur".to_string());
        values.insert("Wait".to_string(), String::new());

        // 1: 保存するとファイルが TempDir に作られる。
        RegistryManager::save_pending_snapshot(&values, Some("theme-x"))
            .expect("save_pending_snapshot が成功するべき");
        let snapshot_path = tmp.path().join("_pending_apply.snapshot");
        assert!(
            snapshot_path.exists(),
            "_pending_apply.snapshot が TempDir に存在するべき"
        );

        // 2: 読み戻した内容が往復で保持される。
        let snapshot = RegistryManager::check_pending_snapshot()
            .expect("check_pending_snapshot が成功するべき")
            .expect("スナップショットが Some であるべき");
        assert_eq!(
            snapshot.original_values, values,
            "original_values が保存値と一致するべき"
        );
        assert_eq!(
            snapshot.target_theme_id.as_deref(),
            Some("theme-x"),
            "target_theme_id が保存値と一致するべき"
        );
        assert_eq!(snapshot.schema_version, 1, "schema_version は 1 であるべき");

        // 3: 削除後は None。
        RegistryManager::remove_pending_snapshot().expect("remove_pending_snapshot が成功するべき");
        let after = RegistryManager::check_pending_snapshot()
            .expect("削除後の check_pending_snapshot が成功するべき");
        assert!(
            after.is_none(),
            "remove_pending_snapshot 後は None になるべき, got {after:?}"
        );
    }

    /// `restore_from_snapshot` を直接行使する特性化テスト。
    ///
    /// 既知の全 17 役割マップ (`roles` の `CursorRole::all` から役割名を取得) で
    /// `restore_from_snapshot` を呼び、`read_current_cursors` が与えた値と一致することを
    /// 確認する。`apply_cursors` の書込失敗時ロールバック経路が依存する復元動作の特性化。
    ///
    /// HKCU の 17 役割を直接書き換えるため、`apply_cursors_test_lock` を取得し、
    /// `CursorValuesCleanup` でテスト前の値を退避・復元する。
    ///
    /// 各役割には実在する Windows 既定カーソル (`aero_arrow.cur`) を割り当てる。
    /// `restore_from_snapshot` 末尾の `notify_cursor_change` が OS にカーソルを
    /// 再ロードさせるため、存在しないパスだと `ERROR_FILE_NOT_FOUND` (0x80070002)
    /// で失敗するので実在ファイルが必要。全役割を同一ファイルに向けても、各役割が
    /// 「書いた値そのまま」読み戻せるかという往復の特性化には十分。
    #[test]
    fn restore_from_snapshot_writes_all_17_roles() {
        let _apply_lock = apply_cursors_test_lock();

        // HKCU の 17 役割を退避 (Drop で確実に復元)。
        let _cleanup = CursorValuesCleanup::capture();

        // 既知の全 17 役割マップ。SPI 再ロードを通すため実在ファイルを指す。
        const ARROW_CUR: &str = "C:\\Windows\\Cursors\\aero_arrow.cur";
        let mut values: HashMap<String, String> = HashMap::new();
        for role in CursorRole::all() {
            let name = role.registry_name();
            values.insert(name.to_string(), ARROW_CUR.to_string());
        }

        RegistryManager::restore_from_snapshot(&values)
            .expect("restore_from_snapshot が成功するべき");

        let current =
            RegistryManager::read_current_cursors().expect("read_current_cursors が成功するべき");
        for role in CursorRole::all() {
            let name = role.registry_name();
            assert_eq!(
                current.get(name).map(String::as_str),
                Some(ARROW_CUR),
                "役割 {name} が復元値と一致するべき"
            );
        }
    }

    /// `reset_to_windows_default` を直接行使する特性化テスト。
    ///
    /// 起動時クラッシュリカバリ (`main.rs`) の経路 (b) は、適用前値の復元ではなく
    /// この `reset_to_windows_default` で全役割を空文字列にして Windows 既定へ
    /// 倒す。その「17 役割すべてが空文字列になる」というレジストリ書込契約を特性化する。
    ///
    /// レジストリ書込 (全役割 → "") は `reset_to_windows_default` の中で
    /// `notify_cursor_change` (SPI 再ロード) より**前**に完了する。SPI の
    /// `WM_SETTINGCHANGE` ブロードキャストは環境により偽陽性 Err を返すことがある
    /// (応答しないウィンドウのタイムアウト / 全役割が空のときの ERROR_INVALID_PARAMETER 等)
    /// が、それは書込結果には影響しない。よって戻り値の Err は許容し、書込結果
    /// (= read_current_cursors) のみを契約として検証する。
    ///
    /// HKCU の 17 役割を直接書き換えるため、`apply_cursors_test_lock` を取得し、
    /// `CursorValuesCleanup` でテスト前の値を退避・復元する。
    #[test]
    fn reset_to_windows_default_clears_all_17_roles() {
        let _apply_lock = apply_cursors_test_lock();

        // HKCU の 17 役割を退避 (Drop で確実に復元)。
        let _cleanup = CursorValuesCleanup::capture();

        // 戻り値の Err は SPI ブロードキャスト偽陽性のみ許容 (書込は既に完了している)。
        // Registry 以外のエラー種別なら本物の失敗なので panic させる。
        if let Err(e) = RegistryManager::reset_to_windows_default() {
            assert!(
                matches!(e, AppError::Registry(_)),
                "reset_to_windows_default の許容外エラー: {e:?}"
            );
        }

        let current =
            RegistryManager::read_current_cursors().expect("read_current_cursors が成功するべき");
        for role in CursorRole::all() {
            let name = role.registry_name();
            assert_eq!(
                current.get(name).map(String::as_str),
                Some(""),
                "役割 {name} は Windows 既定リセットで空文字列になるべき"
            );
        }
    }

    /// Wave 2AB Task 8: `restore_from_snapshot_pub` が **per-role 書込エラーを収集**
    /// して 1 つの `AppError::Registry` にまとめて返す契約。
    ///
    /// 旧実装は `let _ = cursors_key.set_value(name, value);` で個別エラーを握り潰し、
    /// 「半分だけ復元」状態 (= 一部役割だけ書き戻し失敗) でも成功扱いを返していた。
    /// 正常ケース (= 全役割のレジストリ書込が Err を返さない) で `Ok(())` が返ること
    /// を確認する。失敗ケース (= 1 役割以上のレジストリ書込が Err) を直接起こすのは
    /// 環境依存 (= 16MB 値や DWORD/REG_SZ 衝突等) で CI で安定しないため、ここでは
    /// 「成功時に `Ok` を返すこと」と「実装中に `let _ = ...` 退化が起きないこと」
    /// に絞って保証する (リファクタで復元動作が壊れたら既存テスト
    /// `restore_from_snapshot_writes_all_17_roles` が落ちる)。
    ///
    /// HKCU の 17 役割を直接書き換えるため、`apply_cursors_test_lock` を取得し、
    /// `CursorValuesCleanup` でテスト前の値を退避・復元する。
    #[test]
    fn restore_from_snapshot_pub_succeeds_on_normal_writes() {
        let _apply_lock = apply_cursors_test_lock();
        let _cleanup = CursorValuesCleanup::capture();

        // 実在する Windows 既定カーソルを使って 17 役割すべてを書込。
        const ARROW_CUR: &str = r"C:\Windows\Cursors\aero_arrow.cur";
        let mut values = HashMap::new();
        for role in CursorRole::all() {
            values.insert(role.registry_name().to_string(), ARROW_CUR.to_string());
        }

        let result = RegistryManager::restore_from_snapshot_pub(&values);
        // 成功ケース: Err を返さない。Win32 エラーが混入したら panic。
        assert!(
            result.is_ok(),
            "全役割書込成功時は Err を返さない (= per-role 失敗なし), got {:?}",
            result
        );

        // 復元後のレジストリ状態を確認 (書き込んだパスがそのまま読める)。
        let current = RegistryManager::read_current_cursors().expect("read_current_cursors");
        for role in CursorRole::all() {
            let name = role.registry_name();
            assert_eq!(
                current.get(name).map(String::as_str),
                Some(ARROW_CUR),
                "役割 {name} は書いた値と一致するべき"
            );
        }
    }

    /// Wave 2AB Task 8 parked finding: per-role rollback failure-path の単体テスト。
    ///
    /// `restore_from_snapshot_pub` は 17 役割を順次 set_value し、失敗した役割を
    /// `Vec<(String, String)>` に集めて最後に 1 つの `AppError::Registry` にまとめて
    /// 返す契約。`restore_from_snapshot_pub_succeeds_on_normal_writes` は成功経路の
    /// みしかカバーしておらず、Err 経路 (= 失敗収集 + メッセージ構築) は未テストだった。
    ///
    /// Win32 の `MAX_VALUE_NAME` は **16,383 wchars** (RegSetValueExW の cchValueName
    /// 上限)。これを超える名前で set_value を呼ぶと `ERROR_INVALID_PARAMETER` が
    /// 返り、winreg 経由で確実に Err が伝播する性質を利用する。CI / Windows バージョン
    /// に依存せず決定論的に失敗させられる。
    ///
    /// HKCU の 17 役割を直接書き換えるため、`apply_cursors_test_lock` を取得し、
    /// `CursorValuesCleanup` でテスト前の値を退避・復元する。
    #[test]
    fn restore_from_snapshot_pub_returns_err_when_value_name_exceeds_win32_limit() {
        let _apply_lock = apply_cursors_test_lock();
        let _cleanup = CursorValuesCleanup::capture();

        let mut values = HashMap::new();
        // MAX_VALUE_NAME (16,383) を超える 16,500 chars の名前で Win32 制限を発火させる。
        // 実在する Windows 既定カーソル (.cur) の値を入れれば set_value 自体が Err で
        // 終わるため、`restore_from_snapshot_pub` の per-role 失敗収集経路 (Vec push) を
        // 通過できる。
        let oversized_name = "X".repeat(16_500);
        values.insert(
            oversized_name.clone(),
            r"C:\Windows\Cursors\aero_arrow.cur".to_string(),
        );

        let result = RegistryManager::restore_from_snapshot_pub(&values);
        let err = result
            .expect_err("Win32 MAX_VALUE_NAME を超える名前で set_value した場合、Err が返るべき");
        // per-role 失敗収集で構築されたメッセージに、当該役割名 (oversized_name) と
        // 失敗インジケータ ("復元時のレジストリ書込失敗") が両方含まれる。
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("復元時のレジストリ書込失敗"),
            "Err メッセージが per-role 失敗 summary を含むべき, got: {msg}"
        );
        assert!(
            msg.contains(&oversized_name),
            "Err メッセージに失敗した役割名を含むべき, got: {msg}"
        );
    }
}
