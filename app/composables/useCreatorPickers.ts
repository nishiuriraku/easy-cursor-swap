/**
 * Tauri plugin-dialog の薄いラッパ。
 *
 * Creator から呼ばれるファイル/フォルダピッカーを集約。dialog の動的 import を
 * 各呼び出し元で書く重複を避ける。テスト時はこの composable を spy / mock すれば
 * Tauri 依存を簡単に差し替えられる。
 */

const ALL_ASSET_EXTENSIONS = ['png', 'svg', 'cur', 'ico', 'ani', 'cursorpack']

export function useCreatorPickers() {
  /**
   * Creator のメイン取込ダイアログ。
   * PNG/SVG/CUR/ICO/ANI/.cursorpack を複数選択でまとめて受け付ける。
   */
  async function pickAssetFiles(): Promise<string[] | null> {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const picked = await open({
      multiple: true,
      filters: [{ name: 'Cursor assets / pack', extensions: ALL_ASSET_EXTENSIONS }],
    })
    if (!picked) return null
    return Array.isArray(picked) ? picked : [picked]
  }

  /** フォルダピッカー (再帰取込用)。 */
  async function pickFolder(): Promise<string | null> {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const picked = await open({ directory: true })
    if (!picked || typeof picked !== 'string') return null
    return picked
  }

  return { pickAssetFiles, pickFolder }
}
