/**
 * Windows レジストリスキーム (HKCU\Cursors\Schemes) の IPC 集約。
 * pages/index.vue が直接 invoke していた 3 コマンド
 * (list_windows_schemes / apply_windows_scheme / export_windows_scheme_as_cursorpack)
 * をまとめる。useThemes.ts と同パターン (caller が refresh を管理)。
 */

/** `list_windows_schemes` のレスポンス。Windows レジストリ HKCU\Cursors\Schemes 由来。 */
export interface IpcWindowsScheme {
  name: string
  cursor_paths: Record<string, string>
  role_count: number
  /** Rust 側で `paths_match_current_registry` 判定済み。現在実態と一致するなら true。 */
  is_active?: boolean
}

export function useWindowsSchemes() {
  /** HKCU\Cursors\Schemes のスキーム一覧。失敗時は呼び出し側 catch、null は [] に正規化。 */
  async function listSchemes(): Promise<IpcWindowsScheme[]> {
    return (await invokeTauri<IpcWindowsScheme[]>('list_windows_schemes')) ?? []
  }
  /** 指定スキームを HKCU に適用する。 */
  async function applyScheme(name: string): Promise<void> {
    await invokeTauri<void>('apply_windows_scheme', { name })
  }
  /** スキームを `.cursorpack` に書き出す。生成サイズ (bytes) を返す。 */
  async function exportSchemeAsCursorpack(
    name: string,
    outputPath: string,
  ): Promise<number | null> {
    const result = await invokeTauri<{ theme_id: string; size_bytes: number }>(
      'export_windows_scheme_as_cursorpack',
      { name, outputPath },
    )
    return result?.size_bytes ?? null
  }
  return { listSchemes, applyScheme, exportSchemeAsCursorpack }
}
