/**
 * useProfileBackup が export_profile / import_profile IPC を正しい引数で叩くことを検証する
 * (settings.vue の直 invoke 集約)。
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'

const invoke = vi.fn()
vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invoke(...args),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
  open: vi.fn(),
  ask: vi.fn(),
}))

import { useProfileBackup, useProfileBackupDialog } from '../useProfileBackup'

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

describe('useProfileBackupDialog (P08a S6)', () => {
  const t = (key: string) => key

  beforeEach(() => {
    invoke.mockReset()
    vi.clearAllMocks()
  })

  it('save が null で何もしない', async () => {
    const { save, open, ask } = await import('@tauri-apps/plugin-dialog')
    vi.mocked(save).mockResolvedValueOnce(null)
    const s = useProfileBackupDialog({ t })
    await s.exportWithDialog()
    expect(invoke).not.toHaveBeenCalledWith('export_profile', expect.anything())
    expect(s.message.value).toBeNull()
    expect(s.busy.value).toBe(false)
    void open
    void ask
  })

  it('成功で profileExportSuccess 文言', async () => {
    const { save } = await import('@tauri-apps/plugin-dialog')
    vi.mocked(save).mockResolvedValueOnce('/tmp/x.cursorprofile')
    invoke.mockResolvedValueOnce(null)
    const s = useProfileBackupDialog({ t })
    await s.exportWithDialog()
    expect(s.message.value).toBe('settings.profileExportSuccess')
  })
})
