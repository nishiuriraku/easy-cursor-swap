/**
 * Library の再読込シグナル購読 (P08a Step 4 / L6)。
 *
 * `cursor-changed` (Tauri listen) / `easycs:cursors-changed` (DOM) /
 * `focus` / `visibilitychange` / `watch(locale)` を登録し、
 * `onUnmounted` で解除する。
 */
import type { Ref } from 'vue'
import { listenTauri } from './useTauri'

export interface LibraryRefreshSignalsDeps {
  reload: (opts?: { silent?: boolean }) => unknown
  onBeforeReload?: () => void
  locale: Ref<string>
}

export function useLibraryRefreshSignals(deps: LibraryRefreshSignalsDeps) {
  const { reload, onBeforeReload, locale } = deps
  let unlistenCursorChange: (() => void) | null = null

  // 外部カーソル変更検知 — Rust 側で SPI_SETCURSORS を購読し、変更があれば UI 更新
  async function setupCursorChangeListener() {
    try {
      unlistenCursorChange = await listenTauri('cursor-changed', () => {
        console.info('[Library] cursor-changed event received → reload')
        onBeforeReload?.()
        void reload()
      })
    } catch (err) {
      console.warn('[Library] cursor-changed listener unavailable:', err)
    }
  }

  /**
   * 「テーマ状態が変わったかも」というシグナルを 3 経路から拾う。
   *
   * 1. `easycs:cursors-changed` (DOM CustomEvent): default.vue の PanicFlow done フック
   *    と同じウィンドウから dispatch される。Tauri の listen に依らない確実経路。
   * 2. `focus` (window): 別ウィンドウやコントロールパネルでカーソルを変更後に
   *    EasyCursorSwap へ戻ってきたタイミング。
   * 3. `visibilitychange` (document): タブ非表示 → 表示時。focus と相補。
   *
   * いずれもデバウンスせずそのまま `reload` を呼ぶ。`get_themes` 自体が
   * in-flight 共有しているので連発しても安全。
   */
  function onExternalCursorsMaybeChanged() {
    void reload()
  }
  function onWindowFocus() {
    void reload()
  }
  function onVisibilityChange() {
    if (typeof document === 'undefined') return
    if (document.visibilityState === 'visible') void reload()
  }

  function start() {
    if (typeof window !== 'undefined') {
      window.addEventListener('easycs:cursors-changed', onExternalCursorsMaybeChanged)
      window.addEventListener('focus', onWindowFocus)
    }
    if (typeof document !== 'undefined') {
      document.addEventListener('visibilitychange', onVisibilityChange)
    }
  }

  function stop() {
    if (unlistenCursorChange) {
      unlistenCursorChange()
      unlistenCursorChange = null
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('easycs:cursors-changed', onExternalCursorsMaybeChanged)
      window.removeEventListener('focus', onWindowFocus)
    }
    if (typeof document !== 'undefined') {
      document.removeEventListener('visibilitychange', onVisibilityChange)
    }
  }

  watch(locale, () => {
    void reload({ silent: true })
  })

  onMounted(async () => {
    await setupCursorChangeListener()
    start()
  })
  onUnmounted(stop)

  return {
    setupCursorChangeListener,
    onExternalCursorsMaybeChanged,
    onWindowFocus,
    onVisibilityChange,
    start,
    stop,
  }
}
