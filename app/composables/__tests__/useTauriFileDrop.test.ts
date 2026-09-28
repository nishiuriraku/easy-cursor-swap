/**
 * useTauriFileDrop のテスト (P08a Step 1)。
 *
 * `@tauri-apps/api/window` をモックし、enter/over/leave/drop の 4 payload を
 * 流して `showDrop` と `onDrop` / `onRejected` の呼び出し、`stop()` での
 * unlisten を検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'

type DropCallback = (event: {
  payload: { type: 'enter' | 'over' | 'leave' } | { type: 'drop'; paths: string[] }
}) => void

let captured: DropCallback | null = null
const unlistenMock = vi.fn()

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    onDragDropEvent: (cb: DropCallback) => {
      captured = cb
      return Promise.resolve(unlistenMock)
    },
  }),
}))

import { useTauriFileDrop } from '../useTauriFileDrop'

beforeEach(() => {
  captured = null
  unlistenMock.mockClear()
})

describe('useTauriFileDrop', () => {
  it('shows overlay on enter/over and hides on leave', async () => {
    const { showDrop, start } = useTauriFileDrop({ accept: [], onDrop: () => {} })
    await start()
    expect(captured).not.toBeNull()
    captured!({ payload: { type: 'enter' } })
    expect(showDrop.value).toBe(true)
    captured!({ payload: { type: 'over' } })
    expect(showDrop.value).toBe(true)
    captured!({ payload: { type: 'leave' } })
    expect(showDrop.value).toBe(false)
  })

  it('calls onDrop with accepted paths and hides overlay', async () => {
    const onDrop = vi.fn()
    const onRejected = vi.fn()
    const { showDrop, start } = useTauriFileDrop({
      accept: ['cursorpack'],
      onDrop,
      onRejected,
    })
    await start()
    captured!({ payload: { type: 'enter' } })
    captured!({
      payload: { type: 'drop', paths: ['/a/b.cursorpack', '/c/d.png'] },
    })
    expect(showDrop.value).toBe(false)
    expect(onDrop).toHaveBeenCalledTimes(1)
    expect(onDrop).toHaveBeenCalledWith(['/a/b.cursorpack'])
    expect(onRejected).not.toHaveBeenCalled()
  })

  it('calls onRejected when no path matches accept', async () => {
    const onDrop = vi.fn()
    const onRejected = vi.fn()
    const { start } = useTauriFileDrop({
      accept: ['png', 'svg'],
      onDrop,
      onRejected,
    })
    await start()
    captured!({ payload: { type: 'drop', paths: ['/a/b.txt'] } })
    expect(onDrop).not.toHaveBeenCalled()
    expect(onRejected).toHaveBeenCalledTimes(1)
  })

  it('matches extensions case-insensitively', async () => {
    const onDrop = vi.fn()
    const { start } = useTauriFileDrop({ accept: ['cur'], onDrop })
    await start()
    captured!({ payload: { type: 'drop', paths: ['/a/B.CUR'] } })
    expect(onDrop).toHaveBeenCalledWith(['/a/B.CUR'])
  })

  it('stop() calls unlisten and is idempotent', async () => {
    const { start, stop } = useTauriFileDrop({ accept: [], onDrop: () => {} })
    await start()
    stop()
    expect(unlistenMock).toHaveBeenCalledTimes(1)
    stop()
    expect(unlistenMock).toHaveBeenCalledTimes(1)
  })
})
