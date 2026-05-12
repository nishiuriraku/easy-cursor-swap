//! Windows カーソルスキーム連携 — `HKCU\Control Panel\Cursors\Schemes` の
//! 値 1 つを `WindowsScheme` 構造体で扱うための pure 関数群。
//!
//! Schemes 値は `path1,path2,...,path17` のカンマ区切り文字列で、
//! [`CursorRole::scheme_index`] 順に並ぶ。レジストリ I/O は
//! [`super::manager`] / [`super::mod`] の `RegistryManager` 側で行い、
//! ここではパースとシリアライズだけを純粋関数で提供する (テスト容易性のため)。

use super::env::expand_env_vars;
use super::roles::CursorRole;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
pub(crate) fn scheme_is_app_managed(scheme: &WindowsScheme, app_prefix_lower: &str) -> bool {
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
pub(crate) fn parse_scheme_value(name: &str, value: &str) -> WindowsScheme {
    let parts: Vec<&str> = value.split(',').collect();
    let mut roles: Vec<&CursorRole> = CursorRole::all().iter().collect();
    roles.sort_by_key(|r| r.scheme_index());

    let mut cursor_paths: HashMap<String, String> = HashMap::new();
    let mut role_count: usize = 0;
    for (i, role) in roles.iter().enumerate() {
        let raw = parts.get(i).copied().unwrap_or("").trim();
        // %SYSTEMROOT%\Cursors\... のようなレジストリ生値を絶対パスへ展開する。
        // 展開しないと後段の `is_file()` 判定が常に false になりサムネイルが
        // 表示されない / cursorpack エクスポートでファイル読込に失敗する。
        let expanded = if raw.is_empty() {
            String::new()
        } else {
            expand_env_vars(raw)
        };
        if !expanded.is_empty() {
            role_count += 1;
        }
        cursor_paths.insert(role.registry_name().to_string(), expanded);
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
pub(crate) fn compute_apply_values(
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

/// `Schemes` レジストリ値文字列を構築する。
///
/// 仕様 (Windows コントロールパネル準拠):
///   - 17 役割を `scheme_index` 順に並べる
///   - 区切り文字は **カンマ** `,` (Windows 既定スキームの慣例)
///   - 未指定役割は空文字列を入れる (= 該当役割は OS 既定継承)
///
/// 戻り値の文字列は `REG_EXPAND_SZ` で書き込むことを前提とし、
/// `%SystemRoot%` 等の環境変数展開を許容する。
pub(crate) fn build_scheme_value(cursor_paths: &HashMap<String, PathBuf>) -> String {
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
pub(crate) fn sanitize_scheme_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_control() && *c != '\\' && *c != '/')
        .take(255)
        .collect::<String>()
        .trim()
        .to_string()
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

        assert_eq!(by_name["Arrow"], "C:\\cursors\\arrow.cur");
        assert_eq!(by_name["IBeam"], "C:\\cursors\\ibeam.cur");
        assert_eq!(by_name["SizeAll"], "");
        assert_eq!(by_name["Wait"], "");
        assert_eq!(by_name["Hand"], "");
    }

    #[test]
    fn compute_apply_values_ignores_unknown_role_names() {
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
        assert_eq!(parts[0], "C:\\a.cur");
        assert_eq!(parts[5], "C:\\i.cur");
        assert_eq!(parts[16], "C:\\p.cur");
        assert_eq!(parts[1], "");
        assert_eq!(parts[12], ""); // SizeAll
    }

    #[test]
    fn build_scheme_value_all_empty_for_empty_map() {
        let map: HashMap<String, PathBuf> = HashMap::new();
        let value = build_scheme_value(&map);
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
    fn parse_scheme_value_distributes_paths_in_index_order() {
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

    /// テーマ削除時のスキームクリーンアップでは、アプリ全体ではなく **個別テーマ
    /// ディレクトリ** を prefix にして「そのテーマを指すスキーム」だけを抽出する。
    /// 同じ pure 関数を異なる粒度の prefix で使い回せることを保証する。
    #[test]
    fn scheme_is_app_managed_matches_per_theme_subdir() {
        let theme_prefix = "c:\\users\\me\\.custom_cursors\\11111111-1111-1111-1111-111111111111\\";
        let scheme = parse_scheme_value(
            "EasyCursorSwap - ThemeOne",
            "C:\\Users\\me\\.custom_cursors\\11111111-1111-1111-1111-111111111111\\arrow.cur,\
             C:\\Users\\me\\.custom_cursors\\11111111-1111-1111-1111-111111111111\\ibeam.cur",
        );
        assert!(scheme_is_app_managed(&scheme, theme_prefix));
    }

    /// 別テーマ UUID 配下を指すスキームは、対象テーマの prefix では一致しない
    /// (= 削除対象から除外される) ことを保証する。テーマ A 削除でテーマ B の
    /// スキームを巻き添えで消さないための回帰テスト。
    #[test]
    fn scheme_is_app_managed_excludes_other_theme_subdirs() {
        let theme_prefix = "c:\\users\\me\\.custom_cursors\\11111111-1111-1111-1111-111111111111\\";
        let scheme = parse_scheme_value(
            "Other",
            "C:\\Users\\me\\.custom_cursors\\22222222-2222-2222-2222-222222222222\\arrow.cur",
        );
        assert!(!scheme_is_app_managed(&scheme, theme_prefix));
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
