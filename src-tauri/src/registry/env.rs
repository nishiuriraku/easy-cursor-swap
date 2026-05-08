//! Win32 環境変数展開と UTF-16 エンコードの薄いラッパー。
//!
//! `HKCU\Control Panel\Cursors` 配下のレジストリ値は `REG_EXPAND_SZ` 型で
//! `%SystemRoot%\Cursors\…` のような未展開のパスが入っているため、
//! ファイル読み込み前に必ず [`expand_env_vars`] を通す。

/// `%SYSTEMROOT%` 等の環境変数を Win32 `ExpandEnvironmentStringsW` で展開する。
///
/// `winreg::RegKey::get_value::<String>` は REG_EXPAND_SZ を生のまま返すため、
/// `HKCU\Cursors\Schemes` の値や `HKCU\Cursors\<role>` の値を直接ファイル
/// 読み込みに使う前に必ずこの関数を通す必要がある。展開に失敗した場合は
/// 入力をそのまま返す (= ベストエフォート)。
#[cfg(windows)]
pub fn expand_env_vars(input: &str) -> String {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::System::Environment::ExpandEnvironmentStringsW;

    if !input.contains('%') {
        return input.to_string();
    }
    let src_h = HSTRING::from(input);
    // 1 回目で必要バッファサイズを問い合わせ、2 回目で実際に書き込む。
    let needed = unsafe { ExpandEnvironmentStringsW(PCWSTR(src_h.as_ptr()), None) };
    if needed == 0 {
        return input.to_string();
    }
    let mut buf: Vec<u16> = vec![0u16; needed as usize];
    let written =
        unsafe { ExpandEnvironmentStringsW(PCWSTR(src_h.as_ptr()), Some(buf.as_mut_slice())) };
    if written == 0 {
        return input.to_string();
    }
    // 戻り値には NUL を含む。strip the trailing NUL(s) for safety.
    let trimmed = if (written as usize) > 0 && buf.last() == Some(&0u16) {
        &buf[..(written as usize - 1).min(buf.len())]
    } else {
        &buf[..(written as usize).min(buf.len())]
    };
    String::from_utf16_lossy(trimmed)
}

#[cfg(not(windows))]
pub fn expand_env_vars(input: &str) -> String {
    input.to_string()
}

/// 文字列を NUL 終端付き UTF-16 LE バイト列にエンコードする。
/// REG_EXPAND_SZ / REG_SZ の生バイト書き込み用。
pub(crate) fn encode_utf16_with_nul(s: &str) -> Vec<u8> {
    let utf16: Vec<u16> = s.encode_utf16().chain(std::iter::once(0u16)).collect();
    let mut bytes = Vec::with_capacity(utf16.len() * 2);
    for u in utf16 {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_utf16_with_nul_appends_null_terminator() {
        let bytes = encode_utf16_with_nul("AB");
        // A=0x41 B=0x42, terminator
        assert_eq!(bytes, vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn encode_utf16_with_nul_handles_japanese() {
        let bytes = encode_utf16_with_nul("あ");
        // あ = U+3042 → 0x42 0x30 (LE)、+ NUL terminator 0x00 0x00
        assert_eq!(bytes, vec![0x42, 0x30, 0x00, 0x00]);
    }
}
