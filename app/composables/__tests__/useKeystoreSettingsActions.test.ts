/**
 * useKeystoreSettingsActions のテスト (P08a Step 3 / S3)。
 *
 * `useKeystore` と dialog をモックし、ask 分岐・export/import 文言を検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'

const generateMock = vi.fn()
const removeMock = vi.fn()
const exportPrivateMock = vi.fn()
const importPrivateMock = vi.fn()
const askMock = vi.fn()
const saveMock = vi.fn()
const openMock = vi.fn()

vi.mock('../useKeystore', () => ({
  useKeystore: () => ({
    info: { value: null },
    busy: { value: false },
    lastError: { value: null },
    refresh: vi.fn(),
    generate: generateMock,
    remove: removeMock,
    exportPrivate: exportPrivateMock,
    importPrivate: importPrivateMock,
  }),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  ask: (...args: unknown[]) => askMock(...args),
  save: (...args: unknown[]) => saveMock(...args),
  open: (...args: unknown[]) => openMock(...args),
}))

import { useKeystoreSettingsActions } from '../useKeystoreSettingsActions'

const t = (key: string) => key

beforeEach(() => {
  vi.clearAllMocks()
})

describe('useKeystoreSettingsActions', () => {
  it('onKeystoreGenerate clears message and generates without overwrite', async () => {
    const s = useKeystoreSettingsActions({ t })
    await s.onKeystoreGenerate()
    expect(generateMock).toHaveBeenCalledWith(false)
    expect(s.keystoreMessage.value).toBeNull()
  })

  it('onKeystoreRegenerate asks first and skips on cancel', async () => {
    askMock.mockResolvedValueOnce(false)
    const s = useKeystoreSettingsActions({ t })
    await s.onKeystoreRegenerate()
    expect(generateMock).not.toHaveBeenCalled()
    askMock.mockResolvedValueOnce(true)
    await s.onKeystoreRegenerate()
    expect(generateMock).toHaveBeenCalledWith(true)
  })

  it('export success shows keyExportSuccess message', async () => {
    saveMock.mockResolvedValueOnce('/tmp/k.cfkey')
    exportPrivateMock.mockResolvedValueOnce(12)
    const s = useKeystoreSettingsActions({ t })
    s.onKeystoreExport()
    expect(s.passphrasePrompt.value).toEqual({ mode: 'export', open: true })
    await s.onPassphraseConfirm('pw')
    expect(s.keystoreMessage.value).toBe('settings.keyExportSuccess')
  })

  it('import success shows key_id', async () => {
    openMock.mockResolvedValueOnce('/tmp/k.cfkey')
    importPrivateMock.mockResolvedValueOnce({ key_id: 'abc' })
    const s = useKeystoreSettingsActions({ t })
    s.onKeystoreImport()
    await s.onPassphraseConfirm('pw')
    expect(s.keystoreMessage.value).toBe('settings.keyImportSuccess')
  })

  it('onKeystoreDelete asks and removes on confirm', async () => {
    askMock.mockResolvedValueOnce(true)
    const s = useKeystoreSettingsActions({ t })
    await s.onKeystoreDelete()
    expect(removeMock).toHaveBeenCalledTimes(1)
  })
})
