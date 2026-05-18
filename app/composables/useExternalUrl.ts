/**
 * 外部 URL をシステムのデフォルトブラウザで開く小さな composable。
 *
 * - 第一選択: Rust 側 `open_url` IPC (Win32 ShellExecuteW 経由)。`http(s)://`
 *   許可リストで scheme 検証済み。
 * - フォールバック: Tauri 未接続 (nuxt dev preview / vitest 環境) では
 *   `window.open(url, '_blank', 'noopener,noreferrer')`。
 *
 * 経緯: AboutSection / OssLicenseModal / SubmitDeviceFlowModal /
 * marketplace.vue / SubmitThemeDialog (×2) / ThemeDetailDrawer の合計
 * 6 箇所で同型の try/catch 重複があった (audit B10-related & D26 横断)。
 */

/**
 * 指定 URL をシステムブラウザで開く。Tauri 未接続時は `window.open` にフォールバックする。
 * 戻り値はなし。失敗しても throw しない (UI フローを止めない設計)。
 */
export async function openExternalUrl(url: string): Promise<void> {
  if (!url) return
  try {
    await invokeTauri<void>('open_url', { url })
  } catch (e) {
    if (typeof window !== 'undefined') {
      window.open(url, '_blank', 'noopener,noreferrer')
    } else {
      console.warn('[useExternalUrl] open_url failed and no window fallback:', e)
    }
  }
}

/** 関数のみ提供する composable 風 API (利用側で他の composable と合わせやすい)。 */
export function useExternalUrl() {
  return { openExternalUrl }
}
