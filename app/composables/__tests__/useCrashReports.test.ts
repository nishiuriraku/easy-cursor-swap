/**
 * useCrashReports が crash 系 IPC を正しいコマンド名で叩き結果を返すことを検証する
 * (settings.vue の直 invoke 集約)。
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'

const invoke = vi.fn()
vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invoke(...args),
}))

import { useCrashReports } from '../useCrashReports'

describe('useCrashReports', () => {
  beforeEach(() => invoke.mockReset())

  it('listCrashReports は list_crash_reports を invoke し結果を返す', async () => {
    invoke.mockResolvedValue([{}, {}])
    const { listCrashReports } = useCrashReports()
    const r = await listCrashReports()
    expect(invoke).toHaveBeenCalledWith('list_crash_reports')
    expect(r).toEqual([{}, {}])
  })

  it('submitCrashReports は submit_crash_reports を invoke し summary を返す', async () => {
    const summary = { sent: 1, failed: 0, skipped: 2 }
    invoke.mockResolvedValue(summary)
    const { submitCrashReports } = useCrashReports()
    expect(await submitCrashReports()).toEqual(summary)
    expect(invoke).toHaveBeenCalledWith('submit_crash_reports')
  })

  it('clearCrashReports は clear_crash_reports を invoke し削除件数を返す', async () => {
    invoke.mockResolvedValue(3)
    const { clearCrashReports } = useCrashReports()
    expect(await clearCrashReports()).toBe(3)
    expect(invoke).toHaveBeenCalledWith('clear_crash_reports')
  })
})
