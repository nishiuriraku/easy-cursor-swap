/**
 * useProfileBackup が export_profile / import_profile IPC を正しい引数で叩くことを検証する
 * (settings.vue の直 invoke 集約)。
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'

const invoke = vi.fn()
vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invoke(...args),
}))

import { useProfileBackup } from '../useProfileBackup'

describe('useProfileBackup', () => {
  beforeEach(() => invoke.mockReset())

  it('exportProfile は export_profile を path 付きで invoke する', async () => {
    invoke.mockResolvedValue(null)
    const { exportProfile } = useProfileBackup()
    await exportProfile('C:/x.cursorprofile')
    expect(invoke).toHaveBeenCalledWith('export_profile', { path: 'C:/x.cursorprofile' })
  })

  it('importProfile は import_profile を path/merge 付きで invoke し結果を返す', async () => {
    const env = { schema_version: 1, exported_at: '2026-06-03T00:00:00Z', app_version: '0.0.5' }
    invoke.mockResolvedValue(env)
    const { importProfile } = useProfileBackup()
    const r = await importProfile('C:/y.cursorprofile', true)
    expect(invoke).toHaveBeenCalledWith('import_profile', {
      path: 'C:/y.cursorprofile',
      merge: true,
    })
    expect(r).toEqual(env)
  })
})
