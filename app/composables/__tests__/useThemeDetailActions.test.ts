/**
 * useThemeDetailActions のテスト (P08a Step 4 / L2)。
 *
 * `useThemes` / `useWindowsSchemes` / `useThemePreviews` をモックし、
 * delete ガード・confirm 拒否・system export 経路・showDetails 並列取得を検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { useThemeDetailActions } from '../useThemeDetailActions'

const repackageThemeMock = vi.fn()
const duplicateThemeMock = vi.fn()
const deleteThemeMock = vi.fn()
const exportSchemeMock = vi.fn()
const getMapMock = vi.fn()
const getDetailsMock = vi.fn()

vi.mock('~/composables/useThemes', () => ({
  useThemes: () => ({
    repackageTheme: repackageThemeMock,
    duplicateTheme: duplicateThemeMock,
    deleteTheme: deleteThemeMock,
  }),
}))

vi.mock('~/composables/useWindowsSchemes', () => ({
  useWindowsSchemes: () => ({ exportSchemeAsCursorpack: exportSchemeMock }),
}))

vi.mock('~/composables/useThemePreviews', () => ({
  useThemePreviews: () => ({ getMap: getMapMock, getDetails: getDetailsMock }),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
  open: vi.fn(),
  ask: vi.fn(),
}))

const t = (key: string) => key

function card(partial: Partial<ThemeCardData> & { id: string }): ThemeCardData {
  return {
    name: 'T',
    author: null,
    version: '1',
    date: '',
    applyCount: 0,
    isFavorite: false,
    isActive: false,
    includedRoles: [],
    kind: 'local',
    tags: [],
    sizeBytes: undefined,
    signed: false,
    lastAppliedAt: null,
    description: null,
    schemaVersion: undefined,
    license: null,
    homepage: null,
    ...partial,
  } as ThemeCardData
}

function setup(themes: ThemeCardData[]) {
  const themesRef = ref(themes)
  const reload = vi.fn(async () => {})
  const errors: Array<string | null> = []
  const requestApply = vi.fn()
  const actions = useThemeDetailActions({
    themes: themesRef,
    reload,
    setError: (m) => errors.push(m),
    requestApply,
    t,
  })
  return { themesRef, reload, errors, requestApply, actions }
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe('useThemeDetailActions', () => {
  it('deleteTheme refuses active themes without IPC', async () => {
    const { errors, actions } = setup([card({ id: 'a', isActive: true })])
    await actions.deleteTheme('a')
    expect(deleteThemeMock).not.toHaveBeenCalled()
    expect(errors).toEqual(['library.errDeleteActive'])
  })

  it('deleteTheme does nothing when confirm is false', async () => {
    Object.defineProperty(window, 'confirm', { value: () => false, configurable: true })
    const { actions } = setup([card({ id: 'a' })])
    await actions.deleteTheme('a')
    expect(deleteThemeMock).not.toHaveBeenCalled()
  })

  it('exportTheme uses scheme export for system kind', async () => {
    const { save } = await import('@tauri-apps/plugin-dialog')
    vi.mocked(save).mockResolvedValueOnce('/tmp/x.cursorpack')
    exportSchemeMock.mockResolvedValueOnce(123)
    const { actions } = setup([card({ id: 'w', kind: 'system', name: 'Win' })])
    await actions.exportTheme('w')
    expect(exportSchemeMock).toHaveBeenCalledWith('Win', '/tmp/x.cursorpack')
    expect(repackageThemeMock).not.toHaveBeenCalled()
  })

  it('exportTheme does nothing for unknown id', async () => {
    const { actions } = setup([card({ id: 'w', kind: 'system', name: 'Win' })])
    // dialog は happy-dom に無いため、target なし早期 return のみ確認
    await actions.exportTheme('missing')
    expect(exportSchemeMock).not.toHaveBeenCalled()
    expect(repackageThemeMock).not.toHaveBeenCalled()
  })

  it('showDetails loads map and details in parallel', async () => {
    getMapMock.mockResolvedValueOnce({ Arrow: 'blob:x' })
    getDetailsMock.mockResolvedValueOnce({ Arrow: { width: 32 } })
    const { actions } = setup([card({ id: 'a' })])
    await actions.showDetails('a')
    expect(actions.detailTheme.value?.id).toBe('a')
    expect(actions.detailPreviewMap.value).toEqual({ Arrow: 'blob:x' })
    expect(getMapMock).toHaveBeenCalledWith('a')
    expect(getDetailsMock).toHaveBeenCalledWith('a')
  })
})
