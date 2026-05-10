import { beforeEach, describe, expect, it, vi } from 'vitest'

// useTauri の takePendingCursorpack をモック
const takePending = vi.fn<() => Promise<string | null>>()
vi.mock('../useTauri', () => ({
  takePendingCursorpack: () => takePending(),
  invokeTauri: vi.fn(),
}))

// Tauri event API をモック
const eventListeners = new Map<string, (e: { payload: string }) => void>()
const unlistenSpy = vi.fn()
vi.mock('@tauri-apps/api/event', () => ({
  listen: (name: string, cb: (e: { payload: string }) => void) => {
    eventListeners.set(name, cb)
    return Promise.resolve(unlistenSpy)
  },
}))

import { useCursorpackOpener } from '../useCursorpackOpener'

describe('useCursorpackOpener', () => {
  beforeEach(() => {
    takePending.mockReset()
    eventListeners.clear()
    unlistenSpy.mockReset()
  })

  it('mount 直後に take してハンドラを呼ぶ', async () => {
    takePending.mockResolvedValue('C:/themes/foo.cursorpack')
    const onPath = vi.fn()
    const opener = useCursorpackOpener(onPath)
    await opener.start()
    expect(onPath).toHaveBeenCalledWith('C:/themes/foo.cursorpack')
  })

  it('event 受信でもハンドラを呼ぶ', async () => {
    takePending.mockResolvedValue(null)
    const onPath = vi.fn()
    const opener = useCursorpackOpener(onPath)
    await opener.start()
    const cb = eventListeners.get('cursorpack-import-requested')
    expect(cb).toBeDefined()
    cb?.({ payload: 'C:/themes/bar.cursorpack' })
    expect(onPath).toHaveBeenCalledWith('C:/themes/bar.cursorpack')
  })

  it('同じパスを 2 回受け取っても 1 回しか処理しない', async () => {
    takePending.mockResolvedValue('C:/themes/dup.cursorpack')
    const onPath = vi.fn()
    const opener = useCursorpackOpener(onPath)
    await opener.start()
    const cb = eventListeners.get('cursorpack-import-requested')
    cb?.({ payload: 'C:/themes/dup.cursorpack' })
    expect(onPath).toHaveBeenCalledTimes(1)
  })

  it('stop で listener を解除する', async () => {
    takePending.mockResolvedValue(null)
    const opener = useCursorpackOpener(vi.fn())
    await opener.start()
    await opener.stop()
    expect(unlistenSpy).toHaveBeenCalledTimes(1)
  })
})
