/**
 * MSIX パッケージ環境検出 composable (Wave 4B)
 *
 * 起動時に `is_msix_packaged` IPC を 1 度だけ呼び出し、結果をリアクティブに提供する。
 * MSIX 環境では Tauri Updater / 自動起動レジストリ書込みが no-op になるため、
 * 設定 UI の Updates / Startup セクションで「Store 側で制御」等の案内文を
 * 出し分けたい。
 *
 * - 失敗時 (Tauri ランタイム未接続 / dev モード) は安全側で `false` を返す。
 * - 1 度フェッチしたら以降はキャッシュして再呼び出ししない (singleton)。
 */

const isMsixRef = ref<boolean>(false)
let fetched = false


export function useMsixPackaged(): { isMsixPackaged: Readonly<Ref<boolean>> } {
  if (!fetched) {
    fetched = true
    void (async () => {
      try {
        const v = await invokeTauri<boolean>('is_msix_packaged')
        isMsixRef.value = v
      } catch (e) {
        // 失敗時は unpackaged 想定
        console.warn('[useMsixPackaged] is_msix_packaged IPC 失敗、unpackaged として扱う:', e)
        isMsixRef.value = false
      }
    })()
  }
  return { isMsixPackaged: readonly(isMsixRef) }
}
