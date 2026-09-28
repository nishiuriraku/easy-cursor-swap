/**
 * useCreatorStartFlow のテスト (P08a Step 2 / C5)。
 *
 * `useThemes().repackageTheme` と `@tauri-apps/api/path` をモックし、
 * 複製ピッカー選択時の temp path dispatch と `loadFromEditPath` の
 * `sourceThemeId` 設定を検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { ref } from 'vue'
import { useCreatorStartFlow } from '../useCreatorStartFlow'

const repackageTheme = vi.fn()
const dispatchBulkPaths = vi.fn()
const parseCursorpack = vi.fn()

vi.mock('../useThemes', () => ({
  useThemes: () => ({ repackageTheme }),
}))

vi.mock('@tauri-apps/api/path', () => ({
  tempDir: () => Promise.resolve('/tmp/'),
  sep: () => '/',
}))

const t = (key: string) => key

beforeEach(() => {
  vi.clearAllMocks()
})

function setup() {
  const stage = ref<'start' | 'editing'>('start')
  const newThemeModalOpen = ref(false)
  const themePickerOpen = ref(false)
  const themePickerSelected = ref<string | null>(null)
  const sourceThemeId = ref<string | null>(null)
  const saveModalDefault = ref<'file' | 'library' | 'libraryAndApply'>('file')
  const bulkModalOpen = ref(false)
  const bulkCursorpack = ref(null)
  const bulkResolved = ref(null)
  const bulkSourceLabel = ref('')
  const importMessage = ref<string | null>(null)
  const flow = useCreatorStartFlow({
    stage,
    newThemeModalOpen,
    themePickerOpen,
    themePickerSelected,
    sourceThemeId,
    saveModalDefault,
    bulkFlow: {
      bulkModalOpen,
      bulkCursorpack,
      bulkResolved,
      bulkSourceLabel,
      dispatchBulkPaths,
    },
    bulkImport: { parseCursorpack } as never,
    pickBulkAuto: vi.fn(),
    pickBulkFolder: vi.fn(),
    refreshPickerThemes: vi.fn(),
    importMessage,
    t,
  })
  return {
    stage,
    newThemeModalOpen,
    themePickerOpen,
    themePickerSelected,
    sourceThemeId,
    saveModalDefault,
    bulkModalOpen,
    bulkCursorpack,
    bulkSourceLabel,
    importMessage,
    flow,
  }
}

describe('useCreatorStartFlow', () => {
  it('onStartNew opens the modal', () => {
    const { newThemeModalOpen, flow } = setup()
    flow.onStartNew()
    expect(newThemeModalOpen.value).toBe(true)
  })

  it('onNewThemeStartEmpty closes modal and moves to editing', () => {
    const { stage, newThemeModalOpen, flow } = setup()
    newThemeModalOpen.value = true
    flow.onNewThemeStartEmpty()
    expect(newThemeModalOpen.value).toBe(false)
    expect(stage.value).toBe('editing')
  })

  it('onThemePickerSelect dispatches temp path and moves to editing', async () => {
    const { stage, themePickerOpen, flow } = setup()
    themePickerOpen.value = true
    await flow.onThemePickerSelect('theme-id')
    expect(themePickerOpen.value).toBe(false)
    expect(repackageTheme).toHaveBeenCalledTimes(1)
    const tempPath = repackageTheme.mock.calls[0]![1] as string
    expect(tempPath).toContain('_easycursorswap_dup_')
    expect(dispatchBulkPaths).toHaveBeenCalledWith([tempPath])
    // bulkModalOpen は false のままなので stage は start のまま
    expect(stage.value).toBe('start')
  })

  it('loadFromEditPath sets sourceThemeId from parsed metadata', async () => {
    const { stage, saveModalDefault, sourceThemeId, bulkSourceLabel, flow } = setup()
    parseCursorpack.mockResolvedValueOnce({ metadata: { id: 'src-uuid' } })
    await flow.loadFromEditPath('/tmp/x.cursorpack')
    expect(sourceThemeId.value).toBe('src-uuid')
    expect(saveModalDefault.value).toBe('libraryAndApply')
    expect(stage.value).toBe('editing')
    expect(bulkSourceLabel.value).toBe('creator.bulkSourceEditing')
  })

  it('loadFromEditPath reports failure message on parse error', async () => {
    const { stage, importMessage, flow } = setup()
    parseCursorpack.mockRejectedValueOnce(new Error('broken'))
    await flow.loadFromEditPath('/tmp/x.cursorpack')
    expect(importMessage.value).toBe('creator.errEditLoadFailed')
    expect(stage.value).toBe('editing')
  })
})
