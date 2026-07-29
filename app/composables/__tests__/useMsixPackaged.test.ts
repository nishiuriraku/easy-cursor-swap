import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

// 動的 import で各テスト前にモジュールを re-evaluate し、module-level singleton を reset。
let invokeTauriMock = vi.fn<(...args: unknown[]) => Promise<unknown>>()

vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invokeTauriMock(...args),
}))

async function importFresh() {
  vi.resetModules()
  const mod = await import('../useMsixPackaged')
  return mod.useMsixPackaged
}

describe('useMsixPackaged', () => {
  beforeEach(() => {
    invokeTauriMock = vi.fn<(...args: unknown[]) => Promise<unknown>>()
  })

  afterEach(() => {
    vi.resetModules()
  })

  it('IPC が true を返したら isMsixPackaged は true', async () => {
    invokeTauriMock.mockResolvedValue(true)
    const useMsixPackaged = await importFresh()
    const { isMsixPackaged } = useMsixPackaged()
    for (let i = 0; i < 5; i++) {
      await Promise.resolve()
    }
    expect(invokeTauriMock).toHaveBeenCalledWith('is_msix_packaged')
    expect(isMsixPackaged.value).toBe(true)
  })

  it('IPC が false を返したら isMsixPackaged は false', async () => {
    invokeTauriMock.mockResolvedValue(false)
    const useMsixPackaged = await importFresh()
    const { isMsixPackaged } = useMsixPackaged()
    for (let i = 0; i < 5; i++) {
      await Promise.resolve()
    }
    expect(isMsixPackaged.value).toBe(false)
  })

  it('IPC が失敗したら安全側で false にフォールバック', async () => {
    invokeTauriMock.mockRejectedValue(new Error('IPC down'))
    const useMsixPackaged = await importFresh()
    const { isMsixPackaged } = useMsixPackaged()
    for (let i = 0; i < 5; i++) {
      await Promise.resolve()
    }
    expect(isMsixPackaged.value).toBe(false)
  })
})
