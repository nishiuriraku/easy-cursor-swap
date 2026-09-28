/**
 * useUpdaterSettings のテスト (P08a Step 3 / S4)。
 *
 * `useUpdater` / `useAppInfo` / dialog / invoke をモックし、5 分岐
 * (新版あり/最新/エラー/major-jump 拒否/force-recheck) を検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'

const checkMock = vi.fn()
const downloadUpdateMock = vi.fn()
const relaunchAppMock = vi.fn()
const updaterState = {
  checking: { value: false },
  downloading: { value: false },
  available: { value: null },
  error: { value: null as string | null },
  progressBytes: { value: 0 },
  totalBytes: { value: 0 },
}
const loadMock = vi.fn()
const askMock = vi.fn()
const invokeTauriMock = vi.fn()

vi.mock('../useUpdater', () => ({
  useUpdater: () => ({
    checking: updaterState.checking,
    downloading: updaterState.downloading,
    available: updaterState.available,
    error: updaterState.error,
    progressBytes: updaterState.progressBytes,
    totalBytes: updaterState.totalBytes,
    check: checkMock,
    downloadAndInstall: downloadUpdateMock,
    relaunch: relaunchAppMock,
  }),
  classifyUpdaterError: (err: unknown) => ({
    key: 'updater.classified',
    message: String(err),
  }),
}))

vi.mock('../useAppInfo', () => ({
  useAppInfo: () => ({ load: loadMock }),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  ask: (...args: unknown[]) => askMock(...args),
}))

vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invokeTauriMock(...args),
}))

import { useUpdaterSettings } from '../useUpdaterSettings'

const t = (key: string) => key

beforeEach(() => {
  vi.clearAllMocks()
  updaterState.error.value = null
  updaterState.available.value = null
  localStorage.clear()
})

describe('useUpdaterSettings', () => {
  it('shows new-version message when check finds info', async () => {
    checkMock.mockResolvedValueOnce({ version: '0.0.9', currentVersion: '0.0.8' })
    const s = useUpdaterSettings({ t })
    await s.onCheckUpdate()
    expect(s.updaterMessage.value).toBe('settings.updateNewVersion')
  })

  it('shows up-to-date when check returns null without error', async () => {
    checkMock.mockResolvedValueOnce(null)
    const s = useUpdaterSettings({ t })
    await s.onCheckUpdate()
    expect(s.updaterMessage.value).toBe('settings.updateUpToDate')
  })

  it('keeps message null when check fails with error', async () => {
    checkMock.mockResolvedValueOnce(null)
    updaterState.error.value = 'boom'
    const s = useUpdaterSettings({ t })
    await s.onCheckUpdate()
    expect(s.updaterMessage.value).toBeNull()
  })

  it('skips download when major-jump dialog is declined', async () => {
    checkMock.mockResolvedValueOnce({ version: '1.0.0', currentVersion: '0.0.8' })
    loadMock.mockResolvedValueOnce({ version: '0.0.8' })
    invokeTauriMock.mockResolvedValueOnce(true)
    askMock.mockResolvedValueOnce(false)
    const s = useUpdaterSettings({ t })
    await s.onCheckUpdate()
    await s.onDownloadUpdate()
    expect(downloadUpdateMock).not.toHaveBeenCalled()
  })

  it('onForceRecheck resets cooldown marker', () => {
    const s = useUpdaterSettings({ t })
    localStorage.setItem('x', '1')
    s.onForceRecheck()
    expect(s.updaterMessage.value).toBe('settings.autoCheckHintReady')
  })
})
