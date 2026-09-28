/**
 * Tauri v2 ウィンドウドラッグ&ドロップ購読の共有 composable。
 *
 * ブラウザの DragEvent は dataTransfer.files に絶対パスを含めないため、
 * Library / Creator 両画面とも Tauri の onDragDropEvent で実パスを受け取る。
 * 拡張子フィルタを `accept` で共通化し、受理/拒否の分岐は呼び出し側の
 * `onDrop` / `onRejected` に委ねる (P08a Step 1)。
 */

export interface TauriFileDropOptions {
  /** 受け付ける拡張子 (小文字、ドット無し)。空なら全て受理。 */
  accept: readonly string[]
  /** 受理パスが 1 件以上あるとき。 */
  onDrop: (paths: string[]) => void
  /** ドロップされたが受理拡張子が 0 件だったとき。 */
  onRejected?: () => void
}

export function useTauriFileDrop(opts: TauriFileDropOptions) {
  const showDrop = ref(false)
  let unlisten: (() => void) | null = null

  async function start() {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      const win = getCurrentWindow()
      unlisten = await win.onDragDropEvent((event) => {
        const p = event.payload
        if (p.type === 'enter' || p.type === 'over') {
          showDrop.value = true
        } else if (p.type === 'leave') {
          showDrop.value = false
        } else if (p.type === 'drop') {
          showDrop.value = false
          const paths = (p.paths ?? []).filter((path: string) => {
            if (opts.accept.length === 0) return true
            const ext = path.toLowerCase().split('.').pop() ?? ''
            return opts.accept.includes(ext)
          })
          if (paths.length === 0) {
            opts.onRejected?.()
            return
          }
          opts.onDrop(paths)
        }
      })
    } catch (err) {
      console.warn('[useTauriFileDrop] Tauri drop API unavailable:', err)
    }
  }

  function stop() {
    if (unlisten) {
      unlisten()
      unlisten = null
    }
  }

  onUnmounted(stop)

  return { showDrop, start, stop }
}
