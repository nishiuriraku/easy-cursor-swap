/**
 * useLibraryRefreshSignals のテスト (P08a Step 4 / L6)。
 *
 * `listenTauri` をモックし、focus/custom event で reload が呼ばれ、
 * unmount 後に呼ばれないことを検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { ref } from 'vue'

const listenTauriMock = vi.fn()

vi.mock('../useTauri', () => ({
  listenTauri: (...args: unknown[]) => listenTauriMock(...args),
}))

import { useLibraryRefreshSignals } from '../useLibraryRefreshSignals'

beforeEach(() => {
  listenTauriMock.mockReset()
  listenTauriMock.mockResolvedValue(() => {})
})

describe('useLibraryRefreshSignals', () => {
  it('reloads on focus and custom event', () => {
    const reload = vi.fn()
    const locale = ref('ja')
    const signals = useLibraryRefreshSignals({ reload, locale })
    signals.start()
    window.dispatchEvent(new Event('focus'))
    expect(reload).toHaveBeenCalledTimes(1)
    window.dispatchEvent(new CustomEvent('easycs:cursors-changed'))
    expect(reload).toHaveBeenCalledTimes(2)
    signals.stop()
  })

  it('stop() removes listeners', () => {
    const reload = vi.fn()
    const locale = ref('ja')
    const signals = useLibraryRefreshSignals({ reload, locale })
    signals.start()
    signals.stop()
    window.dispatchEvent(new Event('focus'))
    expect(reload).not.toHaveBeenCalled()
  })

  it('calls onBeforeReload before reload on cursor-changed', async () => {
    let cb: (() => void) | null = null
    listenTauriMock.mockImplementationOnce((_ev: string, fn: () => void) => {
      cb = fn
      return Promise.resolve(() => {})
    })
    const reload = vi.fn()
    const onBeforeReload = vi.fn()
    const locale = ref('ja')
    const signals = useLibraryRefreshSignals({ reload, onBeforeReload, locale })
    // setupCursorChangeListener は onMounted 内 (テストでは未発火) のため直接呼ぶ
    await signals.setupCursorChangeListener()
    cb!()
    expect(onBeforeReload).toHaveBeenCalledTimes(1)
    expect(reload).toHaveBeenCalledTimes(1)
    signals.stop()
  })
})
