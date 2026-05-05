/**
 * 通知 (層 2: Windows Toast) ヘルパー。
 *
 * 仕様書 §「3 層通知」:
 *  - 層 1: 無通知 (バックグラウンド適用成功など)
 *  - 層 2: Windows Toast 通知 (本 composable)
 *  - 層 3: アプリ内モーダル
 *
 * `tauri-plugin-notification` 経由で `ToastNotificationManager` を呼び出す。
 * 起動時に一度だけ `requestPermission` し、結果をキャッシュして以降の API 呼出を省く。
 *
 * Web 開発環境 (Tauri 未接続) では console にフォールバック。
 */

let permissionGranted: boolean | null = null

async function ensurePermission(): Promise<boolean> {
  if (permissionGranted !== null) return permissionGranted
  try {
    const mod = await import('@tauri-apps/plugin-notification')
    let granted = await mod.isPermissionGranted()
    if (!granted) {
      const result = await mod.requestPermission()
      granted = result === 'granted'
    }
    permissionGranted = granted
    return granted
  } catch {
    permissionGranted = false
    return false
  }
}

export interface NotifyOptions {
  title: string
  body?: string
  /** 'info' / 'success' / 'warn' / 'error' — 現状アイコン表示には未使用、ログ用 */
  level?: 'info' | 'success' | 'warn' | 'error'
}

export async function notify(opts: NotifyOptions): Promise<void> {
  const ok = await ensurePermission()
  if (!ok) {
    console.info(`[notify:${opts.level ?? 'info'}] ${opts.title}`, opts.body ?? '')
    return
  }
  try {
    const { sendNotification } = await import('@tauri-apps/plugin-notification')
    sendNotification({
      title: opts.title,
      body: opts.body,
    })
  } catch (err) {
    console.warn('[notify] sendNotification failed:', err)
  }
}

export function useNotify() {
  return { notify }
}
