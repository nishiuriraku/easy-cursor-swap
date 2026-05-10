import { takePendingCursorpack } from './useTauri'

/**
 * `.cursorpack` のインポート要求を Rust 側から受け取り、ハンドラに引き渡す composable。
 *
 * 受信経路は 2 つあり、どちらが先に届いても 1 回しか処理しないよう
 * 直近の path を記憶して重複を抑止する:
 *  1. mount 直後の `takePendingCursorpack` IPC (起動時 argv 由来)
 *  2. `cursorpack-import-requested` event (2 重起動 callback 由来)
 */
export function useCursorpackOpener(onPath: (path: string) => void) {
  let lastHandled: string | null = null
  let unlisten: (() => void) | null = null

  function dispatch(path: string) {
    if (path === lastHandled) return
    lastHandled = path
    onPath(path)
  }

  async function start() {
    // 1) 起動時 argv のプル
    try {
      const initial = await takePendingCursorpack()
      if (initial) dispatch(initial)
    } catch (e) {
      console.warn('[cursorpack-opener] take failed:', e)
    }

    // 2) event 購読
    try {
      const { listen } = await import('@tauri-apps/api/event')
      unlisten = await listen<string>('cursorpack-import-requested', (e) => {
        if (typeof e.payload === 'string' && e.payload.length > 0) {
          dispatch(e.payload)
        }
      })
    } catch (e) {
      console.warn('[cursorpack-opener] event listen failed:', e)
    }
  }

  async function stop() {
    if (unlisten) {
      unlisten()
      unlisten = null
    }
  }

  return { start, stop }
}
