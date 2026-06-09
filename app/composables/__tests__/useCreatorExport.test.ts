/**
 * useCreatorExport: 事前バリデーション / エラーメッセージが i18n (t()) を通ることを検証する。
 * 以前は 4 箇所が日本語ベタ書きで、英語 UI でも日本語が出ていた (R5)。
 * identity な t (キーをそのまま返す) を注入し「t() を通したか」をキー一致で確認する。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { ref } from 'vue'

const invoke = vi.fn()
vi.mock('../useTauri', () => ({ invokeTauri: (...a: unknown[]) => invoke(...a) }))
vi.mock('../useThemePreviews', () => ({ useThemePreviews: () => ({ invalidate: vi.fn() }) }))
const applyTheme = vi.fn()
vi.mock('../useThemes', () => ({ useThemes: () => ({ applyTheme }) }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }))

import { useCreatorExport, type CreatorExportDeps } from '../useCreatorExport'

const identityT = (key: string): string => key

function makeDeps(roleCount: number, arrow: boolean): CreatorExportDeps {
  return {
    creatorAssets: {
      assignedRoleCount: ref(roleCount),
      arrowAssigned: ref(arrow),
      toExportPayload: () => [],
    },
    metaNameEn: ref(''),
    metaAuthor: ref(''),
    metaVersion: ref('1.0.0'),
    metaDescription: ref(''),
    sourceThemeId: ref<string | null>(null),
    shadowEnabled: ref(false),
    resample: ref<'lanczos' | 'nearest'>('lanczos'),
    t: identityT,
  }
}

const payload = {
  destination: 'library' as const,
  filePath: null as string | null,
  effectiveName: 'X',
  sign: false,
  overwriteExisting: false,
}

describe('useCreatorExport i18n (R5)', () => {
  beforeEach(() => {
    invoke.mockReset()
    applyTheme.mockReset()
  })

  it('役割未割当は saveModal.validateRoleRequired キーで t() を通す', async () => {
    const ex = useCreatorExport(makeDeps(0, false))
    const status = await ex.executeSave(payload)
    expect(status).toBe('failed')
    expect(ex.exportMessage.value).toBe('saveModal.validateRoleRequired')
  })

  it('Arrow 未割当は saveModal.validateArrowRequired キーで t() を通す', async () => {
    const ex = useCreatorExport(makeDeps(1, false))
    const status = await ex.executeSave(payload)
    expect(status).toBe('failed')
    expect(ex.exportMessage.value).toBe('saveModal.validateArrowRequired')
  })

  it('IPC 例外時は saveModal.toastExportFailed キーで t() を通す', async () => {
    invoke.mockRejectedValue(new Error('boom'))
    const ex = useCreatorExport(makeDeps(5, true))
    const status = await ex.executeSave(payload)
    expect(status).toBe('failed')
    expect(ex.exportMessage.value).toContain('saveModal.toastExportFailed')
  })

  it('retryApply 失敗時は saveModal.toastRetryFailed キーで t() を通す', async () => {
    applyTheme.mockRejectedValue(new Error('nope'))
    const ex = useCreatorExport(makeDeps(5, true))
    ex.failedApplyThemeId.value = 'theme-1'
    await ex.retryApply()
    expect(ex.exportMessage.value).toContain('saveModal.toastRetryFailed')
  })
})
