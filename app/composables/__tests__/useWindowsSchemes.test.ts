/**
 * useWindowsSchemes が windows_scheme 系 IPC を正しい引数で叩くことを検証する
 * (pages/index.vue の直 invoke 集約)。
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'

const invoke = vi.fn()
vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invoke(...args),
}))

import { useWindowsSchemes } from '../useWindowsSchemes'

describe('useWindowsSchemes', () => {
  beforeEach(() => invoke.mockReset())

  it('listSchemes は list_windows_schemes を invoke し結果を返す', async () => {
    const schemes = [{ name: 'X', cursor_paths: {}, role_count: 0 }]
    invoke.mockResolvedValue(schemes)
    const { listSchemes } = useWindowsSchemes()
    const r = await listSchemes()
    expect(invoke).toHaveBeenCalledWith('list_windows_schemes')
    expect(r).toEqual(schemes)
  })

  it('listSchemes は null を [] に正規化する', async () => {
    invoke.mockResolvedValue(null)
    const { listSchemes } = useWindowsSchemes()
    expect(await listSchemes()).toEqual([])
  })

  it('applyScheme は apply_windows_scheme を name 付きで invoke する', async () => {
    invoke.mockResolvedValue(undefined)
    const { applyScheme } = useWindowsSchemes()
    await applyScheme('Mint')
    expect(invoke).toHaveBeenCalledWith('apply_windows_scheme', { name: 'Mint' })
  })

  it('exportSchemeAsCursorpack は export_windows_scheme_as_cursorpack を呼び size_bytes を返す', async () => {
    invoke.mockResolvedValue({ theme_id: 'windows:Mint', size_bytes: 1234 })
    const { exportSchemeAsCursorpack } = useWindowsSchemes()
    const r = await exportSchemeAsCursorpack('Mint', 'C:/out.cursorpack')
    expect(invoke).toHaveBeenCalledWith('export_windows_scheme_as_cursorpack', {
      name: 'Mint',
      outputPath: 'C:/out.cursorpack',
    })
    expect(r).toBe(1234)
  })
})
