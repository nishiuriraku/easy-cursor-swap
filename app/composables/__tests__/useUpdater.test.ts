import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

// plugin-updater と useTauri のモック。MSIX 短絡経路を確実に見るため、
// plugin-updater の check / downloadAndInstall が「呼ばれないこと」を assert する。
const invokeTauriMock = vi.fn<(...args: unknown[]) => Promise<unknown>>()
vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invokeTauriMock(...args),
}))

const pluginCheckMock =
  vi.fn<() => Promise<{ available?: boolean; version?: string; currentVersion?: string } | null>>()
const pluginDownloadAndInstallMock = vi.fn<() => Promise<boolean>>()
vi.mock('@tauri-apps/plugin-updater', () => ({
  check: (..._args: unknown[]) => pluginCheckMock(),
  downloadAndInstall: (..._args: unknown[]) => pluginDownloadAndInstallMock(),
}))

import { useUpdater } from '../useUpdater'

async function flushMicrotasks(times = 5) {
  for (let i = 0; i < times; i++) {
    await Promise.resolve()
  }
}

describe('useUpdater (MSIX short-circuit)', () => {
  beforeEach(() => {
    invokeTauriMock.mockReset()
    pluginCheckMock.mockReset()
    pluginDownloadAndInstallMock.mockReset()
  })

  afterEach(() => {
    vi.resetModules()
  })

  it('MSIX=true なら check は plugin-updater.check を呼ばず null を返す', async () => {
    invokeTauriMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'is_msix_packaged') return true
      return null
    })
    const { check } = useUpdater()
    const result = await check()
    expect(invokeTauriMock).toHaveBeenCalledWith('is_msix_packaged')
    expect(pluginCheckMock).not.toHaveBeenCalled()
    expect(result).toBeNull()
  })

  it('MSIX=false なら check は通常経路 (plugin-updater.check を呼ぶ)', async () => {
    invokeTauriMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'is_msix_packaged') return false
      return null
    })
    pluginCheckMock.mockResolvedValue({
      available: true,
      version: '0.2.0',
      currentVersion: '0.1.0',
    })
    const { check, available } = useUpdater()
    const result = await check()
    expect(pluginCheckMock).toHaveBeenCalledTimes(1)
    expect(result?.version).toBe('0.2.0')
    expect(available.value?.version).toBe('0.2.0')
  })

  it('is_msix_packaged IPC 失敗時は unpackaged 想定で plugin-updater.check を呼ぶ', async () => {
    invokeTauriMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'is_msix_packaged') throw new Error('IPC down')
      return null
    })
    pluginCheckMock.mockResolvedValue(null)
    const { check } = useUpdater()
    const result = await check()
    expect(pluginCheckMock).toHaveBeenCalledTimes(1)
    expect(result).toBeNull()
  })

  it('MSIX=true なら downloadAndInstall は plugin-updater を呼ばず false を返す', async () => {
    invokeTauriMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'is_msix_packaged') return true
      return null
    })
    const { downloadAndInstall } = useUpdater()
    const result = await downloadAndInstall()
    expect(pluginCheckMock).not.toHaveBeenCalled()
    expect(pluginDownloadAndInstallMock).not.toHaveBeenCalled()
    expect(result).toBe(false)
  })

  it('MSIX=false + update なしなら downloadAndInstall は false (早期 return)', async () => {
    invokeTauriMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'is_msix_packaged') return false
      return null
    })
    pluginCheckMock.mockResolvedValue(null)
    const { downloadAndInstall } = useUpdater()
    const result = await downloadAndInstall()
    expect(pluginCheckMock).toHaveBeenCalledTimes(1)
    expect(pluginDownloadAndInstallMock).not.toHaveBeenCalled()
    expect(result).toBe(false)
  })
})
